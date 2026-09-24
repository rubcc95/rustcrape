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

/** Provincias de Empresite con su etiqueta para el selector. */
export const PROVINCES: { value: Province; label: string }[] = [
  { value: "coruna", label: "A Coruña" },
  { value: "alava", label: "Álava" },
  { value: "albacete", label: "Albacete" },
  { value: "alicante", label: "Alicante" },
  { value: "almeria", label: "Almería" },
  { value: "asturias", label: "Asturias" },
  { value: "avila", label: "Ávila" },
  { value: "badajoz", label: "Badajoz" },
  { value: "baleares", label: "Baleares" },
  { value: "barcelona", label: "Barcelona" },
  { value: "burgos", label: "Burgos" },
  { value: "caceres", label: "Cáceres" },
  { value: "cadiz", label: "Cádiz" },
  { value: "cantabria", label: "Cantabria" },
  { value: "castellon", label: "Castellón" },
  { value: "ceuta", label: "Ceuta" },
  { value: "ciudad_real", label: "Ciudad Real" },
  { value: "cordoba", label: "Córdoba" },
  { value: "cuenca", label: "Cuenca" },
  { value: "gerona", label: "Girona" },
  { value: "granada", label: "Granada" },
  { value: "guadalajara", label: "Guadalajara" },
  { value: "guipuzcoa", label: "Guipúzcoa" },
  { value: "huelva", label: "Huelva" },
  { value: "huesca", label: "Huesca" },
  { value: "jaen", label: "Jaén" },
  { value: "leon", label: "León" },
  { value: "lerida", label: "Lleida" },
  { value: "lugo", label: "Lugo" },
  { value: "madrid", label: "Madrid" },
  { value: "malaga", label: "Málaga" },
  { value: "melilla", label: "Melilla" },
  { value: "murcia", label: "Murcia" },
  { value: "navarra", label: "Navarra" },
  { value: "orense", label: "Ourense" },
  { value: "palencia", label: "Palencia" },
  { value: "palmas", label: "Las Palmas" },
  { value: "pontevedra", label: "Pontevedra" },
  { value: "rioja", label: "La Rioja" },
  { value: "salamanca", label: "Salamanca" },
  { value: "santa_cruz_de_tenerife", label: "Santa Cruz de Tenerife" },
  { value: "segovia", label: "Segovia" },
  { value: "sevilla", label: "Sevilla" },
  { value: "soria", label: "Soria" },
  { value: "tarragona", label: "Tarragona" },
  { value: "teruel", label: "Teruel" },
  { value: "toledo", label: "Toledo" },
  { value: "valencia", label: "Valencia" },
  { value: "valladolid", label: "Valladolid" },
  { value: "vizcaya", label: "Vizcaya" },
  { value: "zamora", label: "Zamora" },
  { value: "zaragoza", label: "Zaragoza" },
];

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
  province: Province | null;
}

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
