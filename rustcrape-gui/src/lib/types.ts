import { TOWNS_BY_PROVINCE } from "./data/empresiteLocations";

export type Route = "settings" | "project" | "execution" | "results";

export type CoincidenceColumn =
  | "id"
  | "name"
  | "web"
  | "email"
  | "tfno"
  | "source_url"
  | "source"
  | "creado"
  | "legal_name"
  | "tax_id"
  | "legal_form"
  | "sector"
  | "incorporation_date"
  | "last_change_date"
  | "corporate_purpose"
  | "activity"
  | "cnae_activity"
  | "company_status";

export type SortOrder = "asc" | "desc";

export interface CoincidenceRecord {
  id: number;
  name: string;
  web: string;
  email: string;
  tfno: string;
  source_url: string;
  source: string;
  creado: string | null;
  legal_name: string | null;
  tax_id: string | null;
  legal_form: string | null;
  sector: string | null;
  incorporation_date: string | null;
  last_change_date: string | null;
  corporate_purpose: string | null;
  activity: string | null;
  cnae_activity: string | null;
  company_status: string | null;
}

export interface CoincidencePage {
  rows: CoincidenceRecord[];
  total: number;
}

export type ExecutionMode = "Sequential" | "Parallel";

export interface GoogleMapsConfig {
  enabled: boolean;
  search_query: string;
  zoom: number;
  stop_threshold: number;
  delay_min: number;
  delay_max: number;
  headless: boolean;
  rate_limit: number;
  iterations: number;
}

export type CompanySize = "small" | "medium" | "large" | "corporate";

export type IncorporationDate =
  | "last_month"
  | "last_three_months"
  | "last_year"
  | "more_than_a_year";

export type LegalForm =
  | "limited_liability_company"
  | "community_of_property"
  | "civil_partnership"
  | "public_limited_company"
  | "temporary_joint_venture"
  | "cooperative"
  | "public_body"
  | "local_corporation"
  | "public_administration"
  | "foreign_entity";

export type Province =
  | "coruna"
  | "alava"
  | "albacete"
  | "alicante"
  | "almeria"
  | "asturias"
  | "avila"
  | "badajoz"
  | "baleares"
  | "barcelona"
  | "burgos"
  | "caceres"
  | "cadiz"
  | "cantabria"
  | "castellon"
  | "ceuta"
  | "ciudad_real"
  | "cordoba"
  | "cuenca"
  | "gerona"
  | "granada"
  | "guadalajara"
  | "guipuzcoa"
  | "huelva"
  | "huesca"
  | "jaen"
  | "leon"
  | "lerida"
  | "lugo"
  | "madrid"
  | "malaga"
  | "melilla"
  | "murcia"
  | "navarra"
  | "orense"
  | "palencia"
  | "palmas"
  | "pontevedra"
  | "rioja"
  | "salamanca"
  | "santa_cruz_de_tenerife"
  | "segovia"
  | "sevilla"
  | "soria"
  | "tarragona"
  | "teruel"
  | "toledo"
  | "valencia"
  | "valladolid"
  | "vizcaya"
  | "zamora"
  | "zaragoza";

/** Provincia de Empresite con el slug de URL (id del JSON) y la etiqueta. */
export interface ProvinceOption {
  value: Province;
  slug: string;
  label: string;
}

