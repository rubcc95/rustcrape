import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface PersistentConfig {
  zoom: number;
  search_query: string;
}

interface SearchConfig {
  persistent: PersistentConfig;
  stop_threshold: number;
  delay_min: number;
  delay_max: number;
  headless: boolean;
}

interface DbConfig {
  host: string;
  port: number;
  user: string;
  password: string;
  database: string;
}

interface Config {
  search: SearchConfig;
  rate_limit: number;
  iterations: number;
  db: DbConfig;
  nordvpn_path: string | null;
  ip_rotation_frequency: number;
}

interface SavedConfig {
  id: string;
  name: string;
  config: Config;
  started: boolean;
}

interface VerboserPayload {
  kind: string;
  message: string;
}

// State
let currentId: string | null = null;
let currentStarted: boolean = false;
let isRunning: boolean = false;
let unlisten: UnlistenFn | null = null;
let savedConfigSnapshot: { name: string; config: Config } | null = null;
const projectLogs = new Map<string, string>();

// DOM refs
const projectList = document.getElementById("project-list")!;
const form = document.getElementById("config-form") as HTMLFormElement;
const projectName = document.getElementById("project-name") as HTMLInputElement;
const searchQuery = document.getElementById("search-query") as HTMLInputElement;
const zoom = document.getElementById("zoom") as HTMLInputElement;
const stopThreshold = document.getElementById("stop-threshold") as HTMLInputElement;
const delayMin = document.getElementById("delay-min") as HTMLInputElement;
const delayMax = document.getElementById("delay-max") as HTMLInputElement;
const headless = document.getElementById("headless") as HTMLInputElement;
const rateLimit = document.getElementById("rate-limit") as HTMLInputElement;
const iterations = document.getElementById("iterations") as HTMLInputElement;
const dbHost = document.getElementById("db-host") as HTMLInputElement;
const dbPort = document.getElementById("db-port") as HTMLInputElement;
const dbDatabase = document.getElementById("db-database") as HTMLInputElement;
const dbUser = document.getElementById("db-user") as HTMLInputElement;
const dbPassword = document.getElementById("db-password") as HTMLInputElement;
const nordvpnPath = document.getElementById("nordvpn-path") as HTMLInputElement;
const ipRotationFrequency = document.getElementById("ip-rotation-frequency") as HTMLInputElement;
const projectLogOutput = document.getElementById("project-log-output")!;
const clearProjectLogBtn = document.getElementById("clear-project-log-btn")!;
const saveBtn = document.getElementById("save-btn")!;
const runBtn = document.getElementById("run-btn")!;
const deleteBtn = document.getElementById("delete-btn")!;
const newConfigBtn = document.getElementById("new-config-btn")!;
const modalOverlay = document.getElementById("modal-overlay")!;
const modalMessage = document.getElementById("modal-message")!;
const modalCancelBtn = document.getElementById("modal-cancel-btn")!;
const modalConfirmBtn = document.getElementById("modal-confirm-btn")!;

function readForm(): Config {
  return {
    search: {
      persistent: {
        zoom: parseInt(zoom.value) || 12,
        search_query: searchQuery.value,
      },
      stop_threshold: parseInt(stopThreshold.value) || 3,
      delay_min: parseInt(delayMin.value) || 500,
      delay_max: parseInt(delayMax.value) || 2000,
      headless: headless.checked,
    },
    rate_limit: parseInt(rateLimit.value) || 0,
    iterations: parseInt(iterations.value) || 0,
    db: {
      host: dbHost.value,
      port: parseInt(dbPort.value) || 3306,
      user: dbUser.value,
      password: dbPassword.value,
      database: dbDatabase.value,
    },
    nordvpn_path: nordvpnPath.value || null,
    ip_rotation_frequency: parseInt(ipRotationFrequency.value) || 0,
  };
}

function fillForm(config: Config): void {
  searchQuery.value = config.search.persistent.search_query;
  zoom.value = config.search.persistent.zoom.toString();
  stopThreshold.value = config.search.stop_threshold.toString();
  delayMin.value = config.search.delay_min.toString();
  delayMax.value = config.search.delay_max.toString();
  headless.checked = config.search.headless;
  rateLimit.value = config.rate_limit.toString();
  iterations.value = config.iterations.toString();
  dbHost.value = config.db.host;
  dbPort.value = config.db.port.toString();
  dbDatabase.value = config.db.database;
  dbUser.value = config.db.user;
  dbPassword.value = config.db.password;
  nordvpnPath.value = config.nordvpn_path || "";
  ipRotationFrequency.value = config.ip_rotation_frequency.toString();
  updateSnapshot(projectName.value, config);
}

function resetForm(): void {
  currentId = null;
  currentStarted = false;
  savedConfigSnapshot = null;
  projectName.value = "";
  searchQuery.value = "tintorerías";
  zoom.value = "12";
  stopThreshold.value = "3";
  delayMin.value = "500";
  delayMax.value = "2000";
  headless.checked = false;
  rateLimit.value = "0";
  iterations.value = "0";
  dbHost.value = "localhost";
  dbPort.value = "3306";
  dbDatabase.value = "biz_scraping";
  dbUser.value = "root";
  dbPassword.value = "";
  nordvpnPath.value = "";
  ipRotationFrequency.value = "0";
  projectLogOutput.innerHTML = "";
  projectLogs.delete("unsaved");
  highlightSelected(null);
  updateUI();
}

