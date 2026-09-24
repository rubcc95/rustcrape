import type {
  SavedConfig,
  ExecutionConfig,
  ProjectDraft,
  Config,
  DbConfig,
  EmpresiteFilters,
  IterationStats,
} from "../types";
import {
  listConfigs,
  saveConfig,
  deleteConfig,
  setLastSelected,
  loadProjectStats,
} from "../tauri";
import { isMysql } from "../types";
import { appStore } from "./app.store.svelte";

function defaultExecutionConfig(): ExecutionConfig {
  return {
    execution_mode: "Sequential",
    gmaps: {
      stop_threshold: 3,
      delay_min: 500,
      delay_max: 2000,
      headless: false,
      rate_limit: 0,
      iterations: 0,
    },
    empresite: {
      delay_min: 500,
      delay_max: 2000,
      headless: false,
      rate_limit: 0,
      iterations: 0,
    },
    use_vpn: true,
    ip_rotation_frequency: 0,
  };
}

function defaultEmpresiteFilters(): EmpresiteFilters {
  return {
    web: false,
    phone: false,
    email: false,
    location: false,
    branch: false,
    company_size: null,
    employees_enabled: false,
    employees_min: 0,
    employees_max: 100,
    incorporation_date: null,
    legal_form: null,
    province: null,
  };
}

function emptyStats(): IterationStats {
  return {
    bounds_total: 0,
    bounds_processed: 0,
    bounds_remaining: 0,
    results_found: 0,
    phones_found: 0,
  };
}

function defaultProjectDraft(): ProjectDraft {
  return {
    name: "",
    enable_google_maps: true,
    enable_empresite: false,
    search_query: "",
    zoom: 12,
    empresite_filters: defaultEmpresiteFilters(),
    use_mysql: false,
    db_host: "localhost",
    db_port: 3306,
    db_database: "rustcrape",
    db_user: "root",
    db_password: "",
  };
}

class ProjectStore {
  projects = $state<SavedConfig[]>([]);
  currentProject = $state<SavedConfig | null>(null);
  executionConfig = $state<ExecutionConfig>(defaultExecutionConfig());
  projectDraft = $state<ProjectDraft>(defaultProjectDraft());
  stats = $state<IterationStats>(emptyStats());

  get isNewProject(): boolean {
    return this.currentProject === null;
  }

  private getOrCreateDbPath(): string {
    if (this.currentProject) {
      const db = this.currentProject.config.db;
      if (db.type === "sqlite" && db.path) {
        return db.path;
      }
    }
    return `${crypto.randomUUID()}.db`;
  }

  buildConfig(): Config {
    const gs = appStore.globalSettings;
    const ef = this.projectDraft.empresite_filters;
    const db: DbConfig = this.projectDraft.use_mysql
      ? {
          type: "mysql",
          host: this.projectDraft.db_host,
          port: this.projectDraft.db_port,
          user: this.projectDraft.db_user,
          password: this.projectDraft.db_password,
          database: this.projectDraft.db_database,
        }
      : { type: "sqlite", path: this.getOrCreateDbPath() };
    return {
      gmaps: {
        enabled: this.projectDraft.enable_google_maps,
        search_query: this.projectDraft.search_query,
        zoom: this.projectDraft.zoom,
        stop_threshold: this.executionConfig.gmaps.stop_threshold,
        delay_min: this.executionConfig.gmaps.delay_min,
        delay_max: this.executionConfig.gmaps.delay_max,
        headless: this.executionConfig.gmaps.headless,
        rate_limit: this.executionConfig.gmaps.rate_limit,
        iterations: this.executionConfig.gmaps.iterations,
      },
      empresite: {
        enabled: this.projectDraft.enable_empresite,
        search_query: this.projectDraft.search_query,
        delay_min: this.executionConfig.empresite.delay_min,
        delay_max: this.executionConfig.empresite.delay_max,
        headless: this.executionConfig.empresite.headless,
        rate_limit: this.executionConfig.empresite.rate_limit,
        iterations: this.executionConfig.empresite.iterations,
        web: ef.web,
        phone: ef.phone,
        email: ef.email,
        location: ef.location,
        branch: ef.branch,
        company_size: ef.company_size,
        employees: ef.employees_enabled
          ? { min: ef.employees_min, max: ef.employees_max }
          : null,
        incorporation_date: ef.incorporation_date,
        legal_form: ef.legal_form,
        province: ef.province,
      },
      execution_mode: this.executionConfig.execution_mode,
      db,
      nordvpn_path: this.executionConfig.use_vpn
        ? gs.nordvpn_path
        : null,
      browser_path: gs.browser_path,
      ip_rotation_frequency: this.executionConfig.use_vpn
        ? this.executionConfig.ip_rotation_frequency
        : 0,
    };
  }

