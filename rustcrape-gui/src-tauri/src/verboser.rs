use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rustcrape::types::Coincidence;
use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct VerboserPayload {
    pub kind: String,
    pub message: String,
}

pub struct TauriVerboser {
    app_handle: tauri::AppHandle,
    cancel_flag: Arc<AtomicBool>,
}

impl TauriVerboser {
    pub fn new(app_handle: tauri::AppHandle, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            app_handle,
            cancel_flag,
        }
    }

    fn emit(&self, kind: &str, message: String) {
        let _ = self.app_handle.emit(
            "verboser-event",
            VerboserPayload {
                kind: kind.to_string(),
                message,
            },
        );
    }
} 

impl rustcrape::verboser::Verboser for TauriVerboser {
    fn seeding_tasks(&self, checked: usize, valid: usize, total: usize) {
        self.emit(
            "seeding_tasks",
            format!("Generando tareas: {checked} revisados, {valid} válidos de {total} totales"),
        );
    }

    fn connecting_db(&self) {
        self.emit("connecting_db", "Conectando a la base de datos...".into());
    }

    fn creating_db(&self) {
        self.emit("creating_db", "Creando base de datos...".into());
    }

    fn verifying_db(&self) {
        self.emit("verifying_db", "Verificando base de datos...".into());
    }

    fn creating_tables(&self) {
        self.emit("creating_tables", "Creando tablas...".into());
    }

    fn rate_limit_wait(&self, wait: std::time::Duration) {
        let total_secs = wait.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        self.emit(
            "rate_limit_wait",
            format!("Límite de tasa alcanzado. Esperando {mins}:{secs:02} minutos..."),
        );
    }

    fn obtaining_task(&self) {
        self.emit("obtaining_task", "Obteniendo tarea para procesar...".into());
    }

    fn claimed_task(&self, label: &str) {
        self.emit("claimed_task", format!("Tarea reclamada en {label}"));
    }

    fn released_task(&self) {
        self.emit("released_task", "Tarea liberada".into());
    }

    fn finished(&self) {
        self.emit("finished", "Procesamiento completado.".into());
    }

    fn opening_browser(&self, label: &str) {
        self.emit(
            "opening_browser",
            format!("Abriendo navegador en {label}..."),
        );
    }

    fn closing_browser(&self) {
        self.emit("closing_browser", "Cerrando navegador...".into());
    }

    fn scraping_start(&self) {
        self.emit("scraping_start", "Scrapeando...".into());
    }

    fn accepting_cookies(&self) {
        self.emit("accepting_cookies", "Aceptando cookies...".into());
    }

    fn searching_coincidences(&self) {
        self.emit(
            "searching_coincidences",
            "Buscando coincidencias...".into(),
        );
    }

    fn found_single_coincidence(&self) {
        self.emit("found_single_coincidence", "Coincidencia única encontrada".into());
    }

    fn found_multiple_coincidences(&self) {
        self.emit(
            "found_multiple_coincidences",
            "Múltiples coincidencias encontradas".into(),
        );
    }

    fn processed_coincidence(&self, name: &str, count: usize) {
        self.emit(
            "processed_coincidence",
            format!("Procesado {name} ({count} items)"),
        );
    }

    fn writing_coincidences(&self, output: &[Coincidence]) {
        self.emit(
            "writing_coincidences",
            format!("Escribiendo {} coincidencias en la base de datos...", output.len()),
        );
    }

    fn written_coincidences(&self, count: i32) {
        self.emit(
            "written_coincidences",
            format!("{count} coincidencias escritas en la base de datos."),
        );
    }


    fn vpn_rotating(&self) {
        self.emit("vpn_rotating", "Rotando IP a través de NordVPN...".into());
    }

    fn vpn_rotated(&self) {
        self.emit("vpn_rotated", "IP rotada correctamente.".into());
    }

    fn vpn_not_available(&self) {
        self.emit("warn", "NordVPN no disponible, continuando sin rotación de IP.".into());
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    fn warn(&self, msg: &str) {
        self.emit("warn", format!("Advertencia: {msg}"));
    }

    fn error(&self, err: &str) {
        self.emit("error", format!("Error: {err}"));
    }
}
