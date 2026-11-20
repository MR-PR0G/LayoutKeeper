use crate::backend::detect_backend;
use crate::config::Config;
use crate::ipc::{send_request, IpcRequest, IpcResponse};
use std::process::Command;

pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const RESET: &str = "\x1b[0m";

pub fn run_cli(args: &[String]) {
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match command {
        "status" => cmd_status(),
        "on" | "start" | "enable" => cmd_on(),
        "off" | "stop" | "disable" => cmd_off(),
        "restart" => cmd_restart(),
        "pause" => cmd_pause(),
        "resume" => cmd_resume(),
        "toggle" => cmd_toggle(),
        "mode" => {
            if let Some(m) = args.get(2) {
                cmd_set_mode(m);
            } else {
                cmd_get_mode();
            }
        }
        "list" | "memory" => cmd_list(),
        "clear" => cmd_clear(),
        "ignore" => {
            if let Some(app) = args.get(2) {
                cmd_ignore(app);
            } else {
                eprintln!("{RED}Error: Missing application name.{RESET} Usage: layoutkeeper ignore <app>");
            }
        }
        "unignore" => {
            if let Some(app) = args.get(2) {
                cmd_unignore(app);
            } else {
                eprintln!("{RED}Error: Missing application name.{RESET} Usage: layoutkeeper unignore <app>");
            }
        }
        "add" => {
            if let Some(term) = args.get(2) {
                cmd_add_terminal(term);
            } else {
                eprintln!("{RED}Error: Missing terminal name.{RESET} Usage: layoutkeeper add <terminal>");
            }
        }
        "remove" => {
            if let Some(term) = args.get(2) {
                cmd_remove_terminal(term);
            } else {
                eprintln!("{RED}Error: Missing terminal name.{RESET} Usage: layoutkeeper remove <terminal>");
            }
        }
        "logs" => cmd_logs(),
        "config" => cmd_config(),
        "version" | "-v" | "--version" => {
            println!("LayoutKeeper v{}", env!("CARGO_PKG_VERSION"));
        }
        "help" | "-h" | "--help" => cmd_help(),
        other => {
            eprintln!("{RED}Unknown command:{RESET} {other}\n");
            cmd_help();
        }
    }
}

fn cmd_status() {
    match send_request(&IpcRequest::Status) {
        Ok(IpcResponse {
            success: true,
            data: Some(val),
            ..
        }) => {
            let paused = val.get("paused").and_then(|v| v.as_bool()).unwrap_or(false);
            let mode = val.get("mode").and_then(|v| v.as_str()).unwrap_or("unknown");
            let win = val.get("active_window");
            let win_id = win.and_then(|w| w.get("id")).and_then(|v| v.as_str()).unwrap_or("None");
            let win_app = win.and_then(|w| w.get("app_id")).and_then(|v| v.as_str()).unwrap_or("None");
            let win_title = win.and_then(|w| w.get("title")).and_then(|v| v.as_str()).unwrap_or("");
            let layout = val.get("current_layout").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let count = val.get("remembered_windows_count").and_then(|v| v.as_u64()).unwrap_or(0);
            let total_switches = val.get("total_switches").and_then(|v| v.as_u64()).unwrap_or(0);
            let uptime_secs = val.get("uptime_secs").and_then(|v| v.as_u64()).unwrap_or(0);

            let uptime_str = format_duration(uptime_secs);
            let status_badge = if paused {
                format!("{YELLOW}⏸ Paused{RESET}")
            } else {
                format!("{GREEN}● Active (Running){RESET}")
            };

            let cfg = Config::load();
            let backend = detect_backend(&cfg);
            let env_info = backend.environment_info();

            println!("{CYAN}{BOLD}┌─────────────────────────────────────────────────────────────┐{RESET}");
            println!("{CYAN}{BOLD}│  ⌨️  LayoutKeeper v{} — Universal Keyboard Layout Manager │{RESET}", env!("CARGO_PKG_VERSION"));
            println!("{CYAN}{BOLD}├─────────────────────────────────────────────────────────────┤{RESET}");
            println!("│  Status          : {status_badge}");
            println!("│  Operating Mode  : {MAGENTA}{BOLD}{mode}{RESET}");
            println!("│  Display Server  : {YELLOW}{}{RESET} ({CYAN}{}{RESET})", env_info.display_server, env_info.compositor);
            println!("│  Layout Backend  : {BLUE}{}{RESET}", env_info.layout_tool);
            println!("│  Active Layout   : {GREEN}{BOLD}{layout}{RESET}");
            println!("│  Focused Window  : [{CYAN}{win_id}{RESET}] {BOLD}{win_app}{RESET} {DIM}({win_title}){RESET}");
            println!("│  Memory Table    : {CYAN}{count}{RESET} windows tracked");
            println!("│  Total Switches  : {YELLOW}{total_switches}{RESET}");
            println!("│  Daemon Uptime   : {DIM}{uptime_str}{RESET}");
            println!("{CYAN}{BOLD}└─────────────────────────────────────────────────────────────┘{RESET}");
        }
        _ => {
            println!("{YELLOW}LayoutKeeper daemon is not currently active.{RESET}");
            let is_systemd = Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", "layoutkeeper.service"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

            if is_systemd {
                println!("Systemd user service is installed. Starting daemon...");
                let _ = Command::new("systemctl")
                    .args(["--user", "start", "layoutkeeper.service"])
                    .status();
            } else {
                println!("Run '{CYAN}layoutkeeper on{RESET}' to enable and start the daemon, or '{CYAN}layoutkeeper daemon{RESET}' to test in foreground.");
            }
        }
    }
}

