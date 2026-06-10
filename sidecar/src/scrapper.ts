import { chromium, type Page, type Browser } from "playwright";

export interface SearchConfig {
  lat: number;
  lng: number;
  zoom: number;
  searchTerm: string;
  browserPath?: string;
  userAgent?: string;
  stopThreshold: number;
}

export interface ScrapeResult {
  nombre: string;
  email: string;
  web: string;
  tfno: string;
  maps_url: string;
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
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
];

function rand(min: number, max: number): number {
  return Math.floor(Math.random() * (max - min + 1)) + min;
}

function limpiarUrlMaps(url: string): string {
  return url.replace(/@-?\d+(?:\.\d+)?,-?\d+(?:\.\d+)?,\d+(?:\.\d+)?z\/?/, "");
}

async function extraerUnica(page: Page): Promise<ScrapeResult[]> {
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
  const nombre = decodeURIComponent(match[1].replace(/\+/g, " "));

  const tfno = await page.evaluate(() => {
    const sel = document.querySelector(
      'button[data-tooltip*="teléfono"], button[data-tooltip*="phone"], button[aria-label*="Teléfono"], [data-item-id*="phone"]'
    );
    if (!sel) return null;
    const aria = sel.getAttribute("aria-label");
    if (aria) return aria.replace(/^teléfono:?\s*/i, "").trim();
    return sel.textContent?.trim() || null;
  });

  const email = await page.evaluate(() => {
    const a = document.querySelector('a[href^="mailto:"]');
    return a?.getAttribute("href")?.replace("mailto:", "") || null;
  });

  const web = await page.evaluate(() => {
    const sel = document.querySelector(
      'button[data-tooltip*="sitio web"], button[data-tooltip*="website"], [data-item-id*="authority"], a[aria-label*="Sitio web"], a[aria-label*="sitio web"]'
    );
    if (!sel) return null;
    const aria = sel.getAttribute("aria-label");
    if (aria) return aria.replace(/^sitio web:?\s*/i, "").trim();
    const link = sel.tagName === "A" ? sel : sel.querySelector("a[href]");
    return link?.getAttribute("href") || null;
  });

  return [{ nombre, email: email || "", web: web || "", tfno: tfno || "", maps_url: limpiarUrlMaps(url) }];
}


export async function buscar(config: SearchConfig): Promise<ScrapeResult[]> {
  const res: ScrapeResult[] = [];
  const viewport = VIEWPORTS[rand(0, VIEWPORTS.length - 1)]!;
  const userAgent = USER_AGENTS[rand(0, USER_AGENTS.length - 1)]!;

  const browser = await chromium.launch({
    headless: false,
    args: [
      '--disable-blink-features=AutomationControlled',
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-infobars',
    ],
  });

  const ctx = await browser.newContext({ viewport, locale: 'es-ES', userAgent });
  const page = await ctx.newPage();

  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => false });
  });

  try {
    await page.goto(
      `https://www.google.com/maps/search/${config.searchTerm}/@${config.lat},${config.lng},${config.zoom}z`,
      { waitUntil: 'domcontentloaded' }
    );
    await page.waitForSelector('button:has-text("Aceptar todo")', { timeout: 10000 });
    await page.getByRole('button', { name: /aceptar todo/i }).click();
    await page.waitForURL(/.*google\.com\/maps.*/, { timeout: 10000 });

    await page.waitForFunction(() => {
      return (
        document.querySelector('[role="feed"]') !== null ||
        window.location.href.includes('/maps/place/')
      );
    }, { timeout: 25000 }).catch(() => {});

    const hasFeed = await page.locator('[role="feed"]').isVisible().catch(() => false);
    if (!hasFeed) {
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
          return res;
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

        const lat = href.match(/[!&]3d(-?\d+(?:\.\d+)?)/)?.[1];
        const lng = href.match(/[!&]4d(-?\d+(?:\.\d+)?)/)?.[1];
        if (!nombre || !lat || !lng) continue;

        const distLat = Math.abs(parseFloat(lat) - config.lat);
        const distLng = Math.abs(parseFloat(lng) - config.lng);
        const dist = Math.max(distLat, distLng);

        if (dist > radio) {
          fuera++;
          if (fuera >= config.stopThreshold) {
            return res;
          }
        } else {
          fuera = 0;
        }

        await page.locator('a[href*="/maps/place"]').nth(i).click({ force: true, timeout: 5000 }).catch(() => {});
        await new Promise(r => setTimeout(r, rand(10, 1400)))

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
        }) ?? '';

        const email = await page.evaluate(() => {
          const a = document.querySelector('a[href^="mailto:"]');
          return a?.getAttribute('href')?.replace('mailto:', '') || null;
        }) ?? '';

        const web = await page.evaluate(() => {
          const sel = document.querySelector(
            'button[data-tooltip*="sitio web"], button[data-tooltip*="website"], [data-item-id*="authority"]'
          );
          if (!sel) return null;
          const aria = sel.getAttribute('aria-label');
          if (aria) return aria.replace(/^sitio web:?\s*/i, '').trim();
          const link = sel.tagName === 'A' ? sel : sel.querySelector('a[href]');
          return link?.getAttribute('href') || null;
        }) ?? '';

        const mapsUrl = href.startsWith('http') ? href : `https://www.google.com${href}`;
        res.push({ nombre, email, web, tfno, maps_url: limpiarUrlMaps(mapsUrl) });
      }

      k = total;

      await page.evaluate(() => {
        const feed = document.querySelector('[role="feed"]');
        if (feed) {
          feed.scrollBy(0, 30 + Math.random() * 400);
        }
      });

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
        if (fin) return res;
      }
    }
  } finally {
    await browser.close();
  }
}
