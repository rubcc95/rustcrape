use serde::Serialize;

/// Parametros de una tarea de Google Maps: el centro de un bound.
#[derive(Debug, Copy, Clone, Serialize)]
pub struct GMapsParams {
    pub lat: f32,
    pub lng: f32,
    // pub headless: bool,
    // pub browser_path: Option<&'a Path>,
    // pub profile_dir: Option<&'a Path>,
}
