use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rustcrape::types::Coincidence;
use serde::Serialize;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
pub struct VerboserPayload {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted_with_phone: Option<u64>,
}

pub struct TauriVerboser {
    app_handle: tauri::AppHandle,
    cancel: CancellationToken,
    log_file: Option<Mutex<File>>,
}

impl TauriVerboser {
    pub fn new(
        app_handle: tauri::AppHandle,
        cancel: CancellationToken,
        log_dir: PathBuf,
        db_label: &str,
    ) -> Self {
        let log_file = Self::open_log(&log_dir, db_label);
        Self {
            app_handle,
            cancel,
            log_file,
        }
    }

    /// Abre el archivo de logs de la ejecución. Un fallo al abrirlo no debe
    /// interrumpir el scraping, por eso se degrada a `None` avisando por stderr.
    fn open_log(log_dir: &Path, db_label: &str) -> Option<Mutex<File>> {
        if let Err(err) = std::fs::create_dir_all(log_dir) {
            eprintln!(
                "No se pudo crear el directorio de logs {}: {err}",
                log_dir.display()
            );
            return None;
        }
        let label = sanitize_label(db_label);
        let path = log_dir.join(format!("{label}_{}.log", now_stamp()));
        match OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&path)
        {
            Ok(file) => Some(Mutex::new(file)),
            Err(err) => {
                eprintln!(
                    "No se pudo abrir el archivo de logs {}: {err}",
                    path.display()
                );
                None
            }
        }
    }

    fn emit(&self, kind: &str, message: String) {
        self.emit_with(kind, message, None, None);
    }

    fn emit_with(
        &self,
        kind: &str,
        message: String,
        inserted: Option<u64>,
        inserted_with_phone: Option<u64>,
    ) {
        self.write_log(kind, &message, inserted, inserted_with_phone);
        self.emit_event(kind, message, inserted, inserted_with_phone);
    }

    fn emit_event(
        &self,
        kind: &str,
        message: String,
        inserted: Option<u64>,
        inserted_with_phone: Option<u64>,
    ) {
        let _ = self.app_handle.emit(
            "verboser-event",
            VerboserPayload {
                kind: kind.to_string(),
                message,
                inserted,
                inserted_with_phone,
            },
        );
    }

    /// Escribe el payload en el archivo de logs. Se registran todos los
    /// payloads, incluidos los de debug, tanto en modo debug como release.
    fn write_log(
        &self,
        kind: &str,
        message: &str,
        inserted: Option<u64>,
        inserted_with_phone: Option<u64>,
    ) {
        let Some(file) = &self.log_file else {
            return;
        };
        let Ok(mut file) = file.lock() else {
            return;
        };
        let mut line = format!("[{}] [{}] {}", now_datetime(), kind, message);
        match (inserted, inserted_with_phone) {
            (Some(inserted), Some(phone)) => {
                line.push_str(&format!(
                    " (inserted={inserted}, inserted_with_phone={phone})"
                ));
            }
            (Some(inserted), None) => line.push_str(&format!(" (inserted={inserted})")),
            (None, Some(phone)) => {
                line.push_str(&format!(" (inserted_with_phone={phone})"));
            }
            (None, None) => {}
        }
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
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
        self.emit("searching_coincidences", "Buscando coincidencias...".into());
    }

    fn found_coincidences(&self, count: Option<usize>) {
        self.emit(
            "found_coincidences",
            match count {
                Some(c) => format!("Coincidencias encontradas: ({c})"),
                None => "Coincidencias encontradas".into(),
            },
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
            format!(
                "Escribiendo {} coincidencias en la base de datos...",
                output.len()
            ),
        );
    }

    fn written_coincidences(&self, inserted: u64, inserted_with_phone: u64) {
        self.emit_with(
            "written_coincidences",
            format!("{inserted} coincidencias escritas en la base de datos."),
            Some(inserted),
            Some(inserted_with_phone),
        );
    }

    fn vpn_rotating(&self) {
        self.emit("vpn_rotating", "Rotando IP a través de NordVPN...".into());
    }

    fn vpn_rotated(&self) {
        self.emit("vpn_rotated", "IP rotada correctamente.".into());
    }

    fn vpn_not_available(&self) {
        self.emit(
            "warn",
            "NordVPN no disponible, continuando sin rotación de IP.".into(),
        );
    }

    fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    fn cancellation(&self) -> Option<CancellationToken> {
        Some(self.cancel.clone())
    }

    fn warn(&self, msg: &str) {
        self.emit("warn", format!("Advertencia: {msg}"));
    }

    fn error(&self, err: &str) {
        self.emit("error", format!("Error: {err}"));
    }

    fn debug(&self, msg: &str) {
        let kind = "verbose";
        let message = format!("Verbose: {msg}");
        // El archivo de logs registra el payload siempre; la GUI solo lo
        // muestra en modo debug.
        self.write_log(kind, &message, None, None);
        if cfg!(debug_assertions) {
            self.emit_event(kind, message, None, None);
        }
    }

    /// Devuelve `true` siempre: aunque en release la GUI no reciba los
    /// mensajes de debug, el archivo de logs sí debe registrarlos.
    fn debug_enabled(&self) -> bool {
        true
    }
}

/// Normaliza la etiqueta usada en el nombre del archivo de logs.
fn sanitize_label(label: &str) -> String {
    let sanitized: String = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if sanitized.is_empty() {
        "rustcrape".to_string()
    } else {
        sanitized
    }
}

/// Descompone el instante actual (UTC) en (año, mes, día, hora, minuto, segundo).
fn now_utc() -> (i64, i64, i64, i64, i64, i64) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86400) as i64;
    let time_secs = (secs % 86400) as i64;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs_part = time_secs % 60;

    let mut y = 1970i64;
    let mut remaining_days = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }
    let month_days: [i64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1i64;
    let mut d = remaining_days;
    for (i, &md) in month_days.iter().enumerate() {
        if d < md {
            m = (i + 1) as i64;
            d += 1;
            break;
        }
        d -= md;
    }
    (y, m, d, hours, mins, secs_part)
}

/// Marca de tiempo compacta para el nombre del archivo: `YYYYMMDD-HHMMSS`.
fn now_stamp() -> String {
    let (y, m, d, h, min, s) = now_utc();
    format!("{y:04}{m:02}{d:02}-{h:02}{min:02}{s:02}")
}

/// Marca de tiempo legible por línea: `YYYY-MM-DD HH:MM:SS`.
fn now_datetime() -> String {
    let (y, m, d, h, min, s) = now_utc();
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}:{s:02}")
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
