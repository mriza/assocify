use ini::Ini;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub desktop_file_id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub mimetypes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssocSource {
    User,
    System,
    AppDefault,
    None,
}

pub struct AppStats {
    pub total: usize,
    pub active: usize,
    pub handled_elsewhere: Vec<(String, String)>, // (mime, current_app_name)
}

pub struct Backend {
    pub apps: HashMap<String, AppInfo>, // desktop_file_id -> AppInfo
    pub system_associations: HashMap<String, String>, // mime -> desktop_file_id
    pub user_associations: HashMap<String, String>, // mime -> desktop_file_id
    pub ext_to_mime: HashMap<String, String>, // e.g. "md" -> "text/markdown"
    pub mime_to_exts: HashMap<String, Vec<String>>, // reverse lookup for UI
    pub fallback_associations: HashMap<String, String>, // mime -> first desktop_file_id that supports it
}

impl Backend {
    pub fn new() -> Self {
        let mut backend = Backend {
            apps: HashMap::new(),
            system_associations: HashMap::new(),
            user_associations: HashMap::new(),
            ext_to_mime: HashMap::new(),
            mime_to_exts: HashMap::new(),
            fallback_associations: HashMap::new(),
        };
        backend.reload();
        backend
    }

    pub fn reload(&mut self) {
        self.apps = Self::load_apps();
        self.system_associations = Self::load_system_associations();
        self.user_associations = Self::load_user_associations();
        
        let ext_map = Self::load_mime_globs();
        let mut mime_map: HashMap<String, Vec<String>> = HashMap::new();
        for (ext, mime) in &ext_map {
            mime_map.entry(mime.clone()).or_default().push(ext.clone());
        }
        self.ext_to_mime = ext_map;
        self.mime_to_exts = mime_map;

        // Build fallback associations from apps
        let mut fallbacks = HashMap::new();
        for (desktop_id, app) in &self.apps {
            for mime in &app.mimetypes {
                if !fallbacks.contains_key(mime) {
                    fallbacks.insert(mime.clone(), desktop_id.clone());
                }
            }
        }
        self.fallback_associations = fallbacks;
    }

    fn load_mime_globs() -> HashMap<String, String> {
        let mut ext_map = HashMap::new();
        let paths = vec![
            PathBuf::from("/usr/share/mime/globs2"),
            xdg::BaseDirectories::with_prefix("").unwrap().get_data_home().join("mime/globs2"),
        ];

        for path in paths {
            if let Ok(content) = fs::read_to_string(&path) {
                for line in content.lines() {
                    if line.starts_with('#') || line.is_empty() {
                        continue;
                    }
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 3 {
                        let mime_type = parts[1].to_string();
                        let glob = parts[2];
                        if glob.starts_with("*.") && !glob.contains('[') && !glob.contains('?') {
                            let ext = glob[2..].to_string();
                            if !ext_map.contains_key(&ext) {
                                ext_map.insert(ext, mime_type);
                            }
                        }
                    }
                }
            }
        }
        ext_map
    }

    fn load_apps() -> HashMap<String, AppInfo> {
        let mut apps = HashMap::new();
        let dirs = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            xdg::BaseDirectories::with_prefix("")
                .unwrap()
                .get_data_home()
                .join("applications"),
        ];

