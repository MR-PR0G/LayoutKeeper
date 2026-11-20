use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct HyprlandBackend {
    #[allow(dead_code)]
    config: Config,
    socket_path: Option<PathBuf>,
}

impl HyprlandBackend {
    pub fn new(config: Config) -> Self {
        let socket_path = Self::find_socket2();
        Self {
            config,
            socket_path,
        }
    }

    fn find_socket2() -> Option<PathBuf> {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = PathBuf::from(runtime)
            .join("hypr")
            .join(sig)
            .join(".socket2.sock");
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
}

impl CompositorBackend for HyprlandBackend {
    fn name(&self) -> &'static str {
        "Hyprland"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: "Wayland".to_string(),
            compositor: "Hyprland".to_string(),
            layout_tool: "hyprctl switchxkblayout".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        let output = Command::new("hyprctl")
            .args(["activewindow", "-j"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        if val.is_null() {
            return None;
        }

        let address = val
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let class = val
            .get("class")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let title = val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(WindowInfo {
            id: address,
            app_id: class,
            title,
        })
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let output = Command::new("hyprctl")
            .args(["devices", "-j"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        if let Some(keyboards) = val.get("keyboards").and_then(|v| v.as_array()) {
            for k in keyboards {
                if k.get("main").and_then(|v| v.as_bool()).unwrap_or(false) {
                    if let Some(layout) = k.get("active_keymap").and_then(|v| v.as_str()) {
                        return Some(layout.to_string());
                    }
                }
            }
            if let Some(first) = keyboards.first() {
                if let Some(layout) = first.get("active_keymap").and_then(|v| v.as_str()) {
                    return Some(layout.to_string());
                }
            }
        }
        None
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        // Query keyboard device names
        let output = match Command::new("hyprctl")
            .args(["devices", "-j"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return false,
        };

        if let Ok(val) = serde_json::from_slice::<Value>(&output.stdout) {
            if let Some(keyboards) = val.get("keyboards").and_then(|v| v.as_array()) {
                let mut success = true;
                for k in keyboards {
                    if let Some(name) = k.get("name").and_then(|v| v.as_str()) {
                        let res = Command::new("hyprctl")
                            .args(["switchxkblayout", name, layout])
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                        if let Ok(s) = res {
                            if !s.success() {
                                success = false;
                            }
                        }
                    }
                }
                return success;
            }
        }
        false
    }

    fn get_available_layouts(&mut self) -> Vec<String> {
        vec!["0".to_string(), "1".to_string()]
    }

    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        let sock_path = match &self.socket_path {
            Some(p) => p.clone(),
            None => {
                if let Some(p) = Self::find_socket2() {
                    p
                } else {
                    eprintln!("[LayoutKeeper] Hyprland socket2.sock not found.");
                    return;
                }
            }
        };

        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            let stream = match UnixStream::connect(&sock_path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[LayoutKeeper] Failed to connect to Hyprland socket2: {e}. Retrying...");
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            let reader = BufReader::new(stream);
            for line_res in reader.lines() {
                if stop_rx.try_recv().is_ok() {
                    return;
                }

                let line = match line_res {
                    Ok(l) => l,
                    Err(_) => break,
                };

                if let Some((evt, data)) = line.split_once(">>") {
                    match evt {
                        "activewindow" => {
                            let (class, title) = data.split_once(',').unwrap_or((data, ""));
                            let win = WindowInfo {
                                id: class.to_string(),
                                app_id: class.to_string(),
                                title: title.to_string(),
                            };
                            if let Ok(mut eng) = engine.lock() {
                                eng.on_focus_change(win, self);
                            }
                        }
                        "activewindowv2" => {
                            if let Some(mut win) = self.get_current_window() {
                                win.id = data.to_string();
                                if let Ok(mut eng) = engine.lock() {
                                    eng.on_focus_change(win, self);
                                }
                            }
                        }
                        "activelayout" => {
                            let (_kbd, layout) = data.split_once(',').unwrap_or(("", data));
                            if let Ok(mut eng) = engine.lock() {
                                eng.on_layout_change(layout.to_string());
                            }
                        }
                        "closewindow" => {
                            if let Ok(mut eng) = engine.lock() {
                                eng.on_window_close(data);
                            }
                        }
                        _ => {}
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    }
}
