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
///
/// Los campos marcados como `Option` no siempre estan disponibles: Google Maps
/// solo rellena `name`, `email`, `web`, `tfno` y `source_url`, mientras que
/// Empresite anade ademas los datos mercantiles (razon social, CIF, forma
/// juridica, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Coincidence {
    pub name: String,
    pub email: Option<String>,
    pub web: Option<String>,
    pub tfno: Option<String>,
    /// URL de origen (ficha de Google Maps, pagina de Empresite, etc.).
    pub source_url: String,
    /// Razon social (Empresite).
    pub legal_name: Option<String>,
    /// CIF / NIF (Empresite).
    pub tax_id: Option<String>,
    /// Forma juridica (Empresite).
    pub legal_form: Option<String>,
    /// Sector (Empresite).
    pub sector: Option<String>,
    /// Fecha de constitucion (Empresite).
    pub incorporation_date: Option<String>,
    /// Fecha del ultimo cambio en el registro (Empresite).
    pub last_change_date: Option<String>,
    /// Objeto social (Empresite).
    pub corporate_purpose: Option<String>,
    /// Actividad declarada (Empresite).
    pub activity: Option<String>,
    /// Actividad CNAE (Empresite).
    pub cnae_activity: Option<String>,
    /// Estado de la empresa (Empresite).
    pub company_status: Option<String>,
}

