use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct KdeBackend {
    #[allow(dead_code)]
    config: Config,
}

impl KdeBackend {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl CompositorBackend for KdeBackend {
    fn name(&self) -> &'static str {
        "KDE Plasma"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: std::env::var("WAYLAND_DISPLAY")
                .map(|_| "Wayland".to_string())
                .unwrap_or_else(|_| "X11".to_string()),
            compositor: "KDE Plasma / KWin".to_string(),
            layout_tool: "qdbus org.kde.keyboard /Layouts".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        // Query active window via xdotool or kwin
        if let Ok(output) = Command::new("xdotool").args(["getactivewindow"]).output() {
            if output.status.success() {
                let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let mut app_id = "unknown".to_string();
                if let Ok(prop_out) = Command::new("xprop").args(["-id", &id, "WM_CLASS"]).output() {
                    let prop_str = String::from_utf8_lossy(&prop_out.stdout);
                    if let Some(cls) = prop_str.split('"').nth(3) {
                        app_id = cls.to_string();
                    }
                }
                return Some(WindowInfo {
                    id,
                    app_id,
                    title: String::new(),
                });
            }
        }
        None
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let output = Command::new("qdbus")
            .args(["org.kde.keyboard", "/Layouts", "org.kde.KeyboardLayouts.getCurrentLayout"])
            .output()
            .ok()?;

        if output.status.success() {
            let res = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !res.is_empty() {
                return Some(res);
            }
        }

        // Fallback to getLayouts
        let out2 = Command::new("qdbus")
            .args(["org.kde.keyboard", "/Layouts", "org.kde.KeyboardLayouts.getLayouts"])
            .output()
            .ok()?;
        if out2.status.success() {
            return String::from_utf8_lossy(&out2.stdout)
                .lines()
                .next()
                .map(|s| s.trim().to_string());
        }

        None
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        Command::new("qdbus")
            .args([
                "org.kde.keyboard",
                "/Layouts",
                "org.kde.KeyboardLayouts.setLayout",
                layout,
            ])
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

        while stop_rx.try_recv().is_err() {
            if let Some(win) = self.get_current_window() {
                if win.id != prev_id {
                    prev_id = win.id.clone();
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_focus_change(win, self);
                    }
                }
            }

            if let Some(layout) = self.get_current_layout() {
                if layout != prev_layout {
                    prev_layout = layout.clone();
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_layout_change(layout);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
