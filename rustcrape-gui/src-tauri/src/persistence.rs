use std::path::PathBuf;

use rustcrape::types::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    pub id: String,
    pub name: String,
    pub config: Config,
    pub started: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Metadata {
    last_selected: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSettings {
    pub nordvpn_path: Option<String>,
}

pub struct ConfigStore {
    configs_dir: PathBuf,
    metadata_path: PathBuf,
    settings_path: PathBuf,
}

impl ConfigStore {
    pub fn new(app_data_dir: PathBuf) -> std::io::Result<Self> {
        let configs_dir = app_data_dir.join("configs");
        let metadata_path = app_data_dir.join("metadata.json");
        let settings_path = app_data_dir.join("settings.json");
        std::fs::create_dir_all(&configs_dir)?;
        Ok(Self {
            configs_dir,
            metadata_path,
            settings_path,
        })
    }

    pub fn load_global_settings(&self) -> std::io::Result<GlobalSettings> {
        if self.settings_path.exists() {
            let content = std::fs::read_to_string(&self.settings_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(GlobalSettings { nordvpn_path: None })
        }
    }

    pub fn save_global_settings(&self, settings: &GlobalSettings) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        std::fs::write(&self.settings_path, content)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<SavedConfig> {
        let mut configs = Vec::new();
        let entries = match std::fs::read_dir(&self.configs_dir) {
            Ok(e) => e,
            Err(_) => return configs,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(sc) = serde_json::from_str::<SavedConfigFields>(&content) {
                        configs.push(SavedConfig {
                            id,
                            name: sc.name,
                            config: sc.config,
                            started: sc.started,
                        });
                    }
                }
            }
        }
        configs.sort_by(|a, b| a.name.cmp(&b.name));
        configs
    }

    pub fn save(&self, name: &str, config: Config, started: bool) -> std::io::Result<SavedConfig> {
        // Reuse existing file if a config with this name already exists
        let id = self.list().into_iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .unwrap_or_else(|| generate_id(name));
        let path = self.configs_dir.join(format!("{id}.json"));
        let content = serde_json::to_string_pretty(&SavedConfigFields {
            name: name.to_string(),
            config: config.clone(),
            started,
        })?;
        std::fs::write(&path, content)?;
        Ok(SavedConfig {
            id,
            name: name.to_string(),
            config,
            started,
        })
    }

    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        let path = self.configs_dir.join(format!("{id}.json"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn get_last_selected(&self) -> Option<SavedConfig> {
        let content = std::fs::read_to_string(&self.metadata_path).ok()?;
        let meta: Metadata = serde_json::from_str(&content).ok()?;
        let id = meta.last_selected?;
        self.get(&id)
    }

    pub fn set_last_selected(&self, id: Option<&str>) -> std::io::Result<()> {
        let meta = Metadata {
            last_selected: id.map(|s| s.to_string()),
        };
        let content = serde_json::to_string_pretty(&meta)?;
        std::fs::write(&self.metadata_path, content)?;
        Ok(())
    }

    fn get(&self, id: &str) -> Option<SavedConfig> {
        let path = self.configs_dir.join(format!("{id}.json"));
        let content = std::fs::read_to_string(path).ok()?;
        let sc: SavedConfigFields = serde_json::from_str(&content).ok()?;
        Some(SavedConfig {
            id: id.to_string(),
            name: sc.name,
            config: sc.config,
            started: sc.started,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedConfigFields {
    name: String,
    config: Config,
    #[serde(default)]
    started: bool,
}

fn generate_id(name: &str) -> String {
    let sanitized: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    let ts = {
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let hours = time_secs / 3600;
        let mins = (time_secs % 3600) / 60;
        let secs_part = time_secs % 60;

        let mut y = 1970i64;
        let mut remaining_days = days as i64;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            y += 1;
        }
        let month_days = if is_leap(y) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut m = 0usize;
        let mut d = remaining_days;
        for (i, &md) in month_days.iter().enumerate() {
            if d < md {
                m = i + 1;
                d += 1;
                break;
            }
            d -= md;
        }

        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            y, m, d, hours, mins, secs_part
        )
    };

    if sanitized.is_empty() {
        format!("config-{ts}")
    } else {
        format!("{sanitized}-{ts}")
    }
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
