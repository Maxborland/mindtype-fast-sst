//! MindType Tauri Application
//!
//! A speech-to-text desktop application with Mac OS 7/8 inspired UI.

pub mod commands;
pub mod crash_reporter;
pub mod hotkey;
pub mod recording;
pub mod state;
pub mod tray;

use tauri::Manager;

/// Run the Tauri application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Initialize app state
            let state = state::AppState::new()?;
            app.manage(state);

            // Setup global hotkeys
            hotkey::setup_hotkeys(app.handle())?;

            // Setup system tray
            tray::setup_tray(app)?;

            tracing::info!("MindType started successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::set_language,
            commands::download_model,
            commands::complete_setup,
            commands::start_recording,
            commands::stop_recording,
            commands::get_recent_transcriptions,
            commands::transcribe,
            commands::insert_text,
            commands::get_settings,
            commands::update_settings,
            commands::validate_license,
            commands::get_license_status,
            commands::activate_license,
            commands::deactivate_license,
            commands::open_url,
            // File processing commands
            commands::add_files_to_queue,
            commands::get_file_jobs,
            commands::start_file_processing,
            commands::remove_file_job,
            commands::clear_completed_jobs,
            commands::export_transcript,
            commands::open_file_dialog,
            // LLM configuration commands
            commands::get_llm_providers,
            commands::get_llm_config,
            commands::set_llm_config,
            commands::test_llm_connection,
            // Summarization commands
            commands::summarize_text,
            commands::summarize_file_job,
            commands::get_summary_presets,
            // Update commands
            commands::check_for_updates,
            commands::install_update,
            commands::get_app_version,
            // Crash report commands
            commands::submit_crash_report,
            commands::get_crash_reports_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
