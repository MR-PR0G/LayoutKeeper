use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Every window retains its individual keyboard layout (like macOS/GNOME per-window, but cross-platform)
    PerWindow,
    /// Smart terminal mode: automatically switches to US layout when entering terminal, restores previous when leaving
    TerminalFocus,
    /// Hybrid mode: per-window layout memory across all desktop apps, but terminals always default to US
    Hybrid,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::PerWindow
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "per-window" | "perwindow" | "window" => Ok(Mode::PerWindow),
            "terminal-focus" | "terminal" | "terminals" => Ok(Mode::TerminalFocus),
            "hybrid" => Ok(Mode::Hybrid),
            _ => Err(format!(
                "Unknown mode '{}'. Valid modes: per-window, terminal-focus, hybrid",
                s
            )),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::PerWindow => write!(f, "per-window"),
            Mode::TerminalFocus => write!(f, "terminal-focus"),
            Mode::Hybrid => write!(f, "hybrid"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Active layout keeper mode
    pub mode: Mode,
    /// Default layout for new windows ("0", "us", "auto")
    pub default_layout: String,
    /// Track each window instance by unique window ID (true) or group by app_id / class (false)
    pub remember_per_window_id: bool,
    /// Debounce time in milliseconds to prevent rapid alt-tab jitter
    pub debounce_ms: u64,
    /// Enable detailed debug logging
    pub debug: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            mode: Mode::PerWindow,
            default_layout: "0".to_string(),
            remember_per_window_id: true,
            debounce_ms: 25,
            debug: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    /// List of terminal process names or window classes
    pub terminals: Vec<String>,
    /// Fallback US layout index or code
    pub us_layout: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            terminals: vec![
                "kitty".to_string(),
                "alacritty".to_string(),
                "foot".to_string(),
                "wezterm".to_string(),
                "org.gnome.terminal".to_string(),
                "gnome-terminal".to_string(),
                "konsole".to_string(),
                "ghostty".to_string(),
                "rio".to_string(),
                "warp-terminal".to_string(),
                "contour".to_string(),
                "tilix".to_string(),
                "terminator".to_string(),
                "xterm".to_string(),
                "urxvt".to_string(),
                "xfce4-terminal".to_string(),
                "qterminal".to_string(),
                "st".to_string(),
                "lxterminal".to_string(),
                "mate-terminal".to_string(),
                "guake".to_string(),
                "yakuake".to_string(),
            ],
            us_layout: "0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesConfig {
    /// Window classes or app_ids to completely ignore (games, lockscreens, overlays)
    pub ignored_apps: Vec<String>,
    /// Window classes / app_ids to enforce a fixed layout on focus
    pub enforce_layouts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub terminals: TerminalConfig,
    pub rules: RulesConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            terminals: TerminalConfig::default(),
            rules: RulesConfig {
                ignored_apps: vec![
                    "steam".to_string(),
                    "steam_app_.*".to_string(),
                    "gamescope".to_string(),
                    "org.kde.polkit-kde-authentication-agent-1".to_string(),
                    "polkit-gnome-authentication-agent-1".to_string(),
                    "rofi".to_string(),
                    "wofi".to_string(),
                    "dmenu".to_string(),
                    "swaylock".to_string(),
                    "hyprlock".to_string(),
                    "waylock".to_string(),
                ],
                enforce_layouts: HashMap::new(),
            },
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs_fallback().join("layoutkeeper")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn legacy_config_path() -> PathBuf {
        Self::config_dir().join("config")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<Config>(&content) {
                    return cfg;
                } else {
                    eprintln!("[LayoutKeeper] Warning: Could not parse config.toml, using defaults.");
                }
            }
        }

        // Check for legacy v1 config migration
        let legacy_path = Self::legacy_config_path();
        if legacy_path.exists() {
            let mut cfg = Config::default();
            if let Ok(content) = fs::read_to_string(&legacy_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.starts_with('[') || line.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        let k = k.trim();
                        let v = v.trim();
                        match k {
                            "terminals" => {
                                cfg.terminals.terminals = v
                                    .split(',')
                                    .map(|s| s.trim().to_lowercase())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                            }
                            "debug" => {
                                cfg.general.debug = v.eq_ignore_ascii_case("true");
                            }
                            "us_index" => {
                                cfg.terminals.us_layout = v.to_string();
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = cfg.save();
            return cfg;
        }

        // Initialize new default config
        let cfg = Config::default();
        let _ = cfg.save();
        cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Toml serialize error: {e}"))
        })?;

        let header = r#"# ==============================================================================
# LayoutKeeper Configuration (v2.0)
# Enterprise-grade Universal Per-Window Keyboard Layout Manager for Linux
# Documentation: https://github.com/MR-PR0G/LayoutKeeper
# ==============================================================================

"#;
        fs::write(Self::config_path(), format!("{header}{content}"))
    }

    pub fn is_terminal(&self, app_or_class: &str) -> bool {
        let needle = app_or_class.to_lowercase();
        self.terminals
            .terminals
            .iter()
            .any(|t| needle.contains(&t.to_lowercase()))
    }

    pub fn is_ignored(&self, app_or_class: &str) -> bool {
        let needle = app_or_class.to_lowercase();
        self.rules.ignored_apps.iter().any(|pattern| {
            let p = pattern.to_lowercase();
            if p.contains('*') {
                let prefix = p.trim_end_matches('*');
                needle.starts_with(prefix)
            } else {
                needle.contains(&p)
            }
        })
    }

    pub fn get_enforced_layout(&self, app_or_class: &str) -> Option<&String> {
        let needle = app_or_class.to_lowercase();
        for (app, layout) in &self.rules.enforce_layouts {
            if needle.contains(&app.to_lowercase()) {
                return Some(layout);
            }
        }
        None
    }
}

fn dirs_fallback() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from("/tmp/.config")
}
