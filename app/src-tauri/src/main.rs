//! HyperBox Desktop Application
//!
//! Tauri 2.0 based desktop application for container management.
//! Provides a modern UI for the HyperBox container platform.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod daemon;
mod state;

use commands::*;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing window if another instance is launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            // Initialize application state
            let state = Arc::new(AppState::new());
            app.manage(state.clone());

            // Connect to daemon (clone Arc for async task)
            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = state_clone.connect_daemon().await {
                    log::warn!("Failed to connect to daemon: {}", e);
                }
            });

            log::info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // System commands
            get_system_info,
            get_version,
            check_daemon_status,
            start_daemon,
            stop_daemon,
            // Container commands
            list_containers,
            get_container,
            create_container,
            start_container,
            stop_container,
            restart_container,
            remove_container,
            get_container_logs,
            get_container_stats,
            // Image commands
            list_images,
            pull_image,
            remove_image,
            // Project commands
            list_projects,
            open_project,
            close_project,
            start_project,
            stop_project,
            get_project_status,
            // Performance commands
            get_performance_metrics,
            run_benchmark,
            // Settings
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("Error running HyperBox Desktop");
}
