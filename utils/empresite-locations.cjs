#!/usr/bin/env node
"use strict";

// Rastrea provincias y localidades de Empresite (actividad MANTENIMIENTOS)
// usando el panel del boton "Ubicacion" y genera utils/empresite_locations.json.
//
// Reanudable: si el JSON ya existe, omite las provincias que ya tienen
// localidades. Pensado para ejecutarse con Chrome visible: cuando aparece el
// reCAPTCHA (HTTP 429 "Demasiadas peticiones detectadas") hay que resolverlo
// manualmente en la ventana y el script continua solo.

const fs = require("fs");
const os = require("os");
const path = require("path");

const BASE = "https://empresite.eleconomista.es/Actividad/MANTENIMIENTOS/";
const OUT = process.argv[2] || path.join(__dirname, "empresite_locations.json");
const PROFILE = path.join(os.homedir(), ".claude", "tmp", "empresite-playwright-profile");

const DELAY_MIN_MS = 10000;
const DELAY_MAX_MS = 18000;
const CAPTCHA_TIMEOUT_MS = 30 * 60 * 1000;

// Etiqueta canonica por slug de provincia (rustcrape-gui/src/lib/types.ts).
const PROVINCE_LABELS = {
  CORUNA: "A Coruña",
  ALAVA: "Álava",
  ALBACETE: "Albacete",
  ALICANTE: "Alicante",
  ALMERIA: "Almería",
  ASTURIAS: "Asturias",
  AVILA: "Ávila",
  BADAJOZ: "Badajoz",
  BALEARES: "Baleares",
  BARCELONA: "Barcelona",
  BURGOS: "Burgos",
  CACERES: "Cáceres",
  CADIZ: "Cádiz",
  CANTABRIA: "Cantabria",
  CASTELLON: "Castellón",
  CEUTA: "Ceuta",
  "CIUDAD-REAL": "Ciudad Real",
  CORDOBA: "Córdoba",
  CUENCA: "Cuenca",
  GERONA: "Girona",
  GRANADA: "Granada",
  GUADALAJARA: "Guadalajara",
  GUIPUZCOA: "Guipúzcoa",
  HUELVA: "Huelva",
  HUESCA: "Huesca",
  JAEN: "Jaén",
  LEON: "León",
  LERIDA: "Lleida",
  LUGO: "Lugo",
  MADRID: "Madrid",
  MALAGA: "Málaga",
  MELILLA: "Melilla",
  MURCIA: "Murcia",
  NAVARRA: "Navarra",
  ORENSE: "Ourense",
  PALENCIA: "Palencia",
  PALMAS: "Las Palmas",
  PONTEVEDRA: "Pontevedra",
  RIOJA: "La Rioja",
  SALAMANCA: "Salamanca",
  "SANTA-CRUZ-TENERIFE": "Santa Cruz de Tenerife",
  SEGOVIA: "Segovia",
  SEVILLA: "Sevilla",
  SORIA: "Soria",
  TARRAGONA: "Tarragona",
  TERUEL: "Teruel",
  TOLEDO: "Toledo",
  VALENCIA: "Valencia",
  VALLADOLID: "Valladolid",
  VIZCAYA: "Vizcaya",
  ZAMORA: "Zamora",
  ZARAGOZA: "Zaragoza",
};

function log(...args) {
  console.log(new Date().toISOString().slice(11, 19), ...args);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function randomDelay() {
  return DELAY_MIN_MS + Math.floor(Math.random() * (DELAY_MAX_MS - DELAY_MIN_MS));
}

// Localiza el paquete playwright en node_modules o en la cache de npx.
function loadPlaywright() {
  const candidates = ["playwright"];
  const npx = path.join(os.homedir(), "AppData", "Local", "npm-cache", "_npx");
  if (fs.existsSync(npx)) {
    for (const dir of fs.readdirSync(npx)) {
      const p = path.join(npx, dir, "node_modules", "playwright");
      if (fs.existsSync(path.join(p, "package.json"))) candidates.push(p);
    }
  }
  for (const c of candidates) {
    try {
      return require(c);
    } catch (_) {
      // se prueba el siguiente candidato
    }
  }
  throw new Error("No se encontró el paquete 'playwright'. Instálalo con: npm i playwright");
}

function findChrome() {
  const paths = [
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  ];
  return paths.find((p) => fs.existsSync(p)) || null;
}

// Quita el conteo final del texto del enlace: "Arona (307 )" -> "Arona".
function cleanName(text) {
  return String(text)
    .replace(/\s*\([^()]*\)\s*$/, "")
    .replace(/\s+/g, " ")
    .trim();
}

function readResults() {
  if (!fs.existsSync(OUT)) return [];
  try {
    const data = JSON.parse(fs.readFileSync(OUT, "utf8"));
    return Array.isArray(data) ? data : [];
  } catch (_) {
    return [];
  }
}

function writeResults(list) {
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, JSON.stringify(list, null, 4) + "\n", "utf8");
}

