mod backend;
mod cli;
mod config;
mod engine;
mod ipc;

use backend::detect_backend;
use config::Config;
use engine::LayoutKeeperEngine;
use ipc::IpcServer;
use std::env;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check if invoked as daemon
    let is_daemon_arg = args.get(1).map(|s| s == "daemon" || s == "--daemon").unwrap_or(false);
    let is_systemd = env::var("INVOCATION_ID").is_ok() || env::var("JOURNAL_STREAM").is_ok();

    if is_daemon_arg || (args.len() == 1 && is_systemd) {
        run_daemon();
    } else {
        cli::run_cli(&args);
    }
}

fn run_daemon() {
    let config = Config::load();
    if config.general.debug {
        eprintln!(
            "[LayoutKeeper] Starting daemon v{} (mode: {})",
            env!("CARGO_PKG_VERSION"),
            config.general.mode
        );
    }

    let mut backend = detect_backend(&config);
    let env_info = backend.environment_info();

    eprintln!(
        "[LayoutKeeper] Environment: {} ({}), layout backend: {}",
        env_info.display_server, env_info.compositor, env_info.layout_tool
    );

    let engine = Arc::new(Mutex::new(LayoutKeeperEngine::new(config.clone())));

    // Initial state capture
    if let Some(win) = backend.get_current_window() {
        if let Some(layout) = backend.get_current_layout() {
            if let Ok(mut eng) = engine.lock() {
                eng.active_window = Some(win.clone());
                eng.current_layout = Some(layout.clone());
                let key = if config.general.remember_per_window_id && !win.id.is_empty() {
                    format!("win:{}", win.id)
                } else {
                    format!("app:{}", win.app_id.to_lowercase())
                };
                eng.window_layouts.insert(key, layout.clone());
                eng.app_layouts.insert(format!("app:{}", win.app_id.to_lowercase()), layout);
            }
        }
    }

    // Channel for stopping threads
    let (stop_tx_ipc, stop_rx_ipc) = std::sync::mpsc::channel();
    let (stop_tx_backend, stop_rx_backend) = std::sync::mpsc::channel();

    // Start IPC Server thread
    let engine_ipc = Arc::clone(&engine);
    let ipc_handle = std::thread::spawn(move || {
        IpcServer::start(engine_ipc, stop_rx_ipc);
    });

    // Setup signal handling (SIGINT, SIGTERM)
    let stop_backend_clone = stop_tx_backend.clone();
    let stop_ipc_clone = stop_tx_ipc.clone();

    // Spawn signal listener thread
    std::thread::spawn(move || {
        // Register signal handlers using libc or standard loop
        unsafe {
            libc_signal(libc_sigterm(), handle_sig);
            libc_signal(libc_sigint(), handle_sig);
        }

        while RUNNING.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        let _ = stop_backend_clone.send(());
        let _ = stop_ipc_clone.send(());
    });

    // Start backend event loop (runs in current thread)
    backend.start_event_loop(Arc::clone(&engine), stop_rx_backend);

    RUNNING.store(false, Ordering::SeqCst);
    let _ = stop_tx_ipc.send(());
    let _ = ipc_handle.join();

    eprintln!("[LayoutKeeper] Daemon shut down cleanly.");
}

extern "C" fn handle_sig(_: i32) {
    RUNNING.store(false, Ordering::SeqCst);
}

unsafe fn libc_sigterm() -> i32 {
    15
}

unsafe fn libc_sigint() -> i32 {
    2
}

unsafe fn libc_signal(sig: i32, handler: extern "C" fn(i32)) {
    extern "C" {
        fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    }
    signal(sig, handler);
}
