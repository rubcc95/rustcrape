import type { LogEntry, IterationStats, Config } from "../types";
import {
  runScraping,
  cancelScraping,
  onVerboserEvent,
  onScrapingStarted,
  onScrapingFinished,
} from "../tauri";
import { appStore } from "./app.store.svelte";
import { projectStore } from "./project.store.svelte";

class ExecutionStore {
  isRunning = $state(false);
  isCancelling = $state(false);
  logs = $state<LogEntry[]>([]);
  stats = $state<IterationStats>({
    bounds_total: 0,
    bounds_processed: 0,
    bounds_remaining: 0,
    results_found: 0,
    phones_found: 0,
  });
  configSnapshot = $state<Config | null>(null);

  private logCounter = 0;

  private addLog(kind: string, message: string): void {
    const now = new Date();
    const time = now.toLocaleTimeString("es-ES", { hour12: false });
    this.logs.push({
      id: `log-${++this.logCounter}`,
      timestamp: time,
      kind,
      message,
    });
  }

  async startExecution(config: Config, seedStats = true): Promise<void> {
    this.resetExecution();
    if (seedStats) {
      this.stats = { ...projectStore.stats };
    }
    this.configSnapshot = JSON.parse(JSON.stringify(config));
    this.isRunning = true;
    appStore.navigate("execution");

    this.addLog("info", "Iniciando scraping...");
    try {
      await runScraping(config);
    } catch (e) {
      this.addLog("error", `Error: ${e}`);
    }
  }

  async cancelExecution(): Promise<void> {
    this.isCancelling = true;
    this.addLog("info", "Cancelando scraping...");
    try {
      await cancelScraping();
      this.addLog("info", "Cancelación solicitada");
    } catch (e) {
      this.addLog("error", `Error al cancelar: ${e}`);
    }
  }

  async goBack(): Promise<void> {
    if (this.isRunning) {
      this.isCancelling = true;
      this.addLog("info", "Cancelando scraping...");
      try {
        await cancelScraping();
      } catch (e) {
        this.addLog("error", `Error al cancelar: ${e}`);
      }
    }
    this.syncStatsToProject();
    appStore.navigate("project");
  }

  private syncStatsToProject(): void {
    projectStore.stats = { ...this.stats };
  }

  resetExecution(): void {
    this.isCancelling = false;
    this.logs = [];
    this.logCounter = 0;
    this.stats = {
      bounds_total: 0,
      bounds_processed: 0,
      bounds_remaining: 0,
      results_found: 0,
      phones_found: 0,
    };
    this.configSnapshot = null;
  }

  setupExecutionListeners(): () => void {
    const unlisteners: Array<() => void> = [];

    onVerboserEvent((payload) => {
      this.addLog(payload.kind, payload.message);
      if (payload.kind === "released_task") {
        this.stats.bounds_processed += 1;
        this.stats.bounds_remaining = Math.max(
          0,
          this.stats.bounds_remaining - 1
        );
      } else if (payload.kind === "written_coincidences") {
        this.stats.results_found += payload.inserted ?? 0;
        this.stats.phones_found += payload.inserted_with_phone ?? 0;
      }
    }).then((fn) => unlisteners.push(fn));

    onScrapingStarted(() => {
      this.isRunning = true;
    }).then((fn) => unlisteners.push(fn));

    onScrapingFinished(() => {
      this.isRunning = false;
      this.isCancelling = false;
      this.addLog("info", "Scraping finalizado");
      this.syncStatsToProject();
    }).then((fn) => unlisteners.push(fn));

    return () => {
      for (const fn of unlisteners) fn();
    };
  }

  async resumeExecution(): Promise<void> {
    if (!this.configSnapshot) return;
    const config = JSON.parse(JSON.stringify(this.configSnapshot));
    await this.startExecution(config, false);
  }

  returnToProject(): void {
    this.syncStatsToProject();
    appStore.navigate("project");
  }
}

export const executionStore = new ExecutionStore();
