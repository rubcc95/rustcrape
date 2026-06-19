import type { LogEntry, IterationStats, Config } from "../types";
import {
  runScraping,
  cancelScraping,
  fetchProjectStats,
  onVerboserEvent,
  onScrapingStarted,
  onScrapingFinished,
} from "../tauri";
import { appStore } from "./app.store.svelte";

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

  async startExecution(config: Config): Promise<void> {
    const savedBoundsTotal = this.stats.bounds_total;
    this.resetExecution();
    this.stats.bounds_total = savedBoundsTotal;
    this.stats.bounds_remaining = savedBoundsTotal;
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
    appStore.navigate("project");
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

  private updateStatsFromEvent(kind: string, message: string): void {
    if (kind === "released_bound") {
      this.stats.bounds_processed++;
      if (this.stats.bounds_remaining > 0) {
        this.stats.bounds_remaining--;
      }
    }
    if (kind === "written_coincidences") {
      const m = message.match(/^(\d+).*\((\d+)/);
      if (m) {
        this.stats.results_found += parseInt(m[1]);
        this.stats.phones_found = parseInt(m[2]);
      }
    }
    if(kind === ""){
      
    }
  }

  setupExecutionListeners(): () => void {
    const unlisteners: Array<() => void> = [];

    onVerboserEvent((payload) => {
      this.addLog(payload.kind, payload.message);
      this.updateStatsFromEvent(payload.kind, payload.message);
    }).then((fn) => unlisteners.push(fn));

    onScrapingStarted(() => {
      this.isRunning = true;
    }).then((fn) => unlisteners.push(fn));

    onScrapingFinished(async () => {
      this.isRunning = false;
      this.isCancelling = false;
      this.addLog("info", "Scraping finalizado");
      if (this.configSnapshot) {
        try {
          const fresh = await fetchProjectStats(this.configSnapshot);
          this.stats.bounds_total = fresh.bounds_total;
          this.stats.bounds_processed = fresh.bounds_processed;
          this.stats.bounds_remaining = fresh.bounds_remaining;
          this.stats.results_found = fresh.results_found;
          this.stats.phones_found = fresh.phones_found;
        } catch {
          // keep incremental values if fetch fails
        }
      }
    }).then((fn) => unlisteners.push(fn));

    return () => {
      for (const fn of unlisteners) fn();
    };
  }

  async resumeExecution(): Promise<void> {
    if (!this.configSnapshot) return;
    const config = JSON.parse(JSON.stringify(this.configSnapshot));
    await this.startExecution(config);
  }

  returnToProject(): void {
    appStore.navigate("project");
  }
}

export const executionStore = new ExecutionStore();