impl Coincidence {
    /// Indica si la coincidencia aporta algun dato util (contacto o datos
    /// mercantiles). Se usa para descartar filas vacias antes de persistir.
    pub fn has_any_data(&self) -> bool {
        self.web.is_some()
            || self.email.is_some()
            || self.tfno.is_some()
            || self.legal_name.is_some()
            || self.tax_id.is_some()
            || self.legal_form.is_some()
            || self.sector.is_some()
            || self.incorporation_date.is_some()
            || self.last_change_date.is_some()
            || self.corporate_purpose.is_some()
            || self.activity.is_some()
            || self.cnae_activity.is_some()
            || self.company_status.is_some()
    }
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
    pub gmaps: GMapsConfig,
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
    gmaps: GMapsConfig,
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
            gmaps: raw.gmaps,
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
            gmaps: GMapsConfig {
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
                ..Default::default()
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
        if value.get("gmaps").is_some() {
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
pub struct GMapsConfig {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    // --- Filtros del listado (`Incluyen siguientes datos`) ---
    /// Empresas que incluyen web.
    #[serde(default)]
    pub web: bool,
    /// Empresas que incluyen telefono.
    #[serde(default)]
    pub phone: bool,
    /// Empresas que incluyen email.
    #[serde(default)]
    pub email: bool,
    /// Empresas con ubicacion registrada.
    #[serde(default)]
    pub location: bool,
    /// Empresas con sucursales.
    #[serde(default)]
    pub branch: bool,
    /// Tamano por facturacion.
    pub company_size: Option<CompanySize>,
    /// Rango de numero de empleados.
    pub employees: Option<EmployeeRange>,
    /// Fecha de constitucion.
    pub incorporation_date: Option<IncorporationDate>,
    /// Forma juridica.
    pub legal_form: Option<LegalForm>,
    /// Filtro geografico (opcional): provincia o localidad, mutuamente
    /// excluyentes. Viaja como segmento de path en la URL:
    /// `/provincia/{PROV}/` o `/localidad/{PUEBLO-PROV}/`.
    pub location_filter: Option<EmpresiteLocation>,
}

/// Filtro de ubicacion del listado de Empresite. Provincia y localidad son
/// excluyentes: buscar por una descarta la otra.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmpresiteLocation {
    /// Filtra por toda la provincia.
    Province(Province),
    /// Filtra por una localidad concreta dentro de una provincia. Empresite
    /// espera el slug `PUEBLO-PROVINCIA`, salvo cuando la localidad se llama
    /// igual que su provincia, en cuyo caso va sola (`BARCELONA`).
    Locality { name: String, province: Province },
}

impl EmpresiteLocation {
    /// Slug que Empresite espera para el segmento de localidad:
    /// `PUEBLO-PROVINCIA` o solo `PUEBLO` si coincide con la provincia.
    pub fn locality_slug(name: &str, province: Province) -> String {
        let name_slug = locality_name_slug(name);
        let province_slug = province.query_value();
        if name_slug == province_slug {
            name_slug
        } else {
            format!("{name_slug}-{province_slug}")
        }
    }
}

/// Normaliza el nombre de una localidad a mayusculas, sin acentos, con los
/// espacios convertidos en guiones y descartando articulos y adverbios
/// (`San Mateo de Gállego` -> `SAN-MATEO-GALLEGO`). Se reutiliza la misma
/// normalizacion que para el activity, anadiendo el filtrado de palabras vacias.
fn locality_name_slug(name: &str) -> String {
    const STOP_WORDS: &[&str] = &[
        "DE", "DEL", "LA", "LAS", "EL", "LOS", "Y", "E", "O", "U", "A", "EN", "AL",
    ];
    name.split_whitespace()
        .map(crate::empresite::config::activity_slug)
        .filter(|word| !word.is_empty() && !STOP_WORDS.contains(&word.as_str()))
        .collect::<Vec<_>>()
        .join("-")
}

impl EmpresiteConfig {
    /// Construye la cadena de filtros activos lista para la URL (sin el
    /// prefijo `?`). Devuelve cadena vacia si no hay ningun filtro.
    pub fn filter_query(&self) -> String {
        let mut params: Vec<String> = Vec::new();

        if self.web {
            params.push("emp_web=true".to_string());
        }
        if self.phone {
            params.push("emp_telefono=true".to_string());
        }
        if self.email {
            params.push("emp_email=true".to_string());
        }
        if self.location {
            params.push("municipio=true".to_string());
        }
        if self.branch {
            params.push("numSucursales=true".to_string());
        }
        if let Some(size) = self.company_size {
            params.push(format!("emp_ventas_number={}", size.query_value()));
        }
        if let Some(employees) = self.employees {
            params.push(format!("emp_empleados_number={}", employees.query_value()));
        }
        if let Some(date) = self.incorporation_date {
            params.push(format!("fecha_constitucion={}", date.query_value()));
        }
        if let Some(form) = self.legal_form {
            params.push(format!("emp_formajuridica={}", form.query_value()));
        }

        params.join("&")
    }
}

/// Tamano de empresa por facturacion (filtro del listado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanySize {
    Small,
    Medium,
    Large,
    Corporate,
}

impl CompanySize {
    /// Valor del parametro `emp_ventas_number`.
    pub fn query_value(self) -> &'static str {
        match self {
            CompanySize::Small => "pequenas",
            CompanySize::Medium => "medianas",
            CompanySize::Large => "grandes",
            CompanySize::Corporate => "corporativas",
        }
    }
}

/// Fecha de constitucion de la empresa (filtro del listado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncorporationDate {
    LastMonth,
    LastThreeMonths,
    LastYear,
    MoreThanAYear,
}

impl IncorporationDate {
    /// Valor del parametro `fecha_constitucion`.
    pub fn query_value(self) -> &'static str {
        match self {
            IncorporationDate::LastMonth => "1m",
            IncorporationDate::LastThreeMonths => "3m",
            IncorporationDate::LastYear => "1a",
            IncorporationDate::MoreThanAYear => "1adesde",
        }
    }
}

/// Forma juridica de la empresa (filtro del listado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalForm {
    LimitedLiabilityCompany,
    CommunityOfProperty,
    CivilPartnership,
    PublicLimitedCompany,
    TemporaryJointVenture,
    Cooperative,
    PublicBody,
    LocalCorporation,
    PublicAdministration,
    ForeignEntity,
}

