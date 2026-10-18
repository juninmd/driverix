pub mod backup;
mod commands;
pub mod error;
pub mod guard;
pub mod host;
pub mod install;
pub mod inventory;
pub mod matching;
pub mod model;
pub mod ps;
pub mod scan;
pub mod sys;
pub mod updates;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::load_devices,
            commands::scan_updates,
            commands::cancel_operation,
            commands::install_updates,
            commands::list_backups,
            commands::rollback_backup,
            commands::relaunch_elevated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