function highlightSelected(id: string | null): void {
  document.querySelectorAll(".project-item").forEach((el) => {
    el.classList.toggle("selected", el.getAttribute("data-id") === id);
  });
}

function updateSnapshot(name: string, config: Config): void {
  savedConfigSnapshot = { name, config: JSON.parse(JSON.stringify(config)) };
}

function hasUnsavedChanges(): boolean {
  if (!savedConfigSnapshot) return false;
  return projectName.value !== savedConfigSnapshot.name ||
    JSON.stringify(readForm()) !== JSON.stringify(savedConfigSnapshot.config);
}

function updateUI(): void {
  const isNewProject = currentId === null;

  // Toggle save/delete buttons — hidden only for unsaved new projects
  saveBtn.style.display = isNewProject ? "none" : "";
  deleteBtn.style.display = isNewProject ? "none" : "";

  // Lock search query and zoom only for started (ejecutados) projects
  searchQuery.disabled = currentStarted;
  zoom.disabled = currentStarted;

  // Toggle run/cancel button
  const running = isRunning;
  if (running) {
    runBtn.textContent = "Cancelar";
    runBtn.className = "btn-danger";
    (runBtn as HTMLButtonElement).disabled = false;
  } else {
    runBtn.textContent = "Ejecutar";
    runBtn.className = "btn-run";
    (runBtn as HTMLButtonElement).disabled = false;
  }
  if (!isNewProject) {
    (deleteBtn as HTMLButtonElement).disabled = running;
    (saveBtn as HTMLButtonElement).disabled = running || !hasUnsavedChanges();
  }
}

async function loadProjectList(): Promise<void> {
  try {
    const configs: SavedConfig[] = await invoke("list_configs");
    projectList.innerHTML = configs
      .map(
        (c) => `
        <div class="project-item" data-id="${c.id}">
          <span class="project-name">${escapeHtml(c.name)}</span>
          <span class="project-date">${c.id.split("-").slice(-2).join("-")}</span>
        </div>
      `
      )
      .join("");

    document.querySelectorAll(".project-item").forEach((el) => {
      el.addEventListener("click", () => {
        const id = el.getAttribute("data-id")!;
        loadConfig(id);
      });
    });
  } catch (e) {
    appendLog("error", `Error al cargar proyectos: ${e}`);
  }
}

async function loadConfig(id: string): Promise<void> {
  // Save current log before switching
  const currentKey = currentId ?? "unsaved";
  if (projectLogOutput.innerHTML) {
    projectLogs.set(currentKey, projectLogOutput.innerHTML);
  }

  try {
    const configs: SavedConfig[] = await invoke("list_configs");
    const found = configs.find((c) => c.id === id);
    if (!found) return;

    currentId = found.id;
    currentStarted = found.started;
    projectName.value = found.name;
    fillForm(found.config);
    highlightSelected(id);
    await invoke("set_last_selected", { id });

    projectLogOutput.innerHTML = projectLogs.get(id) ?? "";

    updateUI();
  } catch (e) {
    appendLog("error", `Error al cargar configuración: ${e}`);
  }
}

async function loadLastSelected(): Promise<void> {
  try {
    const last: SavedConfig | null = await invoke("get_last_selected");
    if (last) {
      currentId = last.id;
      currentStarted = last.started;
      projectName.value = last.name;
      fillForm(last.config);
      highlightSelected(last.id);
      updateUI();
    }
  } catch (e) {
    appendLog("error", `Error al cargar última selección: ${e}`);
  }
}

function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function confirmModal(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    modalMessage.textContent = message;
    modalOverlay.classList.remove("hidden");

    function cleanup() {
      modalOverlay.classList.add("hidden");
      modalCancelBtn.removeEventListener("click", onCancel);
      modalConfirmBtn.removeEventListener("click", onConfirm);
    }

    function onCancel() {
      cleanup();
      resolve(false);
    }

    function onConfirm() {
      cleanup();
      resolve(true);
    }

    modalCancelBtn.addEventListener("click", onCancel);
    modalConfirmBtn.addEventListener("click", onConfirm);
  });
}

function appendLog(kind: string, message: string): void {
  const entry = document.createElement("div");
  entry.className = `log-entry log-${kind}`;

  const now = new Date();
  const time = now.toLocaleTimeString("es-ES", { hour12: false });
  entry.textContent = `[${time}] ${message}`;

  projectLogOutput.appendChild(entry);
  projectLogOutput.scrollTop = projectLogOutput.scrollHeight;

  const key = currentId ?? "unsaved";
  projectLogs.set(key, projectLogOutput.innerHTML);
}

// Event listeners