impl LegalForm {
    /// Valor del parametro `emp_formajuridica`.
    pub fn query_value(self) -> &'static str {
        match self {
            LegalForm::LimitedLiabilityCompany => "B",
            LegalForm::CommunityOfProperty => "E",
            LegalForm::CivilPartnership => "J",
            LegalForm::PublicLimitedCompany => "A",
            LegalForm::TemporaryJointVenture => "U",
            LegalForm::Cooperative => "F",
            LegalForm::PublicBody => "Q",
            LegalForm::LocalCorporation => "P",
            LegalForm::PublicAdministration => "S",
            LegalForm::ForeignEntity => "N",
        }
    }
}

/// Rango de numero de empleados (filtro del listado).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmployeeRange {
    pub min: u32,
    pub max: u32,
}

impl EmployeeRange {
    /// Valor del parametro `emp_empleados_number`: `min-max`, o solo `min`
    /// cuando ambos extremos coinciden.
    pub fn query_value(self) -> String {
        if self.min == self.max {
            self.min.to_string()
        } else {
            format!("{}-{}", self.min, self.max)
        }
    }
}

/// Provincia espanola por la que filtrar el listado de Empresite. El slug que
/// espera la web no siempre coincide con el nombre oficial (p. ej. `La Rioja`
/// es `RIOJA` o `A Coruna` es `CORUNA`), por eso cada variante lo fija.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Province {
    Alava,
    Albacete,
    Alicante,
    Almeria,
    Asturias,
    Avila,
    Badajoz,
    Baleares,
    Barcelona,
    Burgos,
    Caceres,
    Cadiz,
    Cantabria,
    Castellon,
    Ceuta,
    CiudadReal,
    Cordoba,
    Coruna,
    Cuenca,
    Gerona,
    Granada,
    Guadalajara,
    Guipuzcoa,
    Huelva,
    Huesca,
    Jaen,
    Leon,
    Lerida,
    Lugo,
    Madrid,
    Malaga,
    Melilla,
    Murcia,
    Navarra,
    Orense,
    Palencia,
    Palmas,
    Pontevedra,
    Rioja,
    Salamanca,
    SantaCruzDeTenerife,
    Segovia,
    Sevilla,
    Soria,
    Tarragona,
    Teruel,
    Toledo,
    Valencia,
    Valladolid,
    Vizcaya,
    Zamora,
    Zaragoza,
}

impl Province {
    /// Slug de provincia tal como lo usa Empresite en la URL.
    pub fn query_value(self) -> &'static str {
        match self {
            Province::Alava => "ALAVA",
            Province::Albacete => "ALBACETE",
            Province::Alicante => "ALICANTE",
            Province::Almeria => "ALMERIA",
            Province::Asturias => "ASTURIAS",
            Province::Avila => "AVILA",
            Province::Badajoz => "BADAJOZ",
            Province::Baleares => "BALEARES",
            Province::Barcelona => "BARCELONA",
            Province::Burgos => "BURGOS",
            Province::Caceres => "CACERES",
            Province::Cadiz => "CADIZ",
            Province::Cantabria => "CANTABRIA",
            Province::Castellon => "CASTELLON",
            Province::Ceuta => "CEUTA",
            Province::CiudadReal => "CIUDAD-REAL",
            Province::Cordoba => "CORDOBA",
            Province::Coruna => "CORUNA",
            Province::Cuenca => "CUENCA",
            Province::Gerona => "GERONA",
            Province::Granada => "GRANADA",
            Province::Guadalajara => "GUADALAJARA",
            Province::Guipuzcoa => "GUIPUZCOA",
            Province::Huelva => "HUELVA",
            Province::Huesca => "HUESCA",
            Province::Jaen => "JAEN",
            Province::Leon => "LEON",
            Province::Lerida => "LERIDA",
            Province::Lugo => "LUGO",
            Province::Madrid => "MADRID",
            Province::Malaga => "MALAGA",
            Province::Melilla => "MELILLA",
            Province::Murcia => "MURCIA",
            Province::Navarra => "NAVARRA",
            Province::Orense => "ORENSE",
            Province::Palencia => "PALENCIA",
            Province::Palmas => "PALMAS",
            Province::Pontevedra => "PONTEVEDRA",
            Province::Rioja => "RIOJA",
            Province::Salamanca => "SALAMANCA",
            Province::SantaCruzDeTenerife => "SANTA-CRUZ-TENERIFE",
            Province::Segovia => "SEGOVIA",
            Province::Sevilla => "SEVILLA",
            Province::Soria => "SORIA",
            Province::Tarragona => "TARRAGONA",
            Province::Teruel => "TERUEL",
            Province::Toledo => "TOLEDO",
            Province::Valencia => "VALENCIA",
            Province::Valladolid => "VALLADOLID",
            Province::Vizcaya => "VIZCAYA",
            Province::Zamora => "ZAMORA",
            Province::Zaragoza => "ZARAGOZA",
        }
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

/// Estado agregado de la base de datos de un proyecto: tareas completadas y
/// pendientes (bounds de Google Maps + paginas de Empresite), resultados
/// almacenados y resultados con telefono.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectStats {
    pub bounds_total: u64,
    pub bounds_processed: u64,
    pub bounds_remaining: u64,
    pub results_found: u64,
    pub phones_found: u64,
}

