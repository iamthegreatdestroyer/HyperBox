//! HyperBox Desktop Application Library
//!
//! This library crate exposes the core functionality of the HyperBox desktop
//! application for use with Tauri's mobile targets.

pub mod commands;
pub mod daemon;
pub mod state;

pub use commands::*;
pub use daemon::DaemonClient;
pub use state::AppState;

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {}))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::default().build())
        .manage(std::sync::Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            // System commands
            commands::get_system_info,
            commands::get_daemon_status,
            commands::start_daemon,
            commands::stop_daemon,
            // Container commands
            commands::list_containers,
            commands::create_container,
            commands::start_container,
            commands::stop_container,
            commands::remove_container,
            commands::get_container_logs,
            commands::get_container_stats,
            // Image commands
            commands::list_images,
            commands::pull_image,
            commands::remove_image,
            // Project commands
            commands::list_projects,
            commands::open_project,
            commands::close_project,
            commands::start_project,
            commands::stop_project,
            commands::get_project_status,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
            commands::reset_settings,
            // Update commands
            commands::check_for_updates,
            commands::install_update,
            commands::get_app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
