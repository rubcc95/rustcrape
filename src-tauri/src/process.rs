use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub nombre: String,
    pub email: String,
    pub web: String,
    pub tfno: String,
    pub maps_url: String,
}

pub struct SidecarManager {
    child: Arc<Mutex<Option<tauri_plugin_shell::process::CommandChild>>>,
}

impl SidecarManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }

    fn sidecar_ts_path() -> Option<String> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
        let p = std::path::Path::new(&manifest)
            .parent()?
            .join("sidecar")
            .join("src")
            .join("main.ts");
        if p.exists() {
            Some(p.to_string_lossy().to_string())
        } else {
            None
        }
    }

    pub async fn spawn(
        &self,
        config: &AppConfig,
        lat: f64,
        lng: f64,
        app: &AppHandle,
    ) -> Result<mpsc::Receiver<Result<ScrapeResult, String>>, String> {
        let ua = config.user_agents.first().cloned().unwrap_or_default();
        let browser_path = config.browser_path.clone().unwrap_or_default();

        let mut cmd = if cfg!(debug_assertions) {
            let ts_path = Self::sidecar_ts_path()
                .ok_or_else(|| "No se encontró sidecar/src/main.ts para debug".to_string())?;
            app.shell()
                .command("bun") 
                .args(["run", &ts_path])
        } else {
            app.shell()
                .sidecar("scraper-sidecar")
                .map_err(|e| format!("Error creando sidecar: {}", e))?
        };

        cmd = cmd.args([
            "--lat", &lat.to_string(),
            "--lng", &lng.to_string(),
            "--search", &config.search_term,
            "--zoom", &config.zoom.to_string(),
            "--threshold", "5",
            "--ua", &ua,
        ]);

        if !browser_path.is_empty() {
            cmd = cmd.args(["--browser", &browser_path]);
        }

        let (mut rx, child) = cmd
            .spawn()
            .map_err(|e| format!("Error al spawnear sidecar: {}", e))?;

        *self.child.lock().await = Some(child);

        let (tx, rx_result) = mpsc::channel::<Result<ScrapeResult, String>>(100);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(bytes) => {
                        let line = String::from_utf8_lossy(&bytes);
                        let trimmed = line.trim().to_string();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(result) = serde_json::from_str::<ScrapeResult>(&trimmed) {
                            let _ = tx.send(Ok(result)).await;
                        } else if let Ok(err_val) =
                            serde_json::from_str::<serde_json::Value>(&trimmed)
                        {
                            if let Some(err) = err_val.get("error").and_then(|v| v.as_str()) {
                                let _ = tx.send(Err(err.to_string())).await;
                            }
                        }
                    }
                    CommandEvent::Stderr(bytes) => {
                        let line = String::from_utf8_lossy(&bytes);
                        let trimmed = line.trim().to_string();
                        if !trimmed.is_empty() {
                            let _ = tx.send(Err(trimmed)).await;
                        }
                    }
                    CommandEvent::Terminated(_) => break,
                    _ => {}
                }
            }
        });

        Ok(rx_result)
    }

    pub async fn kill(&self) -> Result<(), String> {
        let mut child_opt = self.child.lock().await;
        if let Some(child) = child_opt.take() {
            child.kill().map_err(|e| format!("Error matando sidecar: {}", e))?;
        }
        Ok(())
    }
}
