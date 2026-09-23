# Assocify

**Assocify** is a modern, fast, and surgical Linux File Association Manager written in Rust using `egui`. It gives you ultimate control over which applications open which file types on your Linux desktop.

## Features

- **Application-Centric Management**: View all your installed applications and see exactly which file types they support.
- **Surgical Precision Checklist**: Click "Manage Associations..." to see a checklist of every MIME type an app supports. Easily take back control of hijacked extensions or relinquish them with a single click.
- **System-Wide Polkit Integration**: Apply your changes to just your user account, or seamlessly apply them system-wide across all users using `pkexec`.
- **Fast and Modern UI**: Built with Rust and `egui`, Assocify is lightweight, incredibly fast, and features a clean, responsive dark-mode interface.
- **Detailed Extension View**: Search for a specific extension like `.pdf` or `.xml` to find out exactly which application is currently handling it, and whether it's enforced by the system, the user, or simply a fallback.

## Installation

You can download the pre-compiled binary from the [Releases](https://github.com/mriza/assocify/releases) page.

Alternatively, you can build from source:

```bash
git clone git@github.com:mriza/assocify.git
cd assocify
cargo build --release
```
The binary will be located in `target/release/assocify`.

## Usage

Run the binary directly:
```bash
./assocify
```
(Or just `cargo run --release` during development).

## Requirements

- Linux Desktop Environment (X11 or Wayland)
- `xdg-utils` (specifically `mimeapps.list` specification support)
- `pkexec` (Polkit) if you want to make system-wide changes.

## License
MIT License
