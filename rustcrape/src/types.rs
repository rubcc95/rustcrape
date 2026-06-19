use std::num::NonZeroU32;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod zero_is_none {
    use std::num::NonZeroU32;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<NonZeroU32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(value.map(NonZeroU32::get).unwrap_or(0))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NonZeroU32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Ok(NonZeroU32::new(value))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coincidence {
    pub name: String,
    pub email: Option<String>,
    pub web: Option<String>,
    pub tfno: Option<String>,
    pub maps: String,
}
 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub search: SearchConfig,
    #[serde(with = "zero_is_none")]
    pub rate_limit: Option<NonZeroU32>,
    #[serde(with = "zero_is_none")]
    pub iterations: Option<NonZeroU32>,
    pub db: DbConfig,
    pub nordvpn_path: Option<String>,
    pub browser_path: Option<String>,
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

#[derive(Debug, Clone)]
pub enum DbConfig {
    Sqlite {
        path: Option<String>,
    },
    Mysql {
        host: String,
        port: u16,
        user: String,
        password: String,
        database: String,
    },
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig::Sqlite { path: None }
    }
}

impl Serialize for DbConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        #[derive(Serialize)]
        struct SqliteView {
            r#type: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            path: Option<String>,
        }
        #[derive(Serialize)]
        struct MysqlView {
            r#type: &'static str,
            host: String,
            port: u16,
            user: String,
            password: String,
            database: String,
        }
        match self {
            DbConfig::Sqlite { path } => SqliteView { r#type: "sqlite", path: path.clone() }.serialize(serializer),
            DbConfig::Mysql { host, port, user, password, database } => MysqlView {
                r#type: "mysql",
                host: host.clone(),
                port: *port,
                user: user.clone(),
                password: password.clone(),
                database: database.clone(),
            }.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for DbConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use serde::de;

        let value = serde_json::Value::deserialize(deserializer)?;

        // Try tagged format first: { "type": "sqlite" | "mysql", ... }
        if let Some(tag) = value.get("type").and_then(|t| t.as_str()) {
            return match tag {
                "sqlite" => Ok(DbConfig::Sqlite {
                    path: value.get("path").and_then(|v| v.as_str().map(|s| s.to_string())),
                }),
                "mysql" => {
                    let host = value.get("host").and_then(|v| v.as_str()).ok_or_else(|| de::Error::missing_field("host"))?.to_string();
                    let port = value.get("port").and_then(|v| v.as_u64()).ok_or_else(|| de::Error::missing_field("port"))? as u16;
                    let user = value.get("user").and_then(|v| v.as_str()).ok_or_else(|| de::Error::missing_field("user"))?.to_string();
                    let password = value.get("password").and_then(|v| v.as_str()).ok_or_else(|| de::Error::missing_field("password"))?.to_string();
                    let database = value.get("database").and_then(|v| v.as_str()).ok_or_else(|| de::Error::missing_field("database"))?.to_string();
                    Ok(DbConfig::Mysql { host, port, user, password, database })
                }
                other => Err(de::Error::unknown_variant(other, &["sqlite", "mysql"])),
            };
        }

        // Fallback: old flat format → Mysql
        let host = value.get("host").and_then(|v| v.as_str()).ok_or_else(|| {
            de::Error::custom("missing 'type' field; expected {\"type\":\"sqlite\"} or {\"type\":\"mysql\",...}")
        })?.to_string();
        let port = value.get("port").and_then(|v| v.as_u64()).unwrap_or(3306) as u16;
        let user = value.get("user").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let password = value.get("password").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let database = value.get("database").and_then(|v| v.as_str()).unwrap_or("").to_string();
        Ok(DbConfig::Mysql { host, port, user, password, database })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IterationStats {
    pub encountered: u64,
    pub inserted: u64,
    pub duplicated: u64,
    pub empty: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectStats {
    pub bounds_total: i64,
    pub bounds_processed: i64,
    pub bounds_remaining: i64,
    pub results_found: i64,
    pub phones_found: i64,
}
