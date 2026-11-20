use crate::config::{Config, Mode};
use crate::engine::LayoutKeeperEngine;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn socket_path() -> PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime.is_empty() {
            return PathBuf::from(runtime).join("layoutkeeper.sock");
        }
    }
    let uid = unsafe { libc_getuid() };
    PathBuf::from(format!("/tmp/layoutkeeper-{uid}.sock"))
}

unsafe fn libc_getuid() -> u32 {
    // Standard POSIX getuid()
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum IpcRequest {
    Status,
    Pause,
    Resume,
    Toggle,
    SetMode { mode: String },
    GetMemory,
    ClearMemory,
    Ignore { app: String },
    Unignore { app: String },
    AddTerminal { terminal: String },
    RemoveTerminal { terminal: String },
    ReloadConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

pub struct IpcServer;

impl IpcServer {
    pub fn start(
        engine: Arc<Mutex<LayoutKeeperEngine>>,
        stop_rx: std::sync::mpsc::Receiver<()>,
    ) {
        let sock = socket_path();
        let _ = std::fs::remove_file(&sock);

        let listener = match UnixListener::bind(&sock) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[LayoutKeeper] Failed to bind IPC socket {:?}: {e}", sock);
                return;
            }
        };

        let _ = listener.set_nonblocking(true);

        while stop_rx.try_recv().is_err() {
            match listener.accept() {
                Ok((stream, _)) => {
                    let engine_clone = Arc::clone(&engine);
                    std::thread::spawn(move || {
                        handle_client(stream, engine_clone);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }

        let _ = std::fs::remove_file(&sock);
    }
}

fn handle_client(mut stream: UnixStream, engine: Arc<Mutex<LayoutKeeperEngine>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    if reader.read_line(&mut line).is_ok() {
        let req: Result<IpcRequest, _> = serde_json::from_str(&line);
        let resp = match req {
            Ok(IpcRequest::Status) => {
                if let Ok(eng) = engine.lock() {
                    let status = eng.status();
                    IpcResponse {
                        success: true,
                        message: "Status retrieved".to_string(),
                        data: serde_json::to_value(&status).ok(),
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::Pause) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.is_paused = true;
                    IpcResponse {
                        success: true,
                        message: "LayoutKeeper paused".to_string(),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::Resume) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.is_paused = false;
                    IpcResponse {
                        success: true,
                        message: "LayoutKeeper resumed".to_string(),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::Toggle) => {
                if let Ok(mut eng) = engine.lock() {
                    let paused = eng.toggle_paused();
                    let msg = if paused { "LayoutKeeper paused" } else { "LayoutKeeper resumed" };
                    IpcResponse {
                        success: true,
                        message: msg.to_string(),
                        data: Some(serde_json::json!({ "paused": paused })),
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::SetMode { mode }) => {
                if let Ok(m) = mode.parse::<Mode>() {
                    if let Ok(mut eng) = engine.lock() {
                        eng.set_mode(m);
                        IpcResponse {
                            success: true,
                            message: format!("Mode switched to: {m}"),
                            data: None,
                        }
                    } else {
                        IpcResponse {
                            success: false,
                            message: "Lock error".to_string(),
                            data: None,
                        }
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: format!("Invalid mode: '{mode}'. Valid modes: per-window, terminal-focus, hybrid"),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::GetMemory) => {
                if let Ok(eng) = engine.lock() {
                    let data = serde_json::json!({
                        "windows": eng.window_layouts,
                        "apps": eng.app_layouts,
                        "terminals": eng.config.terminals.terminals,
                        "ignored": eng.config.rules.ignored_apps,
                    });
                    IpcResponse {
                        success: true,
                        message: "Memory retrieved".to_string(),
                        data: Some(data),
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::ClearMemory) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.clear_memory();
                    IpcResponse {
                        success: true,
                        message: "Layout memory cache cleared".to_string(),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::Ignore { app }) => {
                if let Ok(mut eng) = engine.lock() {
                    if !eng.config.rules.ignored_apps.iter().any(|a| a.eq_ignore_ascii_case(&app)) {
                        eng.config.rules.ignored_apps.push(app.clone());
                        let _ = eng.config.save();
                    }
                    IpcResponse {
                        success: true,
                        message: format!("Added '{app}' to ignored applications list"),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::Unignore { app }) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.config.rules.ignored_apps.retain(|a| !a.eq_ignore_ascii_case(&app));
                    let _ = eng.config.save();
                    IpcResponse {
                        success: true,
                        message: format!("Removed '{app}' from ignored applications list"),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::AddTerminal { terminal }) => {
                if let Ok(mut eng) = engine.lock() {
                    if !eng.config.terminals.terminals.iter().any(|t| t.eq_ignore_ascii_case(&terminal)) {
                        eng.config.terminals.terminals.push(terminal.clone());
                        let _ = eng.config.save();
                    }
                    IpcResponse {
                        success: true,
                        message: format!("Added '{terminal}' to monitored terminals list"),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::RemoveTerminal { terminal }) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.config.terminals.terminals.retain(|t| !t.eq_ignore_ascii_case(&terminal));
                    let _ = eng.config.save();
                    IpcResponse {
                        success: true,
                        message: format!("Removed '{terminal}' from monitored terminals list"),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Ok(IpcRequest::ReloadConfig) => {
                if let Ok(mut eng) = engine.lock() {
                    eng.config = Config::load();
                    IpcResponse {
                        success: true,
                        message: "Configuration reloaded successfully".to_string(),
                        data: None,
                    }
                } else {
                    IpcResponse {
                        success: false,
                        message: "Lock error".to_string(),
                        data: None,
                    }
                }
            }
            Err(e) => IpcResponse {
                success: false,
                message: format!("Malformed request: {e}"),
                data: None,
            },
        };

        if let Ok(serialized) = serde_json::to_string(&resp) {
            let _ = stream.write_all(serialized.as_bytes());
            let _ = stream.write_all(b"\n");
        }
    }
}

pub fn send_request(req: &IpcRequest) -> Result<IpcResponse, std::io::Error> {
    let sock = socket_path();
    if !sock.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "LayoutKeeper daemon is not running.",
        ));
    }

    let mut stream = UnixStream::connect(&sock)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let payload = serde_json::to_string(req).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;

    stream.write_all(payload.as_bytes())?;
    stream.write_all(b"\n")?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    serde_json::from_str(&response_line).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })
}