export const PROVINCES: ProvinceOption[] = [
  { value: "coruna", slug: "CORUNA", label: "A Coruña" },
  { value: "alava", slug: "ALAVA", label: "Álava" },
  { value: "albacete", slug: "ALBACETE", label: "Albacete" },
  { value: "alicante", slug: "ALICANTE", label: "Alicante" },
  { value: "almeria", slug: "ALMERIA", label: "Almería" },
  { value: "asturias", slug: "ASTURIAS", label: "Asturias" },
  { value: "avila", slug: "AVILA", label: "Ávila" },
  { value: "badajoz", slug: "BADAJOZ", label: "Badajoz" },
  { value: "baleares", slug: "BALEARES", label: "Baleares" },
  { value: "barcelona", slug: "BARCELONA", label: "Barcelona" },
  { value: "burgos", slug: "BURGOS", label: "Burgos" },
  { value: "caceres", slug: "CACERES", label: "Cáceres" },
  { value: "cadiz", slug: "CADIZ", label: "Cádiz" },
  { value: "cantabria", slug: "CANTABRIA", label: "Cantabria" },
  { value: "castellon", slug: "CASTELLON", label: "Castellón" },
  { value: "ceuta", slug: "CEUTA", label: "Ceuta" },
  { value: "ciudad_real", slug: "CIUDAD-REAL", label: "Ciudad Real" },
  { value: "cordoba", slug: "CORDOBA", label: "Córdoba" },
  { value: "cuenca", slug: "CUENCA", label: "Cuenca" },
  { value: "gerona", slug: "GERONA", label: "Girona" },
  { value: "granada", slug: "GRANADA", label: "Granada" },
  { value: "guadalajara", slug: "GUADALAJARA", label: "Guadalajara" },
  { value: "guipuzcoa", slug: "GUIPUZCOA", label: "Guipúzcoa" },
  { value: "huelva", slug: "HUELVA", label: "Huelva" },
  { value: "huesca", slug: "HUESCA", label: "Huesca" },
  { value: "jaen", slug: "JAEN", label: "Jaén" },
  { value: "leon", slug: "LEON", label: "León" },
  { value: "lerida", slug: "LERIDA", label: "Lleida" },
  { value: "lugo", slug: "LUGO", label: "Lugo" },
  { value: "madrid", slug: "MADRID", label: "Madrid" },
  { value: "malaga", slug: "MALAGA", label: "Málaga" },
  { value: "melilla", slug: "MELILLA", label: "Melilla" },
  { value: "murcia", slug: "MURCIA", label: "Murcia" },
  { value: "navarra", slug: "NAVARRA", label: "Navarra" },
  { value: "orense", slug: "ORENSE", label: "Ourense" },
  { value: "palencia", slug: "PALENCIA", label: "Palencia" },
  { value: "palmas", slug: "PALMAS", label: "Las Palmas" },
  { value: "pontevedra", slug: "PONTEVEDRA", label: "Pontevedra" },
  { value: "rioja", slug: "RIOJA", label: "La Rioja" },
  { value: "salamanca", slug: "SALAMANCA", label: "Salamanca" },
  { value: "santa_cruz_de_tenerife", slug: "SANTA-CRUZ-TENERIFE", label: "Santa Cruz de Tenerife" },
  { value: "segovia", slug: "SEGOVIA", label: "Segovia" },
  { value: "sevilla", slug: "SEVILLA", label: "Sevilla" },
  { value: "soria", slug: "SORIA", label: "Soria" },
  { value: "tarragona", slug: "TARRAGONA", label: "Tarragona" },
  { value: "teruel", slug: "TERUEL", label: "Teruel" },
  { value: "toledo", slug: "TOLEDO", label: "Toledo" },
  { value: "valencia", slug: "VALENCIA", label: "Valencia" },
  { value: "valladolid", slug: "VALLADOLID", label: "Valladolid" },
  { value: "vizcaya", slug: "VIZCAYA", label: "Vizcaya" },
  { value: "zamora", slug: "ZAMORA", label: "Zamora" },
  { value: "zaragoza", slug: "ZARAGOZA", label: "Zaragoza" },
];

/** Slug de provincia (id del JSON) a partir del valor interno del selector. */
export function provinceSlug(province: Province | null): string {
  if (!province) return "";
  return PROVINCES.find((p) => p.value === province)?.slug ?? "";
}

