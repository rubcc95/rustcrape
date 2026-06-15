use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coincidence {
    pub name: String,
    pub email: Option<String>,
    pub web: Option<String>,
    pub tfno: Option<String>,
    pub maps_url: String,
}
 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub search: SearchConfig,
    pub rate_limit: u32,
    pub iterations: u32,
    pub db: DbConfig,
    pub nordvpn_path: Option<String>,
    pub ip_rotation_frequency: u32,
}

impl std::ops::Deref for Config {
    type Target = SearchConfig;

    fn deref(&self) -> &Self::Target {
        &self.search
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub persistent: PersistentConfig,
    pub stop_threshold: u32,
    pub delay_min: u64,
    pub delay_max: u64,
    pub headless: bool,
}

impl std::ops::Deref for SearchConfig{
    type Target = PersistentConfig;

    fn deref(&self) -> &Self::Target {
        &self.persistent
    }
}

impl std::ops::DerefMut for SearchConfig{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.persistent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentConfig{
    pub zoom: u32,
    pub search_query: String,
}

#[derive(Debug, Copy, Clone)]
pub struct SearchContext<'a> {
    pub lat: f32,
    pub lng: f32,
    pub config: &'a SearchConfig,
}

impl std::ops::Deref for SearchContext<'_> {
    type Target = SearchConfig;

    fn deref(&self) -> &Self::Target {
        self.config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IterationStats {
    pub encountered: u64,
    pub inserted: u64,
    pub duplicated: u64,
    pub empty: u64,
}
