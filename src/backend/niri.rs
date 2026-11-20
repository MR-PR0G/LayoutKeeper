use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct NiriBackend {
    config: Config,
    cached_layouts: Vec<String>,
    windows_cache: HashMap<u64, WindowInfo>,
}

impl NiriBackend {
    pub fn new(config: Config) -> Self {
        let mut backend = Self {
            config,
            cached_layouts: Vec::new(),
            windows_cache: HashMap::new(),
        };
        backend.refresh_layouts();
        backend.refresh_windows();
        backend
    }

    fn refresh_windows(&mut self) {
        if let Ok(output) = Command::new("niri").args(["msg", "-j", "windows"]).output() {
            if output.status.success() {
                if let Ok(windows) = serde_json::from_slice::<Vec<Value>>(&output.stdout) {
                    for w in windows {
                        if let Some(id) = w.get("id").and_then(|v| v.as_u64()) {
                            let app_id = w
                                .get("app_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let title = w
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            self.windows_cache.insert(
                                id,
                                WindowInfo {
                                    id: id.to_string(),
                                    app_id,
                                    title,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    fn refresh_layouts(&mut self) {
        if let Ok(output) = Command::new("niri")
            .args(["msg", "-j", "keyboard-layouts"])
            .output()
        {
            if output.status.success() {
                if let Ok(val) = serde_json::from_slice::<Value>(&output.stdout) {
                    if let Some(names) = val.get("names").and_then(|v| v.as_array()) {
                        self.cached_layouts = names
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                    }
                }
            }
        }
    }

    fn resolve_layout_target(&mut self, target: &str) -> String {
        // If it's already an index string e.g. "0" or "1"
        if target.parse::<usize>().is_ok() {
            return target.to_string();
        }

        if self.cached_layouts.is_empty() {
            self.refresh_layouts();
        }

        let target_lower = target.to_lowercase();
        for (i, name) in self.cached_layouts.iter().enumerate() {
            let name_lower = name.to_lowercase();
            if name_lower.contains(&target_lower)
                || (target_lower == "us" && name_lower.contains("english"))
                || (target_lower == "en" && name_lower.contains("english"))
                || (target_lower == "fa" && (name_lower.contains("persian") || name_lower.contains("farsi")))
                || (target_lower == "ir" && (name_lower.contains("persian") || name_lower.contains("iran")))
                || (target_lower == "ru" && name_lower.contains("russian"))
                || (target_lower == "ar" && name_lower.contains("arabic"))
            {
                return i.to_string();
            }
        }

        // Default fallback to "0" if unresolvable
        "0".to_string()
    }
}

impl CompositorBackend for NiriBackend {
    fn name(&self) -> &'static str {
        "Niri"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: "Wayland".to_string(),
            compositor: "Niri".to_string(),
            layout_tool: "niri msg action switch-layout".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        let output = Command::new("niri")
            .args(["msg", "-j", "focused-window"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        if val.is_null() {
            return None;
        }

        let id = val.get("id").and_then(|v| v.as_u64())?.to_string();
        let app_id = val
            .get("app_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let title = val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(WindowInfo { id, app_id, title })
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let output = Command::new("niri")
            .args(["msg", "-j", "keyboard-layouts"])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let val: Value = serde_json::from_slice(&output.stdout).ok()?;
        let idx = val.get("current_idx").and_then(|v| v.as_u64())?;
        Some(idx.to_string())
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        let target = self.resolve_layout_target(layout);
        if self.config.general.debug {
            eprintln!("[LayoutKeeper] Niri switching layout to: {} (target: {})", layout, target);
        }

        let status = Command::new("niri")
            .args(["msg", "action", "switch-layout", &target])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(s) => s.success(),
            Err(e) => {
                if self.config.general.debug {
                    eprintln!("[LayoutKeeper] Failed to execute niri switch-layout: {}", e);
                }
                false
            }
        }
    }

    fn get_available_layouts(&mut self) -> Vec<String> {
        self.refresh_layouts();
        self.cached_layouts.clone()
    }

    fn start_event_loop(
        &mut self,
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        if self.config.general.debug {
            eprintln!("[LayoutKeeper] Starting Niri event stream listener...");
        }

        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            // Sync state upon each connection
            self.refresh_layouts();
            self.refresh_windows();
            if let Some(win) = self.get_current_window() {
                if let Some(layout) = self.get_current_layout() {
                    if let Ok(mut eng) = engine.lock() {
                        let w_key = if eng.config.general.remember_per_window_id && !win.id.is_empty() {
                            format!("win:{}", win.id)
                        } else {
                            format!("app:{}", win.app_id.to_lowercase())
                        };
                        eng.active_window = Some(win.clone());
                        eng.current_layout = Some(layout.clone());
                        eng.window_layouts.insert(w_key, layout.clone());
                        eng.app_layouts.insert(format!("app:{}", win.app_id.to_lowercase()), layout);
                    }
                }
            }

            let mut child: Child = match Command::new("niri")
                .args(["msg", "--json", "event-stream"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    if self.config.general.debug {
                        eprintln!("[LayoutKeeper] Niri event-stream not ready: {e}. Retrying in 1s...");
                    }
                    std::thread::sleep(Duration::from_secs(1));
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

                if line.trim().is_empty() {
                    continue;
                }

                let parsed: Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // 1. Windows Changed (Update Window Cache)
                if let Some(windows) = parsed.get("WindowsChanged").and_then(|v| v.get("windows")).and_then(|v| v.as_array()) {
                    for w in windows {
                        if let Some(id) = w.get("id").and_then(|v| v.as_u64()) {
                            let app_id = w.get("app_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let title = w.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            self.windows_cache.insert(id, WindowInfo { id: id.to_string(), app_id, title });
                        }
                    }
                }

                // 2. Window Opened or Changed
                if let Some(w) = parsed.get("WindowOpenedOrChanged").and_then(|v| v.get("window")) {
                    if let Some(id) = w.get("id").and_then(|v| v.as_u64()) {
                        let app_id = w.get("app_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let title = w.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        self.windows_cache.insert(id, WindowInfo { id: id.to_string(), app_id, title });
                    }
                }

                // 3. Window Closed
                if let Some(id) = parsed.get("WindowClosed").and_then(|v| v.get("id")).and_then(|v| v.as_u64()) {
                    self.windows_cache.remove(&id);
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_window_close(&id.to_string());
                    }
                }

                // 4. Window Focus Changed
                if let Some(focus_evt) = parsed.get("WindowFocusChanged") {
                    let win_id_opt = focus_evt.get("id").and_then(|v| v.as_u64());
                    if let Some(win_id) = win_id_opt {
                        let win_info = match self.windows_cache.get(&win_id) {
                            Some(cached) if !cached.app_id.is_empty() && cached.app_id != "unknown" => {
                                cached.clone()
                            }
                            _ => {
                                if let Some(fresh) = self.get_current_window() {
                                    self.windows_cache.insert(win_id, fresh.clone());
                                    fresh
                                } else {
                                    WindowInfo {
                                        id: win_id.to_string(),
                                        app_id: "unknown".to_string(),
                                        title: String::new(),
                                    }
                                }
                            }
                        };

                        if let Ok(mut eng) = engine.lock() {
                            eng.on_focus_change(win_info, self);
                        }
                    }
                }

                // 5. Keyboard Layouts Changed
                if let Some(k_evt) = parsed.get("KeyboardLayoutsChanged").and_then(|v| v.get("keyboard_layouts")) {
                    if let Some(idx) = k_evt.get("current_idx").and_then(|v| v.as_u64()) {
                        if let Ok(mut eng) = engine.lock() {
                            eng.on_layout_change(idx.to_string());
                        }
                    }
                }

                // 6. Keyboard Layout Switched (Real-time active user switch)
                if let Some(switched) = parsed.get("KeyboardLayoutSwitched") {
                    if let Some(idx) = switched.get("idx").and_then(|v| v.as_u64()) {
                        if let Ok(mut eng) = engine.lock() {
                            eng.on_layout_change(idx.to_string());
                        }
                    }
                }
            }

            let _ = child.kill();
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}
