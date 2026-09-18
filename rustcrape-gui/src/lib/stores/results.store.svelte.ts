import type {
  CoincidenceColumn,
  CoincidenceRecord,
  Config,
  SortOrder,
} from "../types";
import { loadCoincidences } from "../tauri";
import { appStore } from "./app.store.svelte";
import { projectStore } from "./project.store.svelte";

const PAGE_SIZES = [25, 50, 100, 200];

class ResultsStore {
  config = $state<Config | null>(null);
  rows = $state<CoincidenceRecord[]>([]);
  total = $state(0);
  page = $state(1);
  pageSize = $state(50);
  sortColumn = $state<CoincidenceColumn>("id");
  sortOrder = $state<SortOrder>("asc");
  loading = $state(false);
  error = $state<string | null>(null);

  get pageSizes(): number[] {
    return PAGE_SIZES;
  }

  get totalPages(): number {
    return Math.max(1, Math.ceil(this.total / this.pageSize));
  }

  get canGoPrev(): boolean {
    return this.page > 1;
  }

  get canGoNext(): boolean {
    return this.page < this.totalPages;
  }

  async openForCurrentProject(): Promise<void> {
    const cfg = projectStore.currentProject?.config ?? null;
    if (!cfg) return;
    this.config = cfg;
    this.page = 1;
    this.sortColumn = "id";
    this.sortOrder = "asc";
    this.error = null;
    appStore.navigate("results");
    await this.reload();
  }

  async reload(): Promise<void> {
    if (!this.config) return;
    this.loading = true;
    this.error = null;
    try {
      const result = await loadCoincidences(
        this.config,
        this.sortColumn,
        this.sortOrder,
        this.page,
        this.pageSize
      );
      this.rows = result.rows;
      this.total = result.total;
      if (this.page > this.totalPages) {
        this.page = this.totalPages;
        await this.reload();
      }
    } catch (e) {
      this.error = String(e);
      this.rows = [];
      this.total = 0;
    } finally {
      this.loading = false;
    }
  }

  async setSort(column: CoincidenceColumn): Promise<void> {
    if (this.sortColumn === column) {
      this.sortOrder = this.sortOrder === "asc" ? "desc" : "asc";
    } else {
      this.sortColumn = column;
      this.sortOrder = "asc";
    }
    this.page = 1;
    await this.reload();
  }

  async setPage(page: number): Promise<void> {
    if (page < 1 || page > this.totalPages) return;
    this.page = page;
    await this.reload();
  }

  async setPageSize(size: number): Promise<void> {
    this.pageSize = size;
    this.page = 1;
    await this.reload();
  }
}

export const resultsStore = new ResultsStore();
