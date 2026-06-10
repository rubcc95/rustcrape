import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

function log(message: string, type: "info" | "error" | "done" = "info") {
  const logBox = document.getElementById("log");
  if (!logBox) return;
  const entry = document.createElement("div");
  entry.className = `log-entry ${type}`;
  entry.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
  logBox.appendChild(entry);
  logBox.scrollTop = logBox.scrollHeight;
}

function getVal(id: string): string {
  return (document.getElementById(id) as HTMLInputElement)?.value ?? "";
}

function setVal(id: string, val: string) {
  const el = document.getElementById(id) as HTMLInputElement;
  if (el) el.value = val;
}

async function loadConfig() {
  try {
    const cfg: any = await invoke("get_config");
    setVal("search-term", cfg.search_term);
    setVal("zoom", String(cfg.zoom));
    setVal("db-type", cfg.db_type.toLowerCase());
    setVal("mysql-host", cfg.mysql_host);
    setVal("mysql-port", String(cfg.mysql_port));
    setVal("mysql-user", cfg.mysql_user);
    setVal("mysql-pass", cfg.mysql_password);
    setVal("mysql-db", cfg.mysql_database);
    setVal("sqlite-path", cfg.sqlite_path);
    setVal("iterations", String(cfg.iterations));
    if (cfg.user_agents?.length) {
      setVal("user-agents", cfg.user_agents.join("\n"));
    }
    toggleDbFields();
    log("Configuracion cargada", "info");
  } catch (e) {
    log(`Error cargando config: ${e}`, "error");
  }
}

async function saveConfig() {
  const cfg: any = {
    search_term: getVal("search-term"),
    zoom: parseInt(getVal("zoom")) || 12,
    db_type: getVal("db-type") === "mysql" ? "Mysql" : "Sqlite",
    mysql_host: getVal("mysql-host"),
    mysql_port: parseInt(getVal("mysql-port")) || 3306,
    mysql_user: getVal("mysql-user"),
    mysql_password: getVal("mysql-pass"),
    mysql_database: getVal("mysql-db"),
    sqlite_path: getVal("sqlite-path"),
    browser_path: null,
    user_agents: getVal("user-agents").split("\n").filter((l) => l.trim()),
    iterations: parseInt(getVal("iterations")) || 0,
  };
  const browserSelect = document.getElementById("browser-select") as HTMLSelectElement;
  if (browserSelect.value) {
    cfg.browser_path = browserSelect.value;
  }
  try {
    await invoke("save_config", { config: cfg });
    log("Configuracion guardada", "done");
  } catch (e) {
    log(`Error guardando config: ${e}`, "error");
  }
}

function toggleDbFields() {
  const dbType = getVal("db-type");
  const mysqlFields = document.getElementById("mysql-fields");
  const sqliteFields = document.getElementById("sqlite-fields");
  if (mysqlFields && sqliteFields) {
    mysqlFields.style.display = dbType === "mysql" ? "block" : "none";
    sqliteFields.style.display = dbType === "sqlite" ? "block" : "none";
  }
}

async function detectBrowsers() {
  try {
    const browsers: any[] = await invoke("detect_browsers");
    const select = document.getElementById("browser-select") as HTMLSelectElement;
    if (!select) return;
    select.innerHTML = '<option value="">Auto (Playwright Chromium)</option>';
    for (const b of browsers) {
      const opt = document.createElement("option");
      opt.value = b.path;
      opt.textContent = `${b.name} (${b.path})`;
      select.appendChild(opt);
    }
    log(`Detectados ${browsers.length} navegadores`, "info");
  } catch (e) {
    log(`Error detectando navegadores: ${e}`, "error");
  }
}

async function generateGrid() {
  try {
    const zoom = parseInt(getVal("zoom")) || 12;
    const count: number = await invoke("generate_grid", { zoom });
    log(`Cuadrantes generados: ${count}`, "done");
  } catch (e) {
    log(`Error generando cuadrantes: ${e}`, "error");
  }
}

async function startScraping() {
  try {
    const startBtn = document.getElementById("start-btn") as HTMLButtonElement;
    const stopBtn = document.getElementById("stop-btn") as HTMLButtonElement;
    if (startBtn) startBtn.disabled = true;
    if (stopBtn) stopBtn.disabled = false;
    await invoke("start_scraping");
    log("Scraping iniciado", "info");
  } catch (e) {
    log(`Error iniciando scraping: ${e}`, "error");
    const startBtn = document.getElementById("start-btn") as HTMLButtonElement;
    const stopBtn = document.getElementById("stop-btn") as HTMLButtonElement;
    if (startBtn) startBtn.disabled = false;
    if (stopBtn) stopBtn.disabled = true;
  }
}

async function stopScraping() {
  try {
    await invoke("stop_scraping");
    log("Scraping detenido", "info");
  } catch (e) {
    log(`Error deteniendo scraping: ${e}`, "error");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  loadConfig();

  document.getElementById("db-type")?.addEventListener("change", toggleDbFields);
  document.getElementById("save-config-btn")?.addEventListener("click", saveConfig);
  document.getElementById("detect-browsers-btn")?.addEventListener("click", detectBrowsers);
  document.getElementById("generate-grid-btn")?.addEventListener("click", generateGrid);
  document.getElementById("start-btn")?.addEventListener("click", startScraping);
  document.getElementById("stop-btn")?.addEventListener("click", stopScraping);

  listen<any>("progress", (event) => {
    const count = event.payload;
    const el = document.getElementById("progress-count");
    if (el) el.textContent = String(count);
    log(`Progreso: ${count} iteraciones`, "info");
  });

  listen<any>("error", (event) => {
    log(`Error: ${event.payload}`, "error");
    const startBtn = document.getElementById("start-btn") as HTMLButtonElement;
    const stopBtn = document.getElementById("stop-btn") as HTMLButtonElement;
    if (startBtn) startBtn.disabled = false;
    if (stopBtn) stopBtn.disabled = true;
  });

  listen<any>("done", (event) => {
    log(`Finalizado: ${event.payload}`, "done");
    const startBtn = document.getElementById("start-btn") as HTMLButtonElement;
    const stopBtn = document.getElementById("stop-btn") as HTMLButtonElement;
    if (startBtn) startBtn.disabled = false;
    if (stopBtn) stopBtn.disabled = true;
  });

  listen<any>("grid_done", (event) => {
    log(`Cuadrantes generados: ${event.payload}`, "done");
  });
});