  selectProject(project: SavedConfig): void {
    this.currentProject = project;
    const c = project.config;
    this.executionConfig = {
      execution_mode: c.execution_mode,
      gmaps: {
        stop_threshold: c.gmaps.stop_threshold,
        delay_min: c.gmaps.delay_min,
        delay_max: c.gmaps.delay_max,
        headless: c.gmaps.headless,
        rate_limit: c.gmaps.rate_limit,
        iterations: c.gmaps.iterations,
      },
      empresite: {
        delay_min: c.empresite.delay_min,
        delay_max: c.empresite.delay_max,
        headless: c.empresite.headless,
        rate_limit: c.empresite.rate_limit,
        iterations: c.empresite.iterations,
      },
      use_vpn: c.nordvpn_path !== null && c.nordvpn_path !== "",
      ip_rotation_frequency: c.ip_rotation_frequency,
    };
    const db = c.db;
    const useMysql = isMysql(db);
    this.projectDraft = {
      name: project.name,
      enable_google_maps: c.gmaps.enabled,
      enable_empresite: c.empresite.enabled,
      search_query: c.gmaps.search_query,
      zoom: c.gmaps.zoom,
      empresite_filters: {
        web: c.empresite.web ?? false,
        phone: c.empresite.phone ?? false,
        email: c.empresite.email ?? false,
        location: c.empresite.location ?? false,
        branch: c.empresite.branch ?? false,
        company_size: c.empresite.company_size ?? null,
        employees_enabled: c.empresite.employees != null,
        employees_min: c.empresite.employees?.min ?? 0,
        employees_max: c.empresite.employees?.max ?? 100,
        incorporation_date: c.empresite.incorporation_date ?? null,
        legal_form: c.empresite.legal_form ?? null,
        province: c.empresite.province ?? null,
      },
      use_mysql: useMysql,
      db_host: useMysql ? db.host : "localhost",
      db_port: useMysql ? db.port : 3306,
      db_database: useMysql ? db.database : "rustcrape",
      db_user: useMysql ? db.user : "root",
      db_password: useMysql ? db.password : "",
    };
    this.loadStats(project.config);
  }

  async loadStats(config: Config): Promise<void> {
    try {
      this.stats = await loadProjectStats(config);
    } catch (e) {
      console.error("Error loading project stats:", e);
      this.stats = emptyStats();
    }
  }

  resetToNew(): void {
    this.currentProject = null;
    this.executionConfig = defaultExecutionConfig();
    this.projectDraft = defaultProjectDraft();
    this.stats = emptyStats();
  }

  async loadProjects(): Promise<void> {
    try {
      this.projects = await listConfigs();
    } catch (e) {
      console.error("Error loading projects:", e);
    }
  }

  async saveCurrentProject(): Promise<{
    saved: SavedConfig | null;
    config: Config;
  }> {
    const name = this.projectDraft.name.trim();
    if (!name) {
      return { saved: null, config: this.buildConfig() };
    }

    try {
      const config = this.buildConfig();
      const started = this.currentProject?.started ?? false;
      const saved = await saveConfig(name, config, started);
      this.currentProject = saved;
      await setLastSelected(saved.id);
      await this.loadProjects();
      return { saved, config };
    } catch (e) {
      console.error("Error saving project:", e);
      return { saved: null, config: this.buildConfig() };
    }
  }

  async removeProject(id: string): Promise<boolean> {
    try {
      await deleteConfig(id);
      if (this.currentProject?.id === id) {
        this.resetToNew();
      }
      await this.loadProjects();
      return true;
    } catch (e) {
      console.error("Error deleting project:", e);
      return false;
    }
  }
}

export const projectStore = new ProjectStore();
