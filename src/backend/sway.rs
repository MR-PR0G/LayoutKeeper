use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct SwayBackend {
    #[allow(dead_code)]
    config: Config,
}

impl SwayBackend {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl CompositorBackend for SwayBackend {
    fn name(&self) -> &'static str {
        "Sway/i3"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: "Wayland".to_string(),
            compositor: "Sway/i3".to_string(),
            layout_tool: "swaymsg input type:keyboard".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        let output = Command::new("swaymsg")
            .args(["-t", "get_tree"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        find_focused_node(&val)
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let output = Command::new("swaymsg")
            .args(["-t", "get_inputs"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        if let Some(arr) = val.as_array() {
            for item in arr {
                if item.get("type").and_then(|v| v.as_str()) == Some("keyboard") {
                    if let Some(idx) = item.get("xkb_active_layout_index").and_then(|v| v.as_u64()) {
                        return Some(idx.to_string());
                    }
                    if let Some(name) = item.get("xkb_active_layout_name").and_then(|v| v.as_str()) {
                        return Some(name.to_string());
                    }
                }
            }
        }
        None
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        let status = if layout.chars().all(|c| c.is_ascii_digit()) {
            Command::new("swaymsg")
                .args(["input", "type:keyboard", "xkb_switch_layout", layout])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        } else {
            Command::new("swaymsg")
                .args(["input", "type:keyboard", "xkb_layout", layout])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };

        status.map(|s| s.success()).unwrap_or(false)
    }

    fn get_available_layouts(&mut self) -> Vec<String> {
        vec!["0".to_string(), "1".to_string()]
    }

    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            let mut child: Child = match Command::new("swaymsg")
                .args(["-t", "subscribe", "-m", r#"["window", "input"]"#])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[LayoutKeeper] Sway subscribe error: {e}. Retrying...");
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = child.kill();
                    continue;
                }
            };

            let reader = BufReader::new(stdout);
            for line_res in reader.lines() {
                if stop_rx.try_recv().is_ok() {
                    let _ = child.kill();
                    return;
                }

                let line = match line_res {
                    Ok(l) => l,
                    Err(_) => break,
                };

                let val: Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let change = val.get("change").and_then(|v| v.as_str()).unwrap_or("");
                if change == "focus" {
                    if let Some(container) = val.get("container") {
                        let id = container.get("id").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default();
                        let app_id = container.get("app_id")
                            .and_then(|v| v.as_str())
                            .or_else(|| container.get("window_properties").and_then(|w| w.get("class")).and_then(|v| v.as_str()))
                            .unwrap_or("unknown")
                            .to_string();
                        let title = container.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();

                        let win = WindowInfo { id, app_id, title };
                        if let Ok(mut eng) = engine.lock() {
                            eng.on_focus_change(win, self);
                        }
                    }
                } else if change == "xkb_layout" {
                    if let Some(input) = val.get("input") {
                        if let Some(idx) = input.get("xkb_active_layout_index").and_then(|v| v.as_u64()) {
                            if let Ok(mut eng) = engine.lock() {
                                eng.on_layout_change(idx.to_string());
                            }
                        }
                    }
                }
            }

            let _ = child.kill();
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

fn find_focused_node(node: &Value) -> Option<WindowInfo> {
    if node.get("focused").and_then(|v| v.as_bool()).unwrap_or(false) {
        let id = node.get("id").and_then(|v| v.as_u64()).map(|v| v.to_string()).unwrap_or_default();
        let app_id = node
            .get("app_id")
            .and_then(|v| v.as_str())
            .or_else(|| node.get("window_properties").and_then(|w| w.get("class")).and_then(|v| v.as_str()))
            .unwrap_or("unknown")
            .to_string();
        let title = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        return Some(WindowInfo { id, app_id, title });
    }

    if let Some(nodes) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in nodes {
            if let Some(w) = find_focused_node(child) {
                return Some(w);
            }
        }
    }

    if let Some(floating) = node.get("floating_nodes").and_then(|v| v.as_array()) {
        for child in floating {
            if let Some(w) = find_focused_node(child) {
                return Some(w);
            }
        }
    }

    None
}
