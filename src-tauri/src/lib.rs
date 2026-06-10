mod config;
mod db;
mod detect;
mod generator;
mod process;

use config::{load, save, AppConfig};
use db::Database;
use db::Resultado;
use detect::BrowserInfo;
use process::SidecarManager;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::Mutex;

struct AppState {
    config: Arc<Mutex<AppConfig>>,
    db: Arc<Mutex<Option<Database>>>,
    sidecar: Arc<SidecarManager>,
    running: Arc<Mutex<bool>>,
}

impl AppState {
    fn new() -> Self {
        let cfg = load();
        eprintln!("[DEBUG APP] AppState::new() config loaded, search_term='{}'", cfg.search_term);
        Self {
            config: Arc::new(Mutex::new(cfg)),
            db: Arc::new(Mutex::new(None)),
            sidecar: Arc::new(SidecarManager::new()),
            running: Arc::new(Mutex::new(false)),
        }
    }
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, String> {
    let cfg = state.config.lock().await;
    Ok(cfg.clone())
}

#[tauri::command]
async fn save_config(
    state: tauri::State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String> {
    eprintln!("[DEBUG APP] save_config called, search_term='{}'", config.search_term);
    save(&config)?;
    *state.config.lock().await = config;
    eprintln!("[DEBUG APP] save_config OK");
    Ok(())
}

#[tauri::command]
async fn detect_browsers() -> Vec<BrowserInfo> {
    detect::detect_browsers()
}

#[tauri::command]
async fn install_browsers(_app: tauri::AppHandle, browsers: Vec<String>) -> Result<(), String> {
    for browser in browsers {
        let output = std::process::Command::new("npx")
            .arg("playwright")
            .arg("install")
            .arg(&browser)
            .output()
            .map_err(|e| format!("Error instalando {}: {}", browser, e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Error instalando {}: {}", browser, stderr));
        }
    }
    Ok(())
}

#[tauri::command]
async fn generate_grid(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    zoom: u32,
) -> Result<u64, String> {
    eprintln!("[DEBUG APP] generate_grid called, zoom={}", zoom);

    {
        let mut db_lock = state.db.lock().await;
        if db_lock.is_none() {
            let cfg = state.config.lock().await;
            *db_lock = Some(Database::new(&cfg).await?);
        }
    }

    let centers = generator::generate_grid(zoom)?;
    let count = centers.len() as u64;

    let db_lock = state.db.lock().await;
    let database = db_lock.as_ref().unwrap();
    database.insert_cuadrantes(&centers).await?;

    let _ = app.emit("grid_done", count);
    eprintln!("[DEBUG APP] generate_grid complete: {} quadrants", count);

    Ok(count)
}

#[tauri::command]
async fn start_scraping(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    eprintln!("[DEBUG APP] start_scraping called");
    let mut running = state.running.lock().await;
    if *running {
        return Err("Ya hay un proceso de scraping en ejecucion".into());
    }
    *running = true;
    drop(running);

    let config = {
        let cfg = state.config.lock().await;
        cfg.clone()
    };
    eprintln!("[DEBUG APP] config loaded: search_term='{}', iterations={}", config.search_term, config.iterations);
    let iterations = config.iterations;

    {
        let mut db_lock = state.db.lock().await;
        if db_lock.is_none() {
            eprintln!("[DEBUG APP] initializing database...");
            *db_lock = Some(Database::new(&config).await?);
            eprintln!("[DEBUG APP] database initialized");
        }
    }

    let database = {
        let db_lock = state.db.lock().await;
        db_lock.as_ref().unwrap().conn.clone()
    };

    let running_flag = state.running.clone();
    let sidecar = state.sidecar.clone();
    let app_handle = app.clone();

    tokio::spawn(async move {
        let mut count: u64 = 0;
        let max_iter = if iterations == 0 { u64::MAX } else { iterations as u64 };
        eprintln!("[DEBUG APP] scraping loop started, max_iter={:?}", if iterations == 0 { "unlimited".to_string() } else { max_iter.to_string() });

        loop {
            if !*running_flag.lock().await {
                eprintln!("[DEBUG APP] running flag false, breaking");
                break;
            }

            let db = Database { conn: database.clone() };
            let cuadrante = db.get_cuadrante().await;
            eprintln!("[DEBUG APP] got cuadrante: {:?}", cuadrante);

            match cuadrante {
                Ok(Some((lat, lng))) => {
                    eprintln!("[DEBUG APP] processing quadrant: lat={}, lng={}", lat, lng);
                    let _ = db.delete_cuadrante(lat, lng).await;
                    eprintln!("[DEBUG APP] deleted quadrant");
                    let sidecar_cfg = config.clone();
                    eprintln!("[DEBUG APP] calling sidecar.spawn()...");
                    let mut rx = match sidecar.spawn(&sidecar_cfg, lat, lng, &app_handle).await {
                        Ok(rx) => {
                            eprintln!("[DEBUG APP] sidecar.spawn() OK, got rx");
                            rx
                        }
                        Err(e) => {
                            eprintln!("[DEBUG APP] sidecar.spawn() ERROR: {}", e);
                            let _ = db.reinsert_cuadrante(lat, lng).await;
                            let _ = app_handle.emit("error", format!("Sidecar error: {}", e));
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            continue;
                        }
                    };

                    eprintln!("[DEBUG APP] waiting for rx.recv()...");
                    let mut result_count = 0;
                    while let Some(result) = rx.recv().await {
                        result_count += 1;
                        eprintln!("[DEBUG APP] rx result #{}: {:?}", result_count, result);
                        match result {
                            Ok(scrape_result) => {
                                let resultado = Resultado {
                                    id: None,
                                    nombre: scrape_result.nombre,
                                    email: scrape_result.email,
                                    web: scrape_result.web,
                                    tfno: scrape_result.tfno,
                                    maps: scrape_result.maps_url,
                                };
                                let _ = db.insert_resultado(&resultado).await;
                                eprintln!("[DEBUG APP] inserted resultado: {}", resultado.nombre);
                            }
                            Err(e) => {
                                eprintln!("[DEBUG APP] error from sidecar: {}", e);
                                let _ = app_handle.emit("error", e);
                            }
                        }
                    }
                    eprintln!("[DEBUG APP] rx stream ended, received {} results", result_count);

                    let _ = sidecar.kill().await;
                    count += 1;
                    eprintln!("[DEBUG APP] progress: {}", count);
                    let _ = app_handle.emit("progress", count);

                    if count >= max_iter {
                        eprintln!("[DEBUG APP] reached max iterations, breaking");
                        break;
                    }
                }
                Ok(None) => {
                    eprintln!("[DEBUG APP] no more cuadrantes");
                    let _ = app_handle.emit("done", "No hay mas cuadrantes");
                    break;
                }
                Err(e) => {
                    eprintln!("[DEBUG APP] DB error: {}", e);
                    let _ = app_handle.emit("error", format!("BD error: {}", e));
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }

        *running_flag.lock().await = false;
        eprintln!("[DEBUG APP] scraping loop finished");
        let _ = app_handle.emit("done", "Scraping finalizado");
    });

    eprintln!("[DEBUG APP] start_scraping returning OK");
    Ok(())
}

#[tauri::command]
async fn stop_scraping(state: tauri::State<'_, AppState>) -> Result<(), String> {
    eprintln!("[DEBUG APP] stop_scraping called");
    *state.running.lock().await = false;
    state.sidecar.kill().await
}

#[tauri::command]
async fn get_progress(state: tauri::State<'_, AppState>) -> Result<u64, String> {
    let db_lock = state.db.lock().await;
    if let Some(db) = db_lock.as_ref() {
        let total = db.get_cuadrante_count().await?;
        let done = db.get_resultado_count().await?;
        return Ok(done + total);
    }
    Ok(0)
}

#[tauri::command]
async fn get_results(
    state: tauri::State<'_, AppState>,
    page: u64,
    page_size: u64,
) -> Result<Vec<Resultado>, String> {
    let db_lock = state.db.lock().await;
    if let Some(db) = db_lock.as_ref() {
        return db.get_resultados(page, page_size).await;
    }
    Ok(Vec::new())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            detect_browsers,
            install_browsers,
            generate_grid,
            start_scraping,
            stop_scraping,
            get_progress,
            get_results,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