// Espera a que la pagina este lista: ni bloqueada por captcha ni sin panel.
async function waitReady(page, kind, label) {
  const start = Date.now();
  let warned = false;
  while (Date.now() - start < CAPTCHA_TIMEOUT_MS) {
    const state = await page
      .evaluate((k) => {
        const blocked = /demasiadas peticiones/i.test(document.body ? document.body.innerText : "");
        const panel = document.getElementById("layoutFilterTagUbicacion");
        // Hay provincias sin localidades (Ceuta, Melilla) que no muestran el
        // panel de Ubicacion; basta con que el listado este cargado.
        const loaded =
          !!panel ||
          !!document.getElementById("filters-headers") ||
          !!document.getElementById("flex-resultado-busqueda");
        const n = panel ? panel.querySelectorAll('a[href*="/' + k + '/"]').length : 0;
        return { blocked, loaded, n };
      }, kind)
      .catch(() => ({ blocked: false, loaded: false, n: 0 }));

    if (!state.blocked && state.loaded) return state.n;
    if (!warned) {
      log(`[!] Bloqueo/captcha en ${label}. Resuélvelo en la ventana del navegador...`);
      warned = true;
    }
    await page.waitForTimeout(3000);
  }
  throw new Error(`Timeout esperando ${label}: captcha no resuelto`);
}

async function extractLinks(page, kind) {
  return page.evaluate((k) => {
    const panel = document.getElementById("layoutFilterTagUbicacion");
    if (!panel) return [];
    const re = new RegExp("/" + k + "/([^/]+)/?");
    const seen = new Set();
    const out = [];
    for (const a of panel.querySelectorAll('a[href*="/' + k + '/"]')) {
      const href = a.getAttribute("href") || "";
      const match = href.match(re);
      if (!match) continue;
      const id = decodeURIComponent(match[1]).replace(/\/$/, "");
      if (seen.has(id)) continue;
      seen.add(id);
      out.push({ id, name: (a.textContent || "").replace(/\s+/g, " ").trim() });
    }
    return out;
  }, kind);
}

async function main() {
  const { chromium } = loadPlaywright();
  fs.mkdirSync(PROFILE, { recursive: true });

  const launchOptions = {
    headless: false,
    viewport: { width: 1366, height: 900 },
    locale: "es-ES",
    args: ["--disable-blink-features=AutomationControlled"],
  };
  const chrome = findChrome();
  if (chrome) launchOptions.executablePath = chrome;
  else launchOptions.channel = "chrome";

  log(`Salida: ${OUT}`);
  const ctx = await chromium.launchPersistentContext(PROFILE, launchOptions);
  const page = ctx.pages()[0] || (await ctx.newPage());

  try {
    log("Cargando página base...");
    await page.goto(BASE, { waitUntil: "domcontentloaded", timeout: 60000 });
    await waitReady(page, "provincia", "página base");

    const provinces = await extractLinks(page, "provincia");
    log(`Provincias detectadas: ${provinces.length}`);
    if (provinces.length === 0) throw new Error("No se detectaron provincias");

    const existing = readResults();
    const byId = new Map(existing.map((p) => [p.id, p]));
    const results = [];

    for (let i = 0; i < provinces.length; i++) {
      const prov = provinces[i];
      const done = byId.get(prov.id);
      if (done && Array.isArray(done.towns) && done.towns.length > 0) {
        log(`[${i + 1}/${provinces.length}] ${prov.id}: ya en JSON (${done.towns.length}), omito`);
        results.push(done);
        continue;
      }

      await sleep(randomDelay());
      const url = `${BASE}provincia/${prov.id}/`;
      log(`[${i + 1}/${provinces.length}] ${prov.id}: cargando ${url}`);

      let towns = null;
      for (let attempt = 1; attempt <= 3 && towns === null; attempt++) {
        try {
          await page.goto(url, { waitUntil: "domcontentloaded", timeout: 60000 });
          await waitReady(page, "localidad", `provincia ${prov.id}`);
          const links = await extractLinks(page, "localidad");
          towns = links.map((l) => ({ name: cleanName(l.name), id: l.id }));
        } catch (err) {
          log(`   intento ${attempt} fallido: ${err.message}`);
          if (attempt < 3) await sleep(randomDelay());
        }
      }
      if (towns === null) towns = [];

      const entry = {
        name: PROVINCE_LABELS[prov.id] || cleanName(prov.name),
        id: prov.id,
        towns,
      };
      results.push(entry);
      byId.set(prov.id, entry);
      writeResults(results);
      log(`   ${prov.id}: ${towns.length} localidades guardadas`);
    }

    writeResults(results);
    const total = results.reduce((sum, p) => sum + p.towns.length, 0);
    log(`Hecho. ${results.length} provincias, ${total} localidades -> ${OUT}`);
  } finally {
    await ctx.close().catch(() => {});
  }
}

main().catch((err) => {
  console.error("ERROR:", err.message);
  process.exit(1);
});