fn cmd_on() {
    let _ = Command::new("systemctl")
        .args(["--user", "enable", "--now", "layoutkeeper.service"])
        .status();

    // If IPC is reachable, ensure it is unpaused
    let _ = send_request(&IpcRequest::Resume);
    println!("{GREEN}{BOLD}✔ LayoutKeeper service enabled and started.{RESET}");
}

fn cmd_off() {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "layoutkeeper.service"])
        .status();
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "layoutkeeper.service"])
        .status();
    println!("{YELLOW}LayoutKeeper service stopped and disabled.{RESET}");
}

fn cmd_restart() {
    let _ = Command::new("systemctl")
        .args(["--user", "restart", "layoutkeeper.service"])
        .status();
    println!("{GREEN}✔ LayoutKeeper service restarted.{RESET}");
}

fn cmd_pause() {
    match send_request(&IpcRequest::Pause) {
        Ok(resp) => println!("{YELLOW}{}{RESET}", resp.message),
        Err(_) => eprintln!("{RED}Daemon not running. Start with 'layoutkeeper on'.{RESET}"),
    }
}

fn cmd_resume() {
    match send_request(&IpcRequest::Resume) {
        Ok(resp) => println!("{GREEN}{}{RESET}", resp.message),
        Err(_) => eprintln!("{RED}Daemon not running. Start with 'layoutkeeper on'.{RESET}"),
    }
}

fn cmd_toggle() {
    match send_request(&IpcRequest::Toggle) {
        Ok(resp) => println!("{CYAN}{BOLD}{}{RESET}", resp.message),
        Err(_) => eprintln!("{RED}Daemon not running.{RESET}"),
    }
}

fn cmd_set_mode(mode: &str) {
    match send_request(&IpcRequest::SetMode {
        mode: mode.to_string(),
    }) {
        Ok(resp) => {
            if resp.success {
                println!("{GREEN}✔ {}{RESET}", resp.message);
            } else {
                eprintln!("{RED}Error: {}{RESET}", resp.message);
            }
        }
        Err(_) => {
            // Update config directly if daemon is stopped
            let mut cfg = Config::load();
            match mode.parse() {
                Ok(m) => {
                    cfg.general.mode = m;
                    let _ = cfg.save();
                    println!("{GREEN}✔ Mode set to '{m}' in configuration.{RESET}");
                }
                Err(e) => eprintln!("{RED}Error: {e}{RESET}"),
            }
        }
    }
}

fn cmd_get_mode() {
    let cfg = Config::load();
    println!("Current mode: {MAGENTA}{BOLD}{}{RESET}", cfg.general.mode);
    println!("Available modes: {CYAN}per-window{RESET}, {CYAN}terminal-focus{RESET}, {CYAN}hybrid{RESET}");
}

fn cmd_list() {
    match send_request(&IpcRequest::GetMemory) {
        Ok(IpcResponse {
            success: true,
            data: Some(val),
            ..
        }) => {
            println!("{BOLD}[Tracked Windows & Memory Table]{RESET}\n");
            if let Some(windows) = val.get("windows").and_then(|v| v.as_object()) {
                if windows.is_empty() {
                    println!("  {DIM}(No active windows recorded yet){RESET}\n");
                } else {
                    for (win, layout) in windows {
                        println!("  • {CYAN}{win:24}{RESET} -> Layout: {GREEN}{BOLD}{layout}{RESET}");
                    }
                    println!();
                }
            }

            println!("{BOLD}[Monitored Terminals]{RESET}");
            if let Some(terms) = val.get("terminals").and_then(|v| v.as_array()) {
                let term_str = terms
                    .iter()
                    .filter_map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  {term_str}\n");
            }

            println!("{BOLD}[Ignored Applications]{RESET}");
            if let Some(ignored) = val.get("ignored").and_then(|v| v.as_array()) {
                let ign_str = ignored
                    .iter()
                    .filter_map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  {ign_str}\n");
            }
        }
        _ => {
            let cfg = Config::load();
            println!("{BOLD}Configured Terminals:{RESET}");
            println!("  {}\n", cfg.terminals.terminals.join(", "));
            println!("{BOLD}Ignored Apps:{RESET}");
            println!("  {}\n", cfg.rules.ignored_apps.join(", "));
        }
    }
}

