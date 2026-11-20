use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct GnomeBackend {
    #[allow(dead_code)]
    config: Config,
}

impl GnomeBackend {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl CompositorBackend for GnomeBackend {
    fn name(&self) -> &'static str {
        "GNOME Shell"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: std::env::var("WAYLAND_DISPLAY")
                .map(|_| "Wayland".to_string())
                .unwrap_or_else(|_| "X11".to_string()),
            compositor: "GNOME Shell".to_string(),
            layout_tool: "gsettings org.gnome.desktop.input-sources".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        let output = Command::new("gdbus")
            .args([
                "call",
                "--session",
                "--dest",
                "org.gnome.Shell",
                "--object-path",
                "/org/gnome/Shell",
                "--method",
                "org.gnome.Shell.Eval",
                "global.display.focus_window ? (global.display.focus_window.get_id() + ':::' + global.display.focus_window.get_wm_class() + ':::' + global.display.focus_window.get_title()) : ''",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        // Output format: (true, '1234:::firefox-esr:::Title')
        let inner = raw.split('\'').nth(1)?;
        if inner.is_empty() {
            return None;
        }

        let parts: Vec<&str> = inner.split(":::").collect();
        let id = parts.first().unwrap_or(&"0").to_string();
        let app_id = parts.get(1).unwrap_or(&"unknown").to_string();
        let title = parts.get(2).unwrap_or(&"").to_string();

        Some(WindowInfo { id, app_id, title })
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let output = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.input-sources", "current"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        // raw is "uint32 0\n"
        let parts: Vec<&str> = raw.split_whitespace().collect();
        parts.last().map(|s| s.to_string())
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        let target = if layout.chars().all(|c| c.is_ascii_digit()) {
            layout
        } else if layout.eq_ignore_ascii_case("us") || layout.eq_ignore_ascii_case("en") {
            "0"
        } else {
            "1"
        };

        Command::new("gsettings")
            .args(["set", "org.gnome.desktop.input-sources", "current", target])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn get_available_layouts(&mut self) -> Vec<String> {
        vec!["0".to_string(), "1".to_string()]
    }

    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        // Monitor gsettings in background thread for layout changes
        let engine_clone = Arc::clone(&engine);
        std::thread::spawn(move || {
            if let Ok(mut child) = Command::new("gsettings")
                .args(["monitor", "org.gnome.desktop.input-sources", "current"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(idx) = parts.last() {
                            if let Ok(mut eng) = engine_clone.lock() {
                                eng.on_layout_change(idx.to_string());
                            }
                        }
                    }
                }
            }
        });

        // Window polling loop (adaptive 40ms)
        let mut prev_id = String::new();
        while stop_rx.try_recv().is_err() {
            if let Some(win) = self.get_current_window() {
                if win.id != prev_id {
                    prev_id = win.id.clone();
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_focus_change(win, self);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    }
}
