import type {
  SavedConfig,
  ExecutionConfig,
  ProjectDraft,
  Config,
  DbConfig,
} from "../types";
import {
  listConfigs,
  saveConfig,
  deleteConfig,
  setLastSelected,
} from "../tauri";
import { isMysql } from "../types";
import { appStore } from "./app.store.svelte";

function defaultExecutionConfig(): ExecutionConfig {
  return {
    execution_mode: "Sequential",
    google_maps: {
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

function defaultProjectDraft(): ProjectDraft {
  return {
    name: "",
    enable_google_maps: true,
    enable_empresite: false,
    search_query: "",
    zoom: 12,
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
      google_maps: {
        enabled: this.projectDraft.enable_google_maps,
        search_query: this.projectDraft.search_query,
        zoom: this.projectDraft.zoom,
        stop_threshold: this.executionConfig.google_maps.stop_threshold,
        delay_min: this.executionConfig.google_maps.delay_min,
        delay_max: this.executionConfig.google_maps.delay_max,
        headless: this.executionConfig.google_maps.headless,
        rate_limit: this.executionConfig.google_maps.rate_limit,
        iterations: this.executionConfig.google_maps.iterations,
      },
      empresite: {
        enabled: this.projectDraft.enable_empresite,
        search_query: this.projectDraft.search_query,
        delay_min: this.executionConfig.empresite.delay_min,
        delay_max: this.executionConfig.empresite.delay_max,
        headless: this.executionConfig.empresite.headless,
        rate_limit: this.executionConfig.empresite.rate_limit,
        iterations: this.executionConfig.empresite.iterations,
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
      google_maps: {
        stop_threshold: c.google_maps.stop_threshold,
        delay_min: c.google_maps.delay_min,
        delay_max: c.google_maps.delay_max,
        headless: c.google_maps.headless,
        rate_limit: c.google_maps.rate_limit,
        iterations: c.google_maps.iterations,
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
      enable_google_maps: c.google_maps.enabled,
      enable_empresite: c.empresite.enabled,
      search_query: c.google_maps.search_query,
      zoom: c.google_maps.zoom,
      use_mysql: useMysql,
      db_host: useMysql ? db.host : "localhost",
      db_port: useMysql ? db.port : 3306,
      db_database: useMysql ? db.database : "rustcrape",
      db_user: useMysql ? db.user : "root",
      db_password: useMysql ? db.password : "",
    };
  }

  resetToNew(): void {
    this.currentProject = null;
    this.executionConfig = defaultExecutionConfig();
    this.projectDraft = defaultProjectDraft();
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
