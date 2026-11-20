pub mod generic;
pub mod gnome;
pub mod hyprland;
pub mod kde;
pub mod niri;
pub mod sway;
pub mod x11;

use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowInfo {
    pub id: String,
    pub app_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub display_server: String,
    pub compositor: String,
    pub layout_tool: String,
}

#[allow(dead_code)]
pub trait CompositorBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn environment_info(&self) -> EnvironmentInfo;
    fn get_current_window(&mut self) -> Option<WindowInfo>;
    fn get_current_layout(&mut self) -> Option<String>;
    fn set_layout(&mut self, layout: &str) -> bool;
    fn get_available_layouts(&mut self) -> Vec<String>;
    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    );
}

pub fn detect_backend(config: &Config) -> Box<dyn CompositorBackend> {
    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let display = std::env::var("DISPLAY").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    if !wayland.is_empty() {
        if std::env::var("NIRI_SOCKET").is_ok() || is_command_available("niri") && is_process_running("niri") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Niri Compositor");
            }
            return Box::new(niri::NiriBackend::new(config.clone()));
        }

        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() || is_process_running("Hyprland") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Hyprland Compositor");
            }
            return Box::new(hyprland::HyprlandBackend::new(config.clone()));
        }

        if std::env::var("SWAYSOCK").is_ok() || is_process_running("sway") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Sway Compositor");
            }
            return Box::new(sway::SwayBackend::new(config.clone()));
        }

        if desktop.contains("gnome") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / GNOME Shell");
            }
            return Box::new(gnome::GnomeBackend::new(config.clone()));
        }

        if desktop.contains("kde") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / KDE Plasma");
            }
            return Box::new(kde::KdeBackend::new(config.clone()));
        }

        return Box::new(generic::GenericBackend::new(config.clone(), "Wayland"));
    }

    if !display.is_empty() {
        if desktop.contains("gnome") {
            return Box::new(gnome::GnomeBackend::new(config.clone()));
        }
        if desktop.contains("kde") {
            return Box::new(kde::KdeBackend::new(config.clone()));
        }
        if std::env::var("I3SOCK").is_ok() || is_process_running("i3") {
            return Box::new(sway::SwayBackend::new(config.clone()));
        }

        return Box::new(x11::X11Backend::new(config.clone()));
    }

    Box::new(generic::GenericBackend::new(config.clone(), "Generic"))
}

pub fn is_process_running(proc_name: &str) -> bool {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let path = entry.path().join("comm");
            if let Ok(comm) = std::fs::read_to_string(path) {
                if comm.trim().eq_ignore_ascii_case(proc_name) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn is_command_available(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let full = std::path::Path::new(dir).join(cmd);
            if full.is_file() && is_executable(&full) {
                return true;
            }
        }
    }
    false
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            return meta.permissions().mode() & 0o111 != 0;
        }
    }
    false
}
