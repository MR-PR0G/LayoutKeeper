use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct X11Backend {
    #[allow(dead_code)]
    config: Config,
}

impl X11Backend {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl CompositorBackend for X11Backend {
    fn name(&self) -> &'static str {
        "X11"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: "X11".to_string(),
            compositor: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "X11 WM".to_string()),
            layout_tool: "setxkbmap".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        let output = Command::new("xdotool")
            .args(["getwindowfocus"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if id.is_empty() {
            return None;
        }

        let mut app_id = "unknown".to_string();
        if let Ok(prop_out) = Command::new("xprop").args(["-id", &id, "WM_CLASS"]).output() {
            let prop_str = String::from_utf8_lossy(&prop_out.stdout);
            if let Some(cls) = prop_str.split('"').nth(3) {
                app_id = cls.to_string();
            } else if let Some(cls) = prop_str.split('"').nth(1) {
                app_id = cls.to_string();
            }
        }

        let mut title = String::new();
        if let Ok(title_out) = Command::new("xdotool").args(["getwindowname", &id]).output() {
            title = String::from_utf8_lossy(&title_out.stdout).trim().to_string();
        }

        Some(WindowInfo { id, app_id, title })
    }

    fn get_current_layout(&mut self) -> Option<String> {
        // Try xkb-switch first if available
        if let Ok(output) = Command::new("xkb-switch").args(["-p"]).output() {
            if output.status.success() {
                let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !res.is_empty() {
                    return Some(res);
                }
            }
        }

        // Fallback to setxkbmap -query
        let output = Command::new("setxkbmap")
            .args(["-query"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        for line in raw.lines() {
            if line.starts_with("layout:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(layout) = parts.get(1) {
                    return Some(layout.to_string());
                }
            }
        }

        None
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        // Try xkb-switch first
        if let Ok(s) = Command::new("xkb-switch")
            .args(["-s", layout])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if s.success() {
                return true;
            }
        }

        // Fallback to setxkbmap
        Command::new("setxkbmap")
            .arg(layout)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn get_available_layouts(&mut self) -> Vec<String> {
        vec!["us".to_string()]
    }

    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        let mut prev_id = String::new();
        let mut prev_layout = String::new();
        let mut idle_count = 0;

        while stop_rx.try_recv().is_err() {
            let mut changed = false;

            if let Some(win) = self.get_current_window() {
                if win.id != prev_id {
                    prev_id = win.id.clone();
                    changed = true;
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_focus_change(win, self);
                    }
                }
            }

            if let Some(layout) = self.get_current_layout() {
                if layout != prev_layout {
                    prev_layout = layout.clone();
                    changed = true;
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_layout_change(layout);
                    }
                }
            }

            if changed {
                idle_count = 0;
                std::thread::sleep(Duration::from_millis(30));
            } else {
                idle_count += 1;
                if idle_count > 30 {
                    // Back off slightly when idle to save CPU/battery
                    std::thread::sleep(Duration::from_millis(100));
                } else {
                    std::thread::sleep(Duration::from_millis(35));
                }
            }
        }
    }
}
