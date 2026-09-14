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

/// Resultado unico de un scrape, comun a todos los targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coincidence {
    pub name: String,
    pub email: Option<String>,
    pub web: Option<String>,
    pub tfno: Option<String>,
    /// URL de origen (ficha de Google Maps, pagina de Empresite, etc.).
    pub source_url: String,
}

/// Modo de ejecucion cuando hay varios targets habilitados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    #[default]
    Sequential,
    Parallel,
}

/// Configuracion global del scraper. Contiene la configuracion de cada target
/// y los ajustes comunes a toda la ejecucion.
#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub google_maps: GoogleMapsConfig,
    pub empresite: EmpresiteConfig,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    pub db: DbConfig,
    /// VPN global para todos los targets.
    pub nordvpn_path: Option<String>,
    pub browser_path: Option<String>,
    /// Directorio raiz de perfiles persistentes de Chrome. Si es `None` se usa
    /// un directorio temporal. Conserva cookies (reCAPTCHA/consentimiento)
    /// entre tareas para reducir captchas.
    pub browser_profile_dir: Option<String>,
    pub ip_rotation_frequency: u32,
}

#[derive(Deserialize)]
struct ConfigRaw {
    google_maps: GoogleMapsConfig,
    empresite: EmpresiteConfig,
    #[serde(default)]
    execution_mode: ExecutionMode,
    db: DbConfig,
    #[serde(default)]
    nordvpn_path: Option<String>,
    #[serde(default)]
    browser_path: Option<String>,
    #[serde(default)]
    browser_profile_dir: Option<String>,
    #[serde(default)]
    ip_rotation_frequency: u32,
}

impl From<ConfigRaw> for Config {
    fn from(raw: ConfigRaw) -> Self {
        Config {
            google_maps: raw.google_maps,
            empresite: raw.empresite,
            execution_mode: raw.execution_mode,
            db: raw.db,
            nordvpn_path: raw.nordvpn_path,
            browser_path: raw.browser_path,
            browser_profile_dir: raw.browser_profile_dir,
            ip_rotation_frequency: raw.ip_rotation_frequency,
        }
    }
}

// Formato antiguo: toda la configuracion bajo `search` (solo existia Google Maps).
#[derive(Deserialize)]
struct LegacyPersistentConfig {
    zoom: u32,
    search_query: String,
}

#[derive(Deserialize)]
struct LegacySearchConfig {
    persistent: LegacyPersistentConfig,
    stop_threshold: u32,
    delay_min: u64,
    delay_max: u64,
    headless: bool,
}

#[derive(Deserialize)]
struct LegacyConfig {
    search: LegacySearchConfig,
    #[serde(default)]
    rate_limit: u32,
    #[serde(default)]
    iterations: u32,
    db: DbConfig,
    #[serde(default)]
    nordvpn_path: Option<String>,
    #[serde(default)]
    browser_path: Option<String>,
    #[serde(default)]
    browser_profile_dir: Option<String>,
    #[serde(default)]
    ip_rotation_frequency: u32,
}

impl From<LegacyConfig> for Config {
    fn from(legacy: LegacyConfig) -> Self {
        let search = legacy.search;
        let search_query = search.persistent.search_query;
        Config {
            google_maps: GoogleMapsConfig {
                enabled: true,
                search_query: search_query.clone(),
                zoom: search.persistent.zoom,
                stop_threshold: search.stop_threshold,
                delay_min: search.delay_min,
                delay_max: search.delay_max,
                headless: search.headless,
                rate_limit: NonZeroU32::new(legacy.rate_limit),
                iterations: NonZeroU32::new(legacy.iterations),
            },
            empresite: EmpresiteConfig {
                enabled: false,
                search_query,
                delay_min: 500,
                delay_max: 2000,
                headless: false,
                rate_limit: None,
                iterations: None,
            },
            execution_mode: ExecutionMode::Sequential,
            db: legacy.db,
            nordvpn_path: legacy.nordvpn_path,
            browser_path: legacy.browser_path,
            browser_profile_dir: legacy.browser_profile_dir,
            ip_rotation_frequency: legacy.ip_rotation_frequency,
        }
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.get("google_maps").is_some() {
            ConfigRaw::deserialize(value)
                .map(Config::from)
                .map_err(serde::de::Error::custom)
        } else {
            LegacyConfig::deserialize(value)
                .map(Config::from)
                .map_err(serde::de::Error::custom)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleMapsConfig {
    pub enabled: bool,
    pub search_query: String,
    pub zoom: u32,
    pub stop_threshold: u32,
    pub delay_min: u64,
    pub delay_max: u64,
    pub headless: bool,
    /// Limite de ejecuciones por hora propio de este target.
    #[serde(with = "zero_is_none")]
    pub rate_limit: Option<NonZeroU32>,
    /// Numero maximo de tareas a procesar (para pruebas).
    #[serde(with = "zero_is_none")]
    pub iterations: Option<NonZeroU32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmpresiteConfig {
    pub enabled: bool,
    /// Termino de busqueda (se comparte con Google Maps).
    pub search_query: String,
    pub delay_min: u64,
    pub delay_max: u64,
    pub headless: bool,
    /// Limite de ejecuciones por hora propio de este target.
    #[serde(with = "zero_is_none")]
    pub rate_limit: Option<NonZeroU32>,
    /// Numero maximo de tareas a procesar (para pruebas).
    #[serde(with = "zero_is_none")]
    pub iterations: Option<NonZeroU32>,
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
