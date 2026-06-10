use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;   
 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbType {
    Mysql,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub search_term: String,
    pub zoom: u8,
    pub db_type: DbType,
    pub mysql_host: String,
    pub mysql_port: u16,
    pub mysql_user: String,
    pub mysql_password: String,
    pub mysql_database: String,
    pub sqlite_path: String,
    pub browser_path: Option<String>,
    pub user_agents: Vec<String>,
    pub iterations: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            search_term: String::new(),
            zoom: 12,
            db_type: DbType::Sqlite,
            mysql_host: "localhost".into(),
            mysql_port: 3306,
            mysql_user: "root".into(),
            mysql_password: String::new(),
            mysql_database: "scrapper".into(),
            sqlite_path: "data/scrapper.db".into(),
            browser_path: None,
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".into(),
            ],
            iterations: 0,
        }
    }
}

pub fn config_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_default();
    path.set_file_name("config.json");
    path
}

pub fn load() -> AppConfig {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}
