import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  SavedConfig,
  Config,
  GlobalSettings,
  VerboserPayload,
  Announcement,
} from "./types";

// ── Config CRUD ──

export async function listConfigs(): Promise<SavedConfig[]> {
  return invoke("list_configs");
}

export async function saveConfig(
  name: string,
  config: Config,
  started: boolean
): Promise<SavedConfig> {
  return invoke("save_config", { name, config, started });
}

export async function deleteConfig(id: string): Promise<void> {
  return invoke("delete_config", { id });
}

export async function getLastSelected(): Promise<SavedConfig | null> {
  return invoke("get_last_selected");
}

export async function setLastSelected(id: string): Promise<void> {
  return invoke("set_last_selected", { id });
}

// ── App ──

export async function checkAnnouncement(): Promise<Announcement | null> {
  return invoke("check_announcement");
}

// ── Execution ──

export async function runScraping(config: Config): Promise<void> {
  return invoke("run_scraping", { config });
}

export async function cancelScraping(): Promise<void> {
  return invoke("cancel_scraping");
}

// ── Global Settings ──

export async function loadGlobalSettings(): Promise<GlobalSettings> {
  return invoke("load_global_settings");
}

export async function saveGlobalSettings(
  settings: GlobalSettings
): Promise<void> {
  return invoke("save_global_settings", { settings });
}

// ── File picker ──

export async function pickExecutable(key: string): Promise<void> {
  return invoke("pick_executable", { key });
}

// ── Event listeners ──

export function onVerboserEvent(
  callback: (payload: VerboserPayload) => void
): Promise<UnlistenFn> {
  return listen<VerboserPayload>("verboser-event", (event) => {
    callback(event.payload);
  });
}

export function onScrapingStarted(
  callback: () => void
): Promise<UnlistenFn> {
  return listen("scraping-started", () => callback());
}

export function onScrapingFinished(
  callback: () => void
): Promise<UnlistenFn> {
  return listen("scraping-finished", () => callback());
}

export interface ExecutablePickedPayload {
  key: string;
  path: string | null;
}

export function onExecutablePicked(
  callback: (payload: ExecutablePickedPayload) => void
): Promise<UnlistenFn> {
  return listen<ExecutablePickedPayload>("executable-picked", (event) => {
    callback(event.payload);
  });
}