fn cmd_clear() {
    match send_request(&IpcRequest::ClearMemory) {
        Ok(resp) => println!("{GREEN}✔ {}{RESET}", resp.message),
        Err(_) => println!("{YELLOW}Daemon is not running.{RESET}"),
    }
}

fn cmd_ignore(app: &str) {
    let _ = send_request(&IpcRequest::Ignore {
        app: app.to_string(),
    });
    let mut cfg = Config::load();
    if !cfg.rules.ignored_apps.iter().any(|a| a.eq_ignore_ascii_case(app)) {
        cfg.rules.ignored_apps.push(app.to_string());
        let _ = cfg.save();
    }
    println!("{GREEN}✔ Added '{app}' to ignored applications list.{RESET}");
}

fn cmd_unignore(app: &str) {
    let _ = send_request(&IpcRequest::Unignore {
        app: app.to_string(),
    });
    let mut cfg = Config::load();
    cfg.rules.ignored_apps.retain(|a| !a.eq_ignore_ascii_case(app));
    let _ = cfg.save();
    println!("{GREEN}✔ Removed '{app}' from ignored applications list.{RESET}");
}

fn cmd_add_terminal(term: &str) {
    let _ = send_request(&IpcRequest::AddTerminal {
        terminal: term.to_string(),
    });
    let mut cfg = Config::load();
    if !cfg.terminals.terminals.iter().any(|t| t.eq_ignore_ascii_case(term)) {
        cfg.terminals.terminals.push(term.to_string());
        let _ = cfg.save();
    }
    println!("{GREEN}✔ Added '{term}' to monitored terminals.{RESET}");
}

fn cmd_remove_terminal(term: &str) {
    let _ = send_request(&IpcRequest::RemoveTerminal {
        terminal: term.to_string(),
    });
    let mut cfg = Config::load();
    cfg.terminals.terminals.retain(|t| !t.eq_ignore_ascii_case(term));
    let _ = cfg.save();
    println!("{GREEN}✔ Removed '{term}' from monitored terminals.{RESET}");
}

fn cmd_logs() {
    let _ = Command::new("journalctl")
        .args(["--user", "-u", "layoutkeeper", "-f", "-n", "40"])
        .status();
}

fn cmd_config() {
    let path = Config::config_path();
    println!("{BOLD}Configuration file path:{RESET} {CYAN}{}{RESET}\n", path.display());
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            println!("{content}");
        }
    } else {
        println!("{YELLOW}Config file has not been created yet. Generating default config...{RESET}");
        let _ = Config::load().save();
    }
}

fn cmd_help() {
    println!("{CYAN}{BOLD}LayoutKeeper v{} — Universal Linux Keyboard Layout Manager{RESET}", env!("CARGO_PKG_VERSION"));
    println!("Usage: {BOLD}layoutkeeper [command] [options]{RESET}\n");
    println!("{BOLD}Commands:{RESET}");
    println!("  {GREEN}status{RESET}                     Show live service status, mode, active window, layout");
    println!("  {GREEN}on / start{RESET}                 Enable and start the background daemon");
    println!("  {GREEN}off / stop{RESET}                 Stop and disable the background daemon");
    println!("  {GREEN}restart{RESET}                    Restart the background daemon");
    println!("  {GREEN}pause{RESET}                      Temporarily pause layout switching");
    println!("  {GREEN}resume{RESET}                     Resume layout switching");
    println!("  {GREEN}toggle{RESET}                     Toggle pause/resume state (useful for hotkeys)");
    println!("  {GREEN}mode <mode>{RESET}                Switch mode ({CYAN}per-window{RESET} | {CYAN}terminal-focus{RESET} | {CYAN}hybrid{RESET})");
    println!("  {GREEN}list{RESET}                       List tracked window memory, terminals, and ignored apps");
    println!("  {GREEN}clear{RESET}                      Clear remembered window layouts memory");
    println!("  {GREEN}ignore <app>{RESET}               Add an application to bypass layout switching (e.g. steam)");
    println!("  {GREEN}unignore <app>{RESET}             Remove an application from ignore list");
    println!("  {GREEN}add <term>{RESET}                 Add a terminal emulator to monitored list");
    println!("  {GREEN}remove <term>{RESET}              Remove a terminal from monitored list");
    println!("  {GREEN}config{RESET}                     Show configuration file path and contents");
    println!("  {GREEN}logs{RESET}                       Tail live system logs from journalctl");
    println!("  {GREEN}daemon{RESET}                     Run daemon in foreground (used by systemd)");
    println!("  {GREEN}version{RESET}                    Show version information");
    println!("  {GREEN}help{RESET}                       Show this help message\n");
    println!("{BOLD}Examples:{RESET}");
    println!("  layoutkeeper status");
    println!("  layoutkeeper mode per-window");
    println!("  layoutkeeper ignore gamescope");
    println!("  layoutkeeper toggle");
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {secs}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}
