use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tintoreria {
    pub nombre: String,
    pub email: Option<String>,
    pub web: Option<String>,
    pub tfno: Option<String>,
    pub maps_url: String,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub lat: f64,
    pub lng: f64,
    pub zoom: u32,
    pub stop_threshold: u32,
    pub search_query: String,
    pub delay_min: u64,
    pub delay_max: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            lat: 40.4168,
            lng: -3.7038,
            zoom: 12,
            stop_threshold: 3,
            search_query: String::new(),
            delay_min: 500,
            delay_max: 2000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone, Default)]
pub struct IterationStats {
    pub encontrados: u64,
    pub insertados: u64,
    pub duplicados: u64,
    pub sin_datos: u64,
}
