// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Single-instance lock using a TCP listener on a fixed port.
    // This is more reliable than file locks because the OS automatically
    // releases the port when the process exits (no stale-lock problem,
    // no race condition on double-click).
    match std::net::TcpListener::bind("127.0.0.1:19876") {
        Ok(_listener) => {
            // First instance — keep _listener alive for the process lifetime.
            // When the process exits, the port is released automatically.
            app_lib::run();
            // _listener dropped here; port freed for the next launch.
        }
        Err(_) => {
            // Another instance is already running. Exit silently.
            std::process::exit(0);
        }
    }
}
