import { chromium, firefox, type Page, type Browser } from 'playwright';
import { ensureBrowser } from './browser-manager.js';

export let currentBrowser: Browser | null = null;

async function extraerUnica(page: Page): Promise<Tintoreria[]> {
  await page.waitForFunction(() => {
    return (
      document.querySelector('button[aria-label*="Teléfono"]') !== null ||
      document.querySelector('[data-item-id*="phone"]') !== null ||
      document.querySelector('a[aria-label*="Sitio web"]') !== null
    );
  }, { timeout: 10000 }).catch(() => {});

  const url = page.url();
  const match = url.match(/\/maps\/place\/([^/]+)/);
  if (!match?.[1]) return [];
  const nombre = decodeURIComponent(match[1].replace(/\+/g, ' '));

  const tfno = await page.evaluate(() => {
    const sel = document.querySelector(
      'button[data-tooltip*="teléfono"], button[data-tooltip*="phone"], button[aria-label*="Teléfono"], [data-item-id*="phone"]'
    );
    if (!sel) return null;
    const aria = sel.getAttribute('aria-label');
    if (aria) return aria.replace(/^teléfono:?\s*/i, '').trim();
    return sel.textContent?.trim() || null;
  });

  const email = await page.evaluate(() => {
    const a = document.querySelector('a[href^="mailto:"]');
    return a?.getAttribute('href')?.replace('mailto:', '') || null;
  });

  const web = await page.evaluate(() => {
    const sel = document.querySelector(
      'button[data-tooltip*="sitio web"], button[data-tooltip*="website"], [data-item-id*="authority"], a[aria-label*="Sitio web"], a[aria-label*="sitio web"]'
    );
    if (!sel) return null;
    const aria = sel.getAttribute('aria-label');
    if (aria) return aria.replace(/^sitio web:?\s*/i, '').trim();
    const link = sel.tagName === 'A' ? sel : sel.querySelector('a[href]');
    return link?.getAttribute('href') || null;
  });

  return [{ nombre, email, web, tfno, maps_url: limpiarUrlMaps(url) }];
}

export interface Tintoreria {
  nombre: string;
  email: string | null;
  web: string | null;
  tfno: string | null;
  maps_url: string;
}

export interface SearchConfig {
  lat: number;
  lng: number;
  zoom: number;
  stopThreshold: number;
  searchQuery: string;
  headless?: boolean;
  delayMin?: number;
  delayMax?: number;
}

const VIEWPORTS = [
  { width: 1366, height: 768 },
  { width: 1440, height: 900 },
  { width: 1536, height: 864 },
  { width: 1920, height: 1080 },
  { width: 1280, height: 800 },
  { width: 1600, height: 900 },
];

const USER_AGENTS = [
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36',
];

function limpiarUrlMaps(url: string): string {
  return url.replace(/@-?\d+(?:\.\d+)?,-?\d+(?:\.\d+)?,\d+(?:\.\d+)?z\/?/, '');
}

