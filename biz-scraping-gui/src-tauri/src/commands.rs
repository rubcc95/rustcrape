use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biz_scraping::types::Config;
use tauri::{AppHandle, State};

use crate::persistence::{ConfigStore, SavedConfig};
use crate::verboser::TauriVerboser;

pub struct AppState {
    pub store: Mutex<ConfigStore>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(store: ConfigStore) -> Self {
        Self {
            store: Mutex::new(store),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[tauri::command]
pub fn list_configs(state: State<'_, AppState>) -> Result<Vec<SavedConfig>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(store.list())
}

#[tauri::command]
pub fn save_config(
    state: State<'_, AppState>,
    name: String,
    config: Config,
    started: bool,
) -> Result<SavedConfig, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let saved = store
        .save(&name, config, started)
        .map_err(|e| e.to_string())?;
    store
        .set_last_selected(Some(&saved.id))
        .map_err(|e| e.to_string())?;
    Ok(saved)
}

#[tauri::command]
pub fn delete_config(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.delete(&id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_last_selected(state: State<'_, AppState>) -> Result<Option<SavedConfig>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(store.get_last_selected())
}

#[tauri::command]
pub fn set_last_selected(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .set_last_selected(Some(&id))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn run_scraping(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Config,
) -> Result<(), String> {
    state.cancel_flag.store(false, Ordering::SeqCst);
    let cancel_flag = state.cancel_flag.clone();

    let (tx, rx) = std::sync::mpsc::channel::<std::result::Result<(), String>>();

    // std::thread::spawn(move || {
    //     let verboser = TauriVerboser::new(app, cancel_flag);
    //     let rt = match tokio::runtime::Runtime::new() {
    //         Ok(rt) => rt,
    //         Err(e) => {
    //             let _ = tx.send(Err(e.to_string()));
    //             return;
    //         }
    //     };
    //     let result = rt.block_on(biz_scraping::engine::run(config, verboser));
    //     let _ = tx.send(result.map_err(|e| e.to_string()));
    // });

    tauri::async_runtime::spawn(async move {
        let verboser = TauriVerboser::new(app, cancel_flag);
        // let rt = match tokio::runtime::Runtime::new() {
        //     Ok(rt) => rt,
        //     Err(e) => {
        //         let _ = tx.send(Err(e.to_string()));
        //         return;
        //     }
        // };
        biz_scraping::engine::run(config, verboser).await.map_err(|e| e.to_string())?;

    });

    tokio::task::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| format!("La tarea de scraping falló: {e}"))?
        .map_err(|_| "El canal de scraping se cerró inesperadamente".to_string())?
}

#[tauri::command]
pub fn cancel_scraping(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::SeqCst);
    Ok(())
}
