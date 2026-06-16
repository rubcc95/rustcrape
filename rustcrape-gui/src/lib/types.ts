export type Route = "settings" | "project" | "execution";

export interface PersistentConfig {
  zoom: number;
  search_query: string;
}

export interface SearchConfig {
  persistent: PersistentConfig;
  stop_threshold: number;
  delay_min: number;
  delay_max: number;
  headless: boolean;
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
  search: SearchConfig;
  rate_limit: number;
  iterations: number;
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

export interface ExecutionConfig {
  rate_limit: number;
  iterations: number;
  delay_min: number;
  delay_max: number;
  stop_threshold: number;
  use_vpn: boolean;
  ip_rotation_frequency: number;
  headless: boolean;
}

export interface ProjectDraft {
  name: string;
  search_query: string;
  zoom: number;
  use_mysql: boolean;
  db_host: string;
  db_port: number;
  db_database: string;
  db_user: string;
  db_password: string;
}
