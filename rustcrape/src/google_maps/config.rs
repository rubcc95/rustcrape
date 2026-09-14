use serde::{Deserialize, Serialize};

/// Parametros de una tarea de Google Maps: el centro de un bound.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GoogleMapsParams {
    pub lat: f32,
    pub lng: f32,
}
