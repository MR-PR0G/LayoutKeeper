use super::{CompositorBackend, EnvironmentInfo, WindowInfo};
use crate::config::Config;
use crate::engine::LayoutKeeperEngine;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct GenericBackend {
    #[allow(dead_code)]
    config: Config,
    env_name: &'static str,
}

impl GenericBackend {
    pub fn new(config: Config, env_name: &'static str) -> Self {
        Self { config, env_name }
    }
}

impl CompositorBackend for GenericBackend {
    fn name(&self) -> &'static str {
        "Generic"
    }

    fn environment_info(&self) -> EnvironmentInfo {
        EnvironmentInfo {
            display_server: self.env_name.to_string(),
            compositor: "Generic / Auto".to_string(),
            layout_tool: "Standard fallback".to_string(),
        }
    }

    fn get_current_window(&mut self) -> Option<WindowInfo> {
        if let Ok(out) = Command::new("xdotool").arg("getactivewindow").output() {
            if out.status.success() {
                let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return Some(WindowInfo {
                    id: id.clone(),
                    app_id: id,
                    title: String::new(),
                });
            }
        }
        None
    }

    fn get_current_layout(&mut self) -> Option<String> {
        let out = Command::new("setxkbmap").arg("-query").output().ok()?;
        let raw = String::from_utf8_lossy(&out.stdout);
        for line in raw.lines() {
            if line.starts_with("layout:") {
                return line.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }
        None
    }

    fn set_layout(&mut self, layout: &str) -> bool {
        Command::new("setxkbmap")
            .arg(layout)
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
        while stop_rx.try_recv().is_err() {
            if let Some(win) = self.get_current_window() {
                if win.id != prev_id {
                    prev_id = win.id.clone();
                    if let Ok(mut eng) = engine.lock() {
                        eng.on_focus_change(win, self);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
