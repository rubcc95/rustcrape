import type {
  SavedConfig,
  ExecutionConfig,
  ProjectDraft,
  Config,
} from "../types";
import {
  listConfigs,
  saveConfig,
  deleteConfig,
  setLastSelected,
} from "../tauri";
import { appStore } from "./app.store.svelte";

class ProjectStore {
  projects = $state<SavedConfig[]>([]);
  currentProject = $state<SavedConfig | null>(null);
  executionConfig = $state<ExecutionConfig>({
    rate_limit: 0,
    iterations: 0,
    delay_min: 500,
    delay_max: 2000,
    stop_threshold: 3,
    use_vpn: true,
    ip_rotation_frequency: 0,
    headless: false,
  });
  projectDraft = $state<ProjectDraft>({
    name: "",
    search_query: "tintorerías",
    zoom: 12,
    db_host: "localhost",
    db_port: 3306,
    db_database: "rustcrape",
    db_user: "root",
    db_password: "",
  });

  get isNewProject(): boolean {
    return this.currentProject === null;
  }

  buildConfig(): Config {
    const gs = appStore.globalSettings;
    return {
      search: {
        persistent: {
          zoom: this.projectDraft.zoom,
          search_query: this.projectDraft.search_query,
        },
        stop_threshold: this.executionConfig.stop_threshold,
        delay_min: this.executionConfig.delay_min,
        delay_max: this.executionConfig.delay_max,
        headless: this.executionConfig.headless,
      },
      rate_limit: this.executionConfig.rate_limit,
      iterations: this.executionConfig.iterations,
      db: {
        host: this.projectDraft.db_host,
        port: this.projectDraft.db_port,
        user: this.projectDraft.db_user,
        password: this.projectDraft.db_password,
        database: this.projectDraft.db_database,
      },
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
      rate_limit: c.rate_limit,
      iterations: c.iterations,
      delay_min: c.search.delay_min,
      delay_max: c.search.delay_max,
      stop_threshold: c.search.stop_threshold,
      use_vpn: c.nordvpn_path !== null && c.nordvpn_path !== "",
      ip_rotation_frequency: c.ip_rotation_frequency,
      headless: c.search.headless,
    };
    this.projectDraft = {
      name: project.name,
      search_query: c.search.persistent.search_query,
      zoom: c.search.persistent.zoom,
      db_host: c.db.host,
      db_port: c.db.port,
      db_database: c.db.database,
      db_user: c.db.user,
      db_password: c.db.password,
    };
  }

  resetToNew(): void {
    this.currentProject = null;
    this.executionConfig = {
      rate_limit: 0,
      iterations: 0,
      delay_min: 500,
      delay_max: 2000,
      stop_threshold: 3,
      use_vpn: true,
      ip_rotation_frequency: 0,
      headless: false,
    };
    this.projectDraft = {
      name: "",
      search_query: "tintorerías",
      zoom: 12,
      db_host: "localhost",
      db_port: 3306,
      db_database: "rustcrape",
      db_user: "root",
      db_password: "",
    };
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
