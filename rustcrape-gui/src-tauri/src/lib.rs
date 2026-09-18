mod commands;
mod persistence;
mod verboser;

use std::sync::atomic::Ordering;
use std::time::Duration;

use commands::AppState;
use persistence::ConfigStore;
use tauri::{Manager, WindowEvent};

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
        // Al cerrar la ventana con un scraping en curso, no matamos el proceso:
        // activamos el flag de cancelación (igual que el botón "Pausar"),
        // ocultamos la ventana y esperamos a que el motor escriba los datos
        // pendientes antes de salir.
        .on_window_event(|window, event| { 
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.is_scraping.load(Ordering::SeqCst) {
                    api.prevent_close();
                    state.cancel_flag.store(true, Ordering::SeqCst);
                    let _ = window.hide();
                    let app = window.app_handle().clone();
                    let is_scraping = state.is_scraping.clone();
                    tauri::async_runtime::spawn(async move {
                        while is_scraping.load(Ordering::SeqCst) {
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                        app.exit(0);
                    });
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_configs,
            commands::save_config,
            commands::delete_config,
            commands::get_last_selected,
            commands::set_last_selected,
            commands::run_scraping,
            commands::load_project_stats,
            commands::load_coincidences,
            commands::cancel_scraping,
            commands::pick_executable,
            commands::load_global_settings,
            commands::save_global_settings,
            commands::check_announcement,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