function rand(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

export async function buscar(config: SearchConfig, onStatus?: (msg: string) => void): Promise<Tintoreria[]> {
  const tintorerias: Tintoreria[] = [];
  const viewport = VIEWPORTS[rand(0, VIEWPORTS.length - 1)]!;
  const userAgent = USER_AGENTS[rand(0, USER_AGENTS.length - 1)]!;

  const browserInfo = await ensureBrowser();
  const dMin = config.delayMin ?? 0;
  const dMax = config.delayMax ?? 1400;

  const launchOpts: Record<string, unknown> = {
    headless: config.headless ?? false,
    executablePath: browserInfo.executablePath,
  };
  if (browserInfo.type === 'firefox') {
    launchOpts.args = ['-disable-blink-features=AutomationControlled'];
  } else {
    launchOpts.args = [
      '--disable-blink-features=AutomationControlled',
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-infobars',
    ];
  }

  const launcher = browserInfo.type === 'firefox' ? firefox : chromium;
  const browser = await launcher.launch(launchOpts);

  currentBrowser = browser;

  const ctx = await browser.newContext({ viewport, locale: 'es-ES', userAgent });
  const page = await ctx.newPage();

  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => false });
  });

  try {
    onStatus?.('Navegando a Google Maps...');
    await page.goto(
      `https://www.google.com/maps/search/${encodeURIComponent(config.searchQuery)}/@${config.lat},${config.lng},${config.zoom}z`,
      { waitUntil: 'domcontentloaded' }
    );
    onStatus?.('Aceptando cookies...');
    await page.waitForSelector('button:has-text("Aceptar todo")', { timeout: 10000 });
    await page.getByRole('button', { name: /aceptar todo/i }).click();
    await page.waitForURL(/.*google\.com\/maps.*/, { timeout: 10000 });

    onStatus?.('Buscando resultados...');
    await page.waitForFunction(() => {
      return (
        document.querySelector('[role="feed"]') !== null ||
        window.location.href.includes('/maps/place/')
      );
    }, { timeout: 10000 }).catch(() => {});

    const hasFeed = await page.locator('[role="feed"]').isVisible().catch(() => false);
    if (!hasFeed) {
      onStatus?.('Extrayendo datos de pagina de detalle...');
      return await extraerUnica(page);
    }

    const radio = 180 / Math.pow(2, config.zoom);
    let k = 0;
    let fuera = 0;
    let scrollsSinNuevos = 0;

    while (true) {
      const total = await page.locator('a[href*="/maps/place"]').count();

      if (total <= k) {
        scrollsSinNuevos++;
        if (scrollsSinNuevos >= 5) {
          return tintorerias;
        }
      } else {
        scrollsSinNuevos = 0;
      }

      for (let i = k; i < total; i++) {
        const href = await page.locator('a[href*="/maps/place"]').nth(i).getAttribute('href');
        if (!href) continue;

        const nombre = decodeURIComponent(
          href.match(/\/maps\/place\/([^/]+)/)?.[1]?.replace(/\+/g, ' ') || ''
        );
        onStatus?.(`Procesando ${nombre}...`);

        const lat = href.match(/[!&]3d(-?\d+(?:\.\d+)?)/)?.[1];
        const lng = href.match(/[!&]4d(-?\d+(?:\.\d+)?)/)?.[1];
        if (!nombre || !lat || !lng) continue;

        const distLat = Math.abs(parseFloat(lat) - config.lat);
        const distLng = Math.abs(parseFloat(lng) - config.lng);
        const dist = Math.max(distLat, distLng);

        if (dist > radio) {
          fuera++;
          if (fuera >= config.stopThreshold) {
            return tintorerias;
          }
        } else {
          fuera = 0;
        }

        await page.locator('a[href*="/maps/place"]').nth(i).click({ force: true, timeout: 5000 });
        await new Promise(r => setTimeout(r, rand(dMin, dMax)))

        try {
          await page.waitForSelector(
            'button[data-tooltip*="teléfono"], button[data-tooltip*="phone"], button[aria-label*="Teléfono"], button[data-tooltip*="sitio web"], button[data-tooltip*="website"]',
            { timeout: 3000 }
          );
        } catch {}

        const tfno = await page.evaluate(() => {
          const sel = document.querySelector(
            'button[data-tooltip*="teléfono"], button[data-tooltip*="phone"], button[aria-label*="Teléfono"], [data-item-id*="phone"]'
          );
          if (!sel) return null;
          const aria = sel.getAttribute('aria-label');
          if (aria) return aria.replace(/^teléfono:?\s*/i, '').trim();
          return sel.textContent?.trim() || null;
        });

        const email = await page.evaluate(() => {
          const a = document.querySelector('a[href^="mailto:"]');
          return a?.getAttribute('href')?.replace('mailto:', '') || null;
        });

        const web = await page.evaluate(() => {
          const sel = document.querySelector(
            'button[data-tooltip*="sitio web"], button[data-tooltip*="website"], [data-item-id*="authority"]'
          );
          if (!sel) return null;
          const aria = sel.getAttribute('aria-label');
          if (aria) return aria.replace(/^sitio web:?\s*/i, '').trim();
          const link = sel.tagName === 'A' ? sel : sel.querySelector('a[href]');
          return link?.getAttribute('href') || null;
        });

        const mapsUrl = href.startsWith('http') ? href : `https://www.google.com${href}`;
        tintorerias.push({ nombre, email, web, tfno, maps_url: limpiarUrlMaps(mapsUrl) });
      }

      k = total;

      onStatus?.('Haciendo scrolling...');
      await page.evaluate(() => {
        const feed = document.querySelector('[role="feed"]');
        if (feed) {
          feed.scrollBy(0, 30 + Math.random() * 400);
        }
      });

      onStatus?.('Cargando mas resultados...');
      try {
        await page.waitForFunction(
          (prev) => document.querySelectorAll('a[href*="/maps/place"]').length > prev,
          total,
          { timeout: 10000 }
        );
      } catch {
        const fin = await page.evaluate(() => {
          const feed = document.querySelector('[role="feed"]');
          if (!feed) return true;
          return feed.scrollHeight - feed.scrollTop - feed.clientHeight < 50;
        });
        if (fin) return tintorerias;
      }
    }
  } finally {
    await cerrarNavegador();
  }
}

// Cierra el navegador lanzado por playwright.
export async function cerrarNavegador(): Promise<void> {
  try {
    await currentBrowser?.close();
  } catch {}
  currentBrowser = null;
}

