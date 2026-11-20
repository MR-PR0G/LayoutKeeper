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

pub fn auto_discover_runtime_env() {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let uid = unsafe { libc_getuid() };
        format!("/run/user/{uid}")
    });

    let runtime_path = std::path::Path::new(&runtime_dir);
    if runtime_path.exists() {
        // 1. Auto-discover WAYLAND_DISPLAY if not set
        if std::env::var("WAYLAND_DISPLAY").map(|s| s.is_empty()).unwrap_or(true) {
            if let Ok(entries) = std::fs::read_dir(runtime_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("wayland-") && !name.ends_with(".lock") {
                        std::env::set_var("WAYLAND_DISPLAY", &name);
                        break;
                    }
                }
            }
        }

        // 2. Auto-discover NIRI_SOCKET if not set
        if std::env::var("NIRI_SOCKET").map(|s| s.is_empty()).unwrap_or(true) {
            if let Ok(entries) = std::fs::read_dir(runtime_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("niri.") && name.ends_with(".sock") {
                        std::env::set_var("NIRI_SOCKET", entry.path().to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }

        // 3. Auto-discover HYPRLAND_INSTANCE_SIGNATURE if not set
        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").map(|s| s.is_empty()).unwrap_or(true) {
            let hypr_dir = runtime_path.join("hypr");
            if hypr_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&hypr_dir) {
                    for entry in entries.flatten() {
                        if entry.path().join(".socket2.sock").exists() {
                            let sig = entry.file_name().to_string_lossy().to_string();
                            std::env::set_var("HYPRLAND_INSTANCE_SIGNATURE", &sig);
                            break;
                        }
                    }
                }
            }
        }

        // 4. Auto-discover SWAYSOCK if not set
        if std::env::var("SWAYSOCK").map(|s| s.is_empty()).unwrap_or(true) {
            if let Ok(entries) = std::fs::read_dir(runtime_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("sway-ipc.") && name.ends_with(".sock") {
                        std::env::set_var("SWAYSOCK", entry.path().to_string_lossy().to_string());
                        break;
                    }
                }
            }
        }
    }

    // 5. Auto-discover DISPLAY if not set
    if std::env::var("DISPLAY").map(|s| s.is_empty()).unwrap_or(true) {
        if std::path::Path::new("/tmp/.X11-unix/X0").exists() {
            std::env::set_var("DISPLAY", ":0");
        }
    }
}

unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

pub fn try_detect_backend(config: &Config) -> Option<Box<dyn CompositorBackend>> {
    auto_discover_runtime_env();

    let wayland = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    let display = std::env::var("DISPLAY").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    if !wayland.is_empty() || is_process_running("niri") || is_process_running("Hyprland") || is_process_running("sway") {
        if std::env::var("NIRI_SOCKET").is_ok() || is_command_available("niri") && is_process_running("niri") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Niri Compositor");
            }
            return Some(Box::new(niri::NiriBackend::new(config.clone())));
        }

        if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() || is_process_running("Hyprland") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Hyprland Compositor");
            }
            return Some(Box::new(hyprland::HyprlandBackend::new(config.clone())));
        }

        if std::env::var("SWAYSOCK").is_ok() || is_process_running("sway") {
            if config.general.debug {
                eprintln!("[LayoutKeeper] Detected Wayland / Sway Compositor");
            }
            return Some(Box::new(sway::SwayBackend::new(config.clone())));
        }

        if desktop.contains("gnome") {
            return Some(Box::new(gnome::GnomeBackend::new(config.clone())));
        }

        if desktop.contains("kde") {
            return Some(Box::new(kde::KdeBackend::new(config.clone())));
        }

        return Some(Box::new(generic::GenericBackend::new(config.clone(), "Wayland")));
    }

    if !display.is_empty() {
        if desktop.contains("gnome") {
            return Some(Box::new(gnome::GnomeBackend::new(config.clone())));
        }
        if desktop.contains("kde") {
            return Some(Box::new(kde::KdeBackend::new(config.clone())));
        }
        if std::env::var("I3SOCK").is_ok() || is_process_running("i3") {
            return Some(Box::new(sway::SwayBackend::new(config.clone())));
        }

        return Some(Box::new(x11::X11Backend::new(config.clone())));
    }

    None
}

pub fn detect_backend(config: &Config) -> Box<dyn CompositorBackend> {
    if let Some(b) = try_detect_backend(config) {
        b
    } else {
        Box::new(generic::GenericBackend::new(config.clone(), "Generic"))
    }
}

pub fn wait_for_backend(config: &Config) -> Box<dyn CompositorBackend> {
    let mut logged = false;
    loop {
        if let Some(backend) = try_detect_backend(config) {
            return backend;
        }

        if !logged {
            eprintln!("[LayoutKeeper] No active graphical session detected yet. Waiting for display server...");
            logged = true;
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
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
