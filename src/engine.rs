use crate::backend::{CompositorBackend, WindowInfo};
use crate::config::{Config, Mode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub paused: bool,
    pub mode: String,
    pub active_window: Option<WindowInfo>,
    pub current_layout: Option<String>,
    pub remembered_windows_count: usize,
    pub total_focus_events: u64,
    pub total_switches: u64,
    pub uptime_secs: u64,
}

pub struct LayoutKeeperEngine {
    pub config: Config,
    pub window_layouts: HashMap<String, String>,
    pub app_layouts: HashMap<String, String>,
    pub active_window: Option<WindowInfo>,
    pub current_layout: Option<String>,
    pub last_non_terminal_layout: Option<String>,
    pub is_paused: bool,
    pub started_at: Instant,
    pub total_focus_events: u64,
    pub total_layout_switches: u64,
}

impl LayoutKeeperEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            window_layouts: HashMap::new(),
            app_layouts: HashMap::new(),
            active_window: None,
            current_layout: None,
            last_non_terminal_layout: None,
            is_paused: false,
            started_at: Instant::now(),
            total_focus_events: 0,
            total_layout_switches: 0,
        }
    }

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            running: true,
            paused: self.is_paused,
            mode: self.config.general.mode.to_string(),
            active_window: self.active_window.clone(),
            current_layout: self.current_layout.clone(),
            remembered_windows_count: self.window_layouts.len(),
            total_focus_events: self.total_focus_events,
            total_switches: self.total_layout_switches,
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }

    fn win_key(&self, win: &WindowInfo) -> String {
        if self.config.general.remember_per_window_id && !win.id.is_empty() {
            format!("win:{}", win.id)
        } else {
            format!("app:{}", win.app_id.to_lowercase())
        }
    }

    fn app_key(&self, win: &WindowInfo) -> String {
        format!("app:{}", win.app_id.to_lowercase())
    }

    pub fn on_focus_change(
        &mut self,
        new_window: WindowInfo,
        backend: &mut dyn CompositorBackend,
    ) {
        if self.is_paused {
            return;
        }

        // Ignore empty windows
        if new_window.id.is_empty() && new_window.app_id.is_empty() {
            return;
        }

        // Avoid re-processing same window focus
        if let Some(active) = &self.active_window {
            if active.id == new_window.id && active.app_id == new_window.app_id {
                return;
            }
        }

        if self.config.general.debug {
            eprintln!(
                "[LayoutKeeper] Focus changed: [{}] {} ('{}')",
                new_window.id, new_window.app_id, new_window.title
            );
        }

        // 1. Ensure current_layout is known
        if self.current_layout.is_none() {
            self.current_layout = backend.get_current_layout();
        }

        // 2. Save previous window layout
        if let Some(prev) = &self.active_window {
            if !self.config.is_ignored(&prev.app_id) {
                if let Some(layout) = &self.current_layout {
                    let w_key = self.win_key(prev);
                    let a_key = self.app_key(prev);
                    self.window_layouts.insert(w_key, layout.clone());
                    self.app_layouts.insert(a_key, layout.clone());

                    if !self.config.is_terminal(&prev.app_id) {
                        self.last_non_terminal_layout = Some(layout.clone());
                    }
                }
            }
        }

        // 3. Immediately update active_window so any subsequent layout events belong to new_window
        self.active_window = Some(new_window.clone());
        self.total_focus_events += 1;

        // Check if new window is ignored
        if self.config.is_ignored(&new_window.app_id) {
            if self.config.general.debug {
                eprintln!(
                    "[LayoutKeeper] Window '{}' is in ignored list. Bypassing switch.",
                    new_window.app_id
                );
            }
            return;
        }

        // 4. Determine target layout for new window
        let target_layout = self.determine_target_layout(&new_window);

        if let Some(target) = target_layout {
            let needs_switch = match &self.current_layout {
                Some(curr) => curr != &target,
                None => true,
            };

            if needs_switch {
                if self.config.general.debug {
                    eprintln!(
                        "[LayoutKeeper] Setting layout for [{}] {} to '{}'",
                        new_window.id, new_window.app_id, target
                    );
                }
                if backend.set_layout(&target) {
                    self.current_layout = Some(target.clone());
                    self.total_layout_switches += 1;
                }
            }

            // Record target layout for new_window
            let w_key = self.win_key(&new_window);
            let a_key = self.app_key(&new_window);
            self.window_layouts.insert(w_key, target.clone());
            self.app_layouts.insert(a_key, target);
        }
    }

    fn determine_target_layout(&self, win: &WindowInfo) -> Option<String> {
        // 1. Explicit Enforced Layout Rule
        if let Some(enforced) = self.config.get_enforced_layout(&win.app_id) {
            return Some(enforced.clone());
        }

        let is_term = self.config.is_terminal(&win.app_id);

        match self.config.general.mode {
            Mode::TerminalFocus => {
                if is_term {
                    Some(self.config.terminals.us_layout.clone())
                } else {
                    // Leaving terminal: restore last non-terminal layout
                    self.last_non_terminal_layout.clone()
                }
            }
            Mode::Hybrid => {
                if is_term {
                    Some(self.config.terminals.us_layout.clone())
                } else {
                    // Per-window layout memory
                    self.resolve_saved_or_default(win)
                }
            }
            Mode::PerWindow => {
                let w_key = self.win_key(win);
                if let Some(saved) = self.window_layouts.get(&w_key) {
                    return Some(saved.clone());
                }

                let a_key = self.app_key(win);
                if let Some(saved_app) = self.app_layouts.get(&a_key) {
                    return Some(saved_app.clone());
                }

                if is_term {
                    return Some(self.config.terminals.us_layout.clone());
                }

                self.resolve_saved_or_default(win)
            }
        }
    }

    fn resolve_saved_or_default(&self, win: &WindowInfo) -> Option<String> {
        let w_key = self.win_key(win);
        if let Some(saved) = self.window_layouts.get(&w_key) {
            return Some(saved.clone());
        }

        let a_key = self.app_key(win);
        if let Some(saved_app) = self.app_layouts.get(&a_key) {
            return Some(saved_app.clone());
        }

        // New window without prior layout: check default_layout
        let def = self.config.general.default_layout.trim();
        if !def.is_empty() && !def.eq_ignore_ascii_case("auto") {
            Some(def.to_string())
        } else {
            None
        }
    }

    pub fn on_layout_change(&mut self, new_layout: String) {
        if self.config.general.debug {
            eprintln!("[LayoutKeeper] Active layout changed to: {}", new_layout);
        }

        self.current_layout = Some(new_layout.clone());

        if let Some(active) = &self.active_window {
            if !self.config.is_ignored(&active.app_id) {
                let w_key = self.win_key(active);
                let a_key = self.app_key(active);
                self.window_layouts.insert(w_key, new_layout.clone());
                self.app_layouts.insert(a_key, new_layout.clone());

                if !self.config.is_terminal(&active.app_id) {
                    self.last_non_terminal_layout = Some(new_layout);
                }
            }
        }
    }

    pub fn on_window_close(&mut self, window_id: &str) {
        let key = format!("win:{window_id}");
        self.window_layouts.remove(&key);
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.config.general.mode = mode;
        let _ = self.config.save();
    }

    pub fn toggle_paused(&mut self) -> bool {
        self.is_paused = !self.is_paused;
        self.is_paused
    }

    pub fn clear_memory(&mut self) {
        self.window_layouts.clear();
        self.app_layouts.clear();
        self.last_non_terminal_layout = None;
    }
}
