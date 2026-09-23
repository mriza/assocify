mod backend;

use backend::{AssocSource, Backend};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use itertools::Itertools;

#[derive(PartialEq)]
enum MainTab {
    Applications,
    Detailed,
}

struct MyApp {
    backend: Backend,
    search_query_apps: String,
    search_query_detailed: String,
    current_tab: MainTab,
    // Context for editing a single mime
    editing_mime: Option<String>,
    // Context for editing all mimes for an app
    manage_app: Option<String>,
    app_selections: Vec<(String, bool)>, // (mime, should_be_associated)
    system_wide_edit: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            backend: Backend::new(),
            search_query_apps: String::new(),
            search_query_detailed: String::new(),
            current_tab: MainTab::Applications,
            editing_mime: None,
            manage_app: None,
            app_selections: Vec::new(),
            system_wide_edit: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Custom visual theme override (darker, rounded, modern)
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        visuals.panel_fill = egui::Color32::from_rgb(25, 25, 30);
        ctx.set_visuals(visuals);

        // Increase base font size globally
        let mut style = (*ctx.style()).clone();
        style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 15.0;
        style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = 15.0;
        style.text_styles.get_mut(&egui::TextStyle::Heading).unwrap().size = 22.0;
        style.spacing.item_spacing = egui::vec2(12.0, 12.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        ctx.set_style(style);

        egui::TopBottomPanel::top("top_panel").frame(egui::Frame::default().fill(egui::Color32::from_rgb(30, 30, 36)).inner_margin(15.0)).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("⚙ Assocify");
                ui.add_space(5.0);
                ui.label(egui::RichText::new("— Linux File Association Manager").color(egui::Color32::from_rgb(160, 160, 160)).size(16.0));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Reload System Data").clicked() {
                        self.backend.reload();
                    }
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, MainTab::Applications, "📦 Applications");
                ui.selectable_value(&mut self.current_tab, MainTab::Detailed, "🔍 Detailed Extensions");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                MainTab::Applications => self.ui_applications(ui),
                MainTab::Detailed => self.ui_detailed(ui),
            }
        });

        self.ui_popups(ctx);
    }
}