        for dir in dirs {
            if !dir.exists() {
                continue;
            }
            for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() && entry.path().extension().map_or(false, |ext| ext == "desktop") {
                    if let Some(app_info) = Self::parse_desktop_file(entry.path(), &dir) {
                        apps.insert(app_info.desktop_file_id.clone(), app_info);
                    }
                }
            }
        }
        apps
    }

    fn parse_desktop_file(path: &Path, base_dir: &Path) -> Option<AppInfo> {
        let ini = Ini::load_from_file(path).ok()?;
        let section = ini.section(Some("Desktop Entry"))?;
        
        if section.get("NoDisplay") == Some("true") || section.get("Hidden") == Some("true") {
            return None;
        }

        let name = section.get("Name")?.to_string();
        let exec = section.get("Exec")?.to_string();
        let icon = section.get("Icon").map(|s| s.to_string());
        
        let mimetypes_str = section.get("MimeType").unwrap_or("");
        let mimetypes: Vec<String> = mimetypes_str
            .split(';')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Only include apps that actually declare mimetypes, since our app-centric management relies on it
        if mimetypes.is_empty() {
            return None;
        }

        let rel_path = path.strip_prefix(base_dir).unwrap_or(path);
        let desktop_file_id = rel_path.to_string_lossy().replace("/", "-");

        Some(AppInfo {
            desktop_file_id,
            name,
            exec,
            icon,
            mimetypes,
        })
    }

    fn load_system_associations() -> HashMap<String, String> {
        let mut assocs = HashMap::new();
        let dirs = vec![
            PathBuf::from("/usr/share/applications/mimeapps.list"),
            PathBuf::from("/usr/local/share/applications/mimeapps.list"),
            PathBuf::from("/etc/xdg/mimeapps.list"),
        ];

        for path in dirs {
            if let Ok(ini) = Ini::load_from_file(path) {
                if let Some(section) = ini.section(Some("Default Applications")) {
                    for (k, v) in section.iter() {
                        if let Some(first_app) = v.split(';').next() {
                            assocs.insert(k.to_string(), first_app.to_string());
                        }
                    }
                }
            }
        }
        assocs
    }

    fn load_user_associations() -> HashMap<String, String> {
        let mut assocs = HashMap::new();
        if let Ok(xdg_dirs) = xdg::BaseDirectories::with_prefix("") {
            if let Some(config_home) = xdg_dirs.find_config_file("mimeapps.list") {
                if let Ok(ini) = Ini::load_from_file(config_home) {
                    if let Some(section) = ini.section(Some("Default Applications")) {
                        for (k, v) in section.iter() {
                            if let Some(first_app) = v.split(';').next() {
                                assocs.insert(k.to_string(), first_app.to_string());
                            }
                        }
                    }
                }
            }
        }
        assocs
    }

    pub fn get_effective_app(&self, mime: &str) -> (Option<String>, AssocSource) {
        if let Some(app) = self.user_associations.get(mime) {
            return (Some(app.clone()), AssocSource::User);
        }
        if let Some(app) = self.system_associations.get(mime) {
            return (Some(app.clone()), AssocSource::System);
        }
        if let Some(app) = self.fallback_associations.get(mime) {
            return (Some(app.clone()), AssocSource::AppDefault);
        }
        (None, AssocSource::None)
    }

    pub fn get_app_stats(&self, desktop_file_id: &str) -> AppStats {
        let app = self.apps.get(desktop_file_id).unwrap();
        let mut active = 0;
        let mut handled_elsewhere = Vec::new();
        
        for mime in &app.mimetypes {
            let (eff_app_id, _) = self.get_effective_app(mime);
            if eff_app_id.as_deref() == Some(desktop_file_id) {
                active += 1;
            } else {
                let eff_name = eff_app_id
                    .and_then(|id| self.apps.get(&id).map(|a| a.name.clone()))
                    .unwrap_or_else(|| "None".to_string());
                handled_elsewhere.push((mime.clone(), eff_name));
            }
        }
        
        AppStats {
            total: app.mimetypes.len(),
            active,
            handled_elsewhere,
        }
    }

    pub fn set_user_association(&mut self, mime: &str, desktop_file_id: &str) -> Result<(), String> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("").map_err(|e| e.to_string())?;
        let config_home = xdg_dirs.place_config_file("mimeapps.list").map_err(|e| e.to_string())?;

        let mut ini = Ini::load_from_file(&config_home).unwrap_or_else(|_| Ini::new());
        ini.with_section(Some("Default Applications"))
            .set(mime, format!("{};", desktop_file_id));

        ini.write_to_file(&config_home).map_err(|e| e.to_string())?;
        self.user_associations.insert(mime.to_string(), desktop_file_id.to_string());
        Ok(())
    }

    pub fn reset_user_association(&mut self, mime: &str) -> Result<(), String> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix("").map_err(|e| e.to_string())?;
        let config_home = xdg_dirs.place_config_file("mimeapps.list").map_err(|e| e.to_string())?;

        let mut ini = Ini::load_from_file(&config_home).unwrap_or_else(|_| Ini::new());
        
        if let Some(section) = ini.section_mut(Some("Default Applications")) {
            section.remove(mime);
        }

        ini.write_to_file(&config_home).map_err(|e| e.to_string())?;
        self.user_associations.remove(mime);
        Ok(())
    }

    pub fn set_system_association(&mut self, mime: &str, desktop_file_id: &str) -> Result<(), String> {
        let system_file = PathBuf::from("/etc/xdg/mimeapps.list");
        let mut ini = Ini::load_from_file(&system_file).unwrap_or_else(|_| Ini::new());
        
        ini.with_section(Some("Default Applications"))
            .set(mime, format!("{};", desktop_file_id));

        let temp_file = std::env::temp_dir().join("linux_file_assoc_manager_mimeapps.list");
        ini.write_to_file(&temp_file).map_err(|e| e.to_string())?;

        let status = std::process::Command::new("pkexec")
            .arg("cp")
            .arg(&temp_file)
            .arg(&system_file)
            .status()
            .map_err(|e| format!("Failed to execute pkexec: {}", e))?;

        std::process::Command::new("pkexec")
            .arg("chmod")
            .arg("644")
            .arg(&system_file)
            .status()
            .ok();

        std::fs::remove_file(&temp_file).ok();

        if !status.success() {
            return Err("Action was cancelled or authentication failed.".to_string());
        }

        self.system_associations.insert(mime.to_string(), desktop_file_id.to_string());
        Ok(())
    }

    pub fn set_app_associations_selectively(&mut self, desktop_file_id: &str, selections: &[(String, bool)], system_wide: bool) -> Result<(), String> {
        if !system_wide {
            let xdg_dirs = xdg::BaseDirectories::with_prefix("").map_err(|e| e.to_string())?;
            let config_home = xdg_dirs.place_config_file("mimeapps.list").map_err(|e| e.to_string())?;

            let mut ini = Ini::load_from_file(&config_home).unwrap_or_else(|_| Ini::new());
            
            for (mime, should_set) in selections {
                if *should_set {
                    ini.with_section(Some("Default Applications"))
                        .set(mime.clone(), format!("{};", desktop_file_id));
                    self.user_associations.insert(mime.clone(), desktop_file_id.to_string());
                } else {
                    if self.user_associations.get(mime).map(|s| s.as_str()) == Some(desktop_file_id) {
                        if let Some(section) = ini.section_mut(Some("Default Applications")) {
                            section.remove(mime);
                        }
                        self.user_associations.remove(mime);
                    }
                }
            }

            ini.write_to_file(&config_home).map_err(|e| e.to_string())?;
        } else {
            let system_file = PathBuf::from("/etc/xdg/mimeapps.list");
            let mut ini = Ini::load_from_file(&system_file).unwrap_or_else(|_| Ini::new());
            
            for (mime, should_set) in selections {
                if *should_set {
                    ini.with_section(Some("Default Applications"))
                        .set(mime.clone(), format!("{};", desktop_file_id));
                } else {
                    if self.system_associations.get(mime).map(|s| s.as_str()) == Some(desktop_file_id) {
                        if let Some(section) = ini.section_mut(Some("Default Applications")) {
                            section.remove(mime);
                        }
                    }
                }
            }

            let temp_file = std::env::temp_dir().join("linux_file_assoc_manager_mimeapps.list");
            ini.write_to_file(&temp_file).map_err(|e| e.to_string())?;

            let status = std::process::Command::new("pkexec")
                .arg("cp")
                .arg(&temp_file)
                .arg(&system_file)
                .status()
                .map_err(|e| format!("Failed to execute pkexec: {}", e))?;

            std::process::Command::new("pkexec")
                .arg("chmod")
                .arg("644")
                .arg(&system_file)
                .status()
                .ok();

            std::fs::remove_file(&temp_file).ok();

            if !status.success() {
                return Err("Action was cancelled or authentication failed.".to_string());
            }

            for (mime, should_set) in selections {
                if *should_set {
                    self.system_associations.insert(mime.clone(), desktop_file_id.to_string());
                } else {
                    if self.system_associations.get(mime).map(|s| s.as_str()) == Some(desktop_file_id) {
                        self.system_associations.remove(mime);
                    }
                }
            }
        }
        
        Ok(())
    }
}