// Guardar: only available for saved projects (has currentId)
form.addEventListener("submit", async (e) => {
  e.preventDefault();
  if (isRunning) return;
  if (currentId === null) return;

  const name = projectName.value.trim();
  if (!name) {
    appendLog("warn", "El nombre del proyecto es obligatorio");
    return;
  }
  try {
    const config = readForm();
    const saved: SavedConfig = await invoke("save_config", {
      name,
      config,
      started: currentStarted,
    });
    currentId = saved.id;
    currentStarted = saved.started;
    updateSnapshot(name, config);
    appendLog("info", `Configuración "${saved.name}" guardada`);
    await loadProjectList();
    highlightSelected(saved.id);
  } catch (err) {
    appendLog("error", `Error al guardar: ${err}`);
  }
});

// Ejecutar / Cancelar
runBtn.addEventListener("click", async () => {
  if (isRunning) {
    try {
      appendLog("info", "Cancelando scraping...");
      await invoke("cancel_scraping");
      appendLog("info", "Cancelación solicitada");
    } catch (err) {
      appendLog("error", `Error al cancelar: ${err}`);
    }
    return;
  }

  const name = projectName.value.trim();
  if (!name) {
    appendLog("warn", "El nombre del proyecto es obligatorio");
    return;
  }

  try {
    const config = readForm();
    const saved: SavedConfig = await invoke("save_config", {
      name,
      config,
      started: true,
    });
    currentId = saved.id;
    currentStarted = true;
    updateSnapshot(name, config);
    await loadProjectList();
    highlightSelected(saved.id);
    updateUI();

    if (projectLogs.has("unsaved")) {
      projectLogs.set(saved.id, projectLogs.get("unsaved")!);
      projectLogs.delete("unsaved");
    }

    appendLog("info", "Iniciando scraping...");
    isRunning = true;
    updateUI();

    const promise = invoke("run_scraping", { config });
    promise.catch((e: unknown) => appendLog("error", `Error: ${e}`));
    promise.finally(() => {
      isRunning = false;
      updateUI();
    });
    appendLog("info", "Scraping lanzado");
  } catch (err) {
    isRunning = false;
    updateUI();
    appendLog("error", `Error al ejecutar: ${err}`);
  }
});

deleteBtn.addEventListener("click", async () => {
  if (isRunning) {
    appendLog("warn", "No se puede eliminar mientras se ejecuta");
    return;
  }
  if (!currentId) {
    appendLog("warn", "No hay ninguna configuración seleccionada");
    return;
  }
  if (!(await confirmModal("¿Eliminar esta configuración?"))) return;
  try {
    await invoke("delete_config", { id: currentId });
    projectLogs.delete(currentId);
    appendLog("info", "Configuración eliminada");
    resetForm();
    await loadProjectList();
  } catch (err) {
    appendLog("error", `Error al eliminar: ${err}`);
  }
});

clearProjectLogBtn.addEventListener("click", () => {
  projectLogOutput.innerHTML = "";
  const key = currentId ?? "unsaved";
  projectLogs.delete(key);
});

newConfigBtn.addEventListener("click", () => {
  resetForm();
  projectName.focus();
});

// Set up event listener for verboser events
async function setupVerboserListener(): Promise<void> {
  if (unlisten) unlisten();
  unlisten = await listen<VerboserPayload>("verboser-event", (event) => {
    appendLog(event.payload.kind, event.payload.message);
  });
}

// Re-evaluate UI on any form change (enables/disables save button)
form.addEventListener("input", () => {
  updateUI();
});

// ── Window controls ──
function setupWindowControls(): void {
  const appWindow = getCurrentWindow();

  document.querySelector(".title-drag")!.addEventListener("mousedown", (e: Event) => {
    const me = e as MouseEvent;
    if (me.button !== 0) return;
    const startX = me.screenX;
    const startY = me.screenY;

    const onMouseMove = (e2: MouseEvent) => {
      if (Math.abs(e2.screenX - startX) > 3 || Math.abs(e2.screenY - startY) > 3) {
        cleanup();
        appWindow.startDragging();
      }
    };
    const onMouseUp = () => cleanup();
    const cleanup = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  });

  document.getElementById("minimize-btn")!.addEventListener("click", () => {
    appWindow.minimize();
  });

  const maximizeBtn = document.getElementById("maximize-btn")!;
  maximizeBtn.addEventListener("click", () => {
    appWindow.toggleMaximize();
  });

  document.getElementById("close-btn")!.addEventListener("click", () => {
    appWindow.close();
  });

  appWindow.onResized(() => {
    appWindow.isMaximized().then((maximized) => {
      maximizeBtn.classList.toggle("is-restore", maximized);
      maximizeBtn.title = maximized ? "Restaurar" : "Maximizar";
      const svg = maximizeBtn.querySelector("svg")!;
      if (maximized) {
        svg.innerHTML = '<rect x="2.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/><rect x="0.5" y="2.5" width="9" height="9" fill="var(--card-bg)" stroke="currentColor" stroke-width="1"/>';
      } else {
        svg.innerHTML = '<rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/>';
      }
    });
  });
}

// Init
async function init(): Promise<void> {
  setupWindowControls();
  await setupVerboserListener();
  await loadProjectList();
  await loadLastSelected();
  updateUI();
}

init().catch((e) => appendLog("error", `Error de inicialización: ${e}`));