/** Indice inverso: id de localidad -> slug de su provincia. */
export const PROVINCE_SLUG_BY_TOWN: Record<string, string> = (() => {
  const map: Record<string, string> = {};
  for (const [slug, towns] of Object.entries(TOWNS_BY_PROVINCE)) {
    for (const town of towns) map[town.id] = slug;
  }
  return map;
})();

/** Filtro geográfico de Empresite: provincia o localidad (excluyentes). */
export type EmpresiteLocation =
  | { province: Province }
  | { locality: { id: string } };

export interface EmployeeRange {
  min: number;
  max: number;
}

/** Filtros del listado de Empresite tal como viajan en la configuracion. */
export interface EmpresiteConfig {
  enabled: boolean;
  search_query: string;
  delay_min: number;
  delay_max: number;
  headless: boolean;
  rate_limit: number;
  iterations: number;
  web: boolean;
  phone: boolean;
  email: boolean;
  location: boolean;
  branch: boolean;
  company_size: CompanySize | null;
  employees: EmployeeRange | null;
  incorporation_date: IncorporationDate | null;
  legal_form: LegalForm | null;
  location_filter: EmpresiteLocation | null;
}

/** Modo del filtro geográfico en el estado del formulario. */
export type LocationMode = "none" | "province" | "locality";

/** Filtros de Empresite en el estado del formulario (rango con bandera). */
export interface EmpresiteFilters {
  web: boolean;
  phone: boolean;
  email: boolean;
  location: boolean;
  branch: boolean;
  company_size: CompanySize | null;
  employees_enabled: boolean;
  employees_min: number;
  employees_max: number;
  incorporation_date: IncorporationDate | null;
  legal_form: LegalForm | null;
  location_mode: LocationMode;
  locality_id: string;
  province: Province | null;
}

export type DbConfig =
  | { type: "sqlite"; path?: string | null }
  | { type: "mysql"; host: string; port: number; user: string; password: string; database: string };

export function isMysql(db: DbConfig): db is { type: "mysql"; host: string; port: number; user: string; password: string; database: string } {
  return db.type === "mysql";
}

export function isSqlite(db: DbConfig): db is { type: "sqlite"; path?: string | null } {
  return db.type === "sqlite";
}

export interface Config {
  gmaps: GoogleMapsConfig;
  empresite: EmpresiteConfig;
  execution_mode: ExecutionMode;
  db: DbConfig;
  nordvpn_path: string | null;
  browser_path: string | null;
  ip_rotation_frequency: number;
}

export interface SavedConfig {
  id: string;
  name: string;
  config: Config;
  started: boolean;
}

export interface GlobalSettings {
  nordvpn_path: string | null;
  browser_path: string | null;
}

export interface VerboserPayload {
  kind: string;
  message: string;
  inserted?: number;
  inserted_with_phone?: number;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  kind: string;
  message: string;
}

export interface IterationStats {
  bounds_total: number;
  bounds_processed: number;
  bounds_remaining: number;
  results_found: number;
  phones_found: number;
}

export interface Announcement {
  title: string;
  content: string;
  link: string;
  link_label: string;
  closeable: boolean;
}

export interface GmapsExecutionConfig {
  stop_threshold: number;
  delay_min: number;
  delay_max: number;
  headless: boolean;
  rate_limit: number;
  iterations: number;
}

export interface EmpresiteExecutionConfig {
  delay_min: number;
  delay_max: number;
  headless: boolean;
  rate_limit: number;
  iterations: number;
}

export interface ExecutionConfig {
  execution_mode: ExecutionMode;
  gmaps: GmapsExecutionConfig;
  empresite: EmpresiteExecutionConfig;
  use_vpn: boolean;
  ip_rotation_frequency: number;
}

export interface ProjectDraft {
  name: string;
  enable_google_maps: boolean;
  enable_empresite: boolean;
  search_query: string;
  zoom: number;
  empresite_filters: EmpresiteFilters;
  use_mysql: boolean;
  db_host: string;
  db_port: number;
  db_database: string;
  db_user: string;
  db_password: string;
}
