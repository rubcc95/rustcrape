mod commands;
mod persistence;
mod verboser;

use commands::AppState;
use persistence::ConfigStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            let app_local_data_dir = app.path().app_local_data_dir().expect("failed to get app local data dir");
            let store = ConfigStore::new(app_data_dir).expect("failed to init config store");
            app.manage(AppState::new(store, app_local_data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_configs,
            commands::save_config,
            commands::delete_config,
            commands::get_last_selected,
            commands::set_last_selected,
            commands::run_scraping,
            commands::cancel_scraping,
            commands::pick_executable,
            commands::load_global_settings,
            commands::save_global_settings,
            commands::check_announcement,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