/// Columna de la tabla `coincidences` por la que ordenar. Los nombres en
/// `snake_case` coinciden con las columnas fisicas de la base de datos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoincidenceColumn {
    Id,
    Name,
    Web,
    Email,
    Tfno,
    SourceUrl,
    Source,
    Creado,
    LegalName,
    TaxId,
    LegalForm,
    Sector,
    IncorporationDate,
    LastChangeDate,
    CorporatePurpose,
    Activity,
    CnaeActivity,
    CompanyStatus,
}

impl CoincidenceColumn {
    /// Nombre real de la columna en la base de datos. Se usa como whitelist
    /// para construir la clausula `ORDER BY` sin riesgo de inyeccion SQL.
    pub fn column_name(self) -> &'static str {
        match self {
            CoincidenceColumn::Id => "id",
            CoincidenceColumn::Name => "name",
            CoincidenceColumn::Web => "web",
            CoincidenceColumn::Email => "email",
            CoincidenceColumn::Tfno => "tfno",
            CoincidenceColumn::SourceUrl => "source_url",
            CoincidenceColumn::Source => "source",
            CoincidenceColumn::Creado => "creado",
            CoincidenceColumn::LegalName => "legal_name",
            CoincidenceColumn::TaxId => "tax_id",
            CoincidenceColumn::LegalForm => "legal_form",
            CoincidenceColumn::Sector => "sector",
            CoincidenceColumn::IncorporationDate => "incorporation_date",
            CoincidenceColumn::LastChangeDate => "last_change_date",
            CoincidenceColumn::CorporatePurpose => "corporate_purpose",
            CoincidenceColumn::Activity => "activity",
            CoincidenceColumn::CnaeActivity => "cnae_activity",
            CoincidenceColumn::CompanyStatus => "company_status",
        }
    }
}

/// Direccion de ordenacion de una consulta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn sql(self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

/// Fila completa de la tabla `coincidences`, tal como se muestra en la GUI.
/// Amplia `Coincidence` con los metadatos de persistencia (`id`, `source` y
/// `creado`) y con todos los campos mercantiles rellenos o vacios.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoincidenceRecord {
    pub id: i64,
    pub name: String,
    pub web: String,
    pub email: String,
    pub tfno: String,
    pub source_url: String,
    pub source: String,
    pub creado: Option<String>,
    pub legal_name: Option<String>,
    pub tax_id: Option<String>,
    pub legal_form: Option<String>,
    pub sector: Option<String>,
    pub incorporation_date: Option<String>,
    pub last_change_date: Option<String>,
    pub corporate_purpose: Option<String>,
    pub activity: Option<String>,
    pub cnae_activity: Option<String>,
    pub company_status: Option<String>,
}

/// Pagina de resultados de `coincidences` con el total de filas disponibles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoincidencePage {
    pub rows: Vec<CoincidenceRecord>,
    pub total: u64,
}
