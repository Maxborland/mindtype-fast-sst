// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Install crash handler first (before anything else can panic)
    mindtype_tauri::crash_reporter::install_crash_handler();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("mindtype=debug".parse().unwrap()),
        )
        .init();

    // Add initial breadcrumb
    mindtype_tauri::crash_reporter::add_breadcrumb("Application starting");

    mindtype_tauri::run();
}