impl MyApp {
    fn ui_applications(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.heading("Application-Centric Management");
        ui.colored_label(egui::Color32::from_rgb(180, 180, 180), "Make an application the default for EVERYTHING it supports.");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("🔍 Search Applications:");
            ui.text_edit_singleline(&mut self.search_query_apps);
        });
        ui.add_space(15.0);

        let query = self.search_query_apps.to_lowercase();
        let mut apps: Vec<_> = self.backend.apps.values()
            .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
            .collect();
        apps.sort_by_key(|a| &a.name);

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(300.0).at_least(200.0))
            .column(Column::initial(300.0).at_least(150.0))
            .column(Column::remainder())
            .header(30.0, |mut header| {
                header.col(|ui| { ui.strong("Application"); });
                header.col(|ui| { ui.strong("Association Status"); });
                header.col(|ui| { ui.strong("Action"); });
            })
            .body(|mut body| {
                for app in apps {
                    let stats = self.backend.get_app_stats(&app.desktop_file_id);
                    
                    body.row(45.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&app.name);
                        });
                        row.col(|ui| {
                            if stats.handled_elsewhere.is_empty() {
                                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), format!("All {} active", stats.total));
                            } else {
                                let label = ui.colored_label(
                                    egui::Color32::from_rgb(255, 200, 100), 
                                    format!("{}/{} active ({} assigned elsewhere)", stats.active, stats.total, stats.handled_elsewhere.len())
                                );
                                
                                // Build tooltip
                                let mut tooltip_text = String::from("Currently handled by other applications:\n");
                                for (mime, thief) in stats.handled_elsewhere.iter().take(15) {
                                    tooltip_text.push_str(&format!("• {}: {}\n", mime, thief));
                                }
                                if stats.handled_elsewhere.len() > 15 {
                                    tooltip_text.push_str(&format!("... and {} more", stats.handled_elsewhere.len() - 15));
                                }
                                label.on_hover_text(tooltip_text);
                            }
                        });
                        row.col(|ui| {
                            if ui.button("Manage Associations...").clicked() {
                                self.manage_app = Some(app.desktop_file_id.clone());
                                self.system_wide_edit = false;
                                
                                // Initialize checklist selections
                                let mut selections = Vec::new();
                                for mime in &app.mimetypes {
                                    let (eff_app, _) = self.backend.get_effective_app(mime);
                                    let is_active = eff_app.as_deref() == Some(app.desktop_file_id.as_str());
                                    selections.push((mime.clone(), is_active));
                                }
                                self.app_selections = selections;
                            }
                        });
                    });
                }
            });
    }

    fn ui_detailed(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("🔍 Search Extension/MIME:");
            ui.text_edit_singleline(&mut self.search_query_detailed);
        });
        ui.add_space(10.0);

        let mut reset_action: Option<String> = None;

        let mut extensions: Vec<_> = self.backend.ext_to_mime.keys().cloned().collect();
        extensions.sort();

        let query = self.search_query_detailed.to_lowercase();
        let filtered_exts: Vec<_> = extensions.into_iter().filter(|ext| {
            query.is_empty() 
            || ext.to_lowercase().contains(&query)
            || self.backend.ext_to_mime.get(ext).map_or(false, |m| m.contains(&query))
        }).collect();

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(100.0).at_least(80.0))
            .column(Column::initial(250.0).at_least(150.0))
            .column(Column::initial(250.0).at_least(150.0))
            .column(Column::initial(120.0).at_least(100.0))
            .column(Column::remainder())
            .header(30.0, |mut header| {
                header.col(|ui| { ui.strong("Extension"); });
                header.col(|ui| { ui.strong("MIME Type"); });
                header.col(|ui| { ui.strong("Effective App"); });
                header.col(|ui| { ui.strong("Source"); });
                header.col(|ui| { ui.strong("Actions"); });
            })
            .body(|mut body| {
                for ext in filtered_exts {
                    if let Some(mime) = self.backend.ext_to_mime.get(&ext) {
                        let (app_id, source) = self.backend.get_effective_app(mime);
                        
                        let app_name = app_id.clone()
                            .and_then(|id| self.backend.apps.get(&id))
                            .map(|app| app.name.clone())
                            .unwrap_or_else(|| app_id.unwrap_or_else(|| "None".to_string()));

                        body.row(35.0, |mut row| {
                            row.col(|ui| { ui.label(format!(".{}", ext)); });
                            row.col(|ui| { ui.colored_label(egui::Color32::from_rgb(180, 180, 180), mime); });
                            row.col(|ui| { ui.label(&app_name); });
                            row.col(|ui| {
                                match source {
                                    AssocSource::User => { ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "● User"); }
                                    AssocSource::System => { ui.colored_label(egui::Color32::from_rgb(100, 200, 255), "● System"); }
                                    AssocSource::AppDefault => { ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "● Fallback"); }
                                    AssocSource::None => { ui.colored_label(egui::Color32::GRAY, "● None"); }
                                };
                            });
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("✏ Change").clicked() {
                                        self.editing_mime = Some(mime.clone());
                                        self.system_wide_edit = false;
                                    }
                                    if source == AssocSource::User {
                                        if ui.button("🔄 Reset").clicked() {
                                            reset_action = Some(mime.clone());
                                        }
                                    }
                                });
                            });
                        });
                    }
                }
            });

        if let Some(mime) = reset_action {
            let _ = self.backend.reset_user_association(&mime);
        }
    }

    fn ui_popups(&mut self, ctx: &egui::Context) {
        let mut close_window = false;
        let mut do_apply_mime: Option<String> = None;
        let mut do_apply_app: bool = false;

        // Popup 1: Change Association for a specific MIME (Detailed tab)
        if let Some(mime) = &self.editing_mime {
            egui::Window::new(format!("Change association for {}", mime))
                .collapsible(false)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.system_wide_edit, "Apply System-wide (Requires Sudo)");
                    if self.system_wide_edit {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "Will prompt for password.");
                    }
                    ui.add_space(10.0);
                    ui.label("Select a new application:");
                    
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let apps = self.backend.apps.values().sorted_by_key(|a| &a.name);
                        for app in apps {
                            if ui.button(&app.name).clicked() {
                                do_apply_mime = Some(app.desktop_file_id.clone());
                                close_window = true;
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        close_window = true;
                    }
                });

            if let Some(app_id) = do_apply_mime {
                let result = if self.system_wide_edit {
                    self.backend.set_system_association(mime, &app_id)
                } else {
                    self.backend.set_user_association(mime, &app_id)
                };
                if let Err(e) = result {
                    log::error!("Failed to set association: {}", e);
                }
            }
        }

        // Popup 2: Manage Associations (Applications tab)
        if let Some(app_id) = &self.manage_app {
            if let Some(app) = self.backend.apps.get(app_id) {
                egui::Window::new(format!("Manage Associations for {}", app.name))
                    .collapsible(false)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.heading(&app.name);
                        ui.label("Select which MIME types this application should handle:");
                        ui.add_space(10.0);
                        
                        egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                            for (mime, is_checked) in self.app_selections.iter_mut() {
                                let mut exts = self.backend.mime_to_exts.get(mime).cloned().unwrap_or_default();
                                exts.sort();
                                let ext_str = if exts.is_empty() {
                                    String::new()
                                } else {
                                    format!(" ({})", exts.iter().map(|e| format!(".{}", e)).join(", "))
                                };
                                
                                // Determine current status
                                let (eff_app, _) = self.backend.get_effective_app(mime);
                                let status_text = if eff_app.as_deref() == Some(app_id.as_str()) {
                                    " (Active)".to_string()
                                } else {
                                    let eff_name = eff_app
                                        .and_then(|id| self.backend.apps.get(&id).map(|a| a.name.clone()))
                                        .unwrap_or_else(|| "None".to_string());
                                    format!(" (Currently: {})", eff_name)
                                };

                                let label = format!("{}{}{}", mime, ext_str, status_text);
                                ui.checkbox(is_checked, label);
                            }
                        });
                        
                        ui.add_space(10.0);
                        ui.checkbox(&mut self.system_wide_edit, "Apply System-wide (Requires Sudo)");
                        if self.system_wide_edit {
                            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "Will prompt for password.");
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                do_apply_app = true;
                                close_window = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close_window = true;
                            }
                        });
                    });
            } else {
                close_window = true;
            }

            if do_apply_app {
                let result = self.backend.set_app_associations_selectively(app_id, &self.app_selections, self.system_wide_edit);
                if let Err(e) = result {
                    log::error!("Failed to set associations selectively: {}", e);
                }
            }
        }

        if close_window {
            self.editing_mime = None;
            self.manage_app = None;
            self.app_selections.clear();
        }
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Assocify",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}
