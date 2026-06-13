import { existsSync, mkdirSync, writeFileSync, readdirSync, rmSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { tmpdir } from 'node:os';
import { execSync } from 'node:child_process';
import cliProgress from 'cli-progress';

export interface BrowserInfo {
  type: 'chromium' | 'firefox';
  executablePath: string;
  channel?: string;
  name: string;
}

function getBrowsersDir(): string {
  return (
    process.env.SCRAPPER_BROWSERS_DIR ??
    join(
      process.env.LOCALAPPDATA ?? join(process.cwd(), '.browsers'),
      'scrapper_tintorerias',
      'browsers',
    )
  );
}

function getPrefixes(): string[] {
  return [
    process.env.LOCALAPPDATA,
    process.env.PROGRAMFILES,
    process.env['PROGRAMFILES(X86)'],
  ].filter((v): v is string => !!v);
}

function findAllSystemChromium(): BrowserInfo[] {
  const result: BrowserInfo[] = [];
  const channels: Record<string, string> = {
    chrome: 'Google\\Chrome\\Application\\chrome.exe',
    'chrome-beta': 'Google\\Chrome Beta\\Application\\chrome.exe',
    'chrome-dev': 'Google\\Chrome Dev\\Application\\chrome.exe',
    'chrome-canary': 'Google\\Chrome SxS\\Application\\chrome.exe',
    msedge: 'Microsoft\\Edge\\Application\\msedge.exe',
    'msedge-beta': 'Microsoft\\Edge Beta\\Application\\msedge.exe',
    'msedge-dev': 'Microsoft\\Edge Dev\\Application\\msedge.exe',
  };

  for (const [channel, relative] of Object.entries(channels)) {
    for (const prefix of getPrefixes()) {
      const full = join(prefix, relative);
      if (existsSync(full)) {
        result.push({
          type: 'chromium',
          executablePath: full,
          channel,
          name: relative.includes('Edge') ? `Edge (${channel})` : `Chrome (${channel})`,
        });
        break;
      }
    }
  }

  try {
    const out = execSync(
      'where chrome.exe chromium.exe msedge.exe 2>nul',
      { encoding: 'utf8', timeout: 5000 },
    );
    for (const line of out.trim().split(/\r?\n/)) {
      const p = line.trim();
      if (p && existsSync(p)) {
        const already = result.some(r => r.name.includes(dirname(p)));
        if (!already) {
          result.push({ type: 'chromium', executablePath: p, name: `Chromium (${dirname(p)})` });
        }
      }
    }
  } catch {}

  return result;
}

function findAllSystemFirefox(): BrowserInfo[] {
  const result: BrowserInfo[] = [];
  const paths = [
    join(process.env.PROGRAMFILES ?? '', 'Mozilla Firefox', 'firefox.exe'),
    join(process.env['PROGRAMFILES(X86)'] ?? '', 'Mozilla Firefox', 'firefox.exe'),
    join(process.env.LOCALAPPDATA ?? '', 'Mozilla Firefox', 'firefox.exe'),
  ];
  const seen = new Set<string>();
  for (const full of paths) {
    if (existsSync(full) && !seen.has(full)) {
      seen.add(full);
      result.push({ type: 'firefox', executablePath: full, name: 'Firefox' });
    }
  }
  try {
    const out = execSync('where firefox.exe 2>nul', { encoding: 'utf8', timeout: 5000 });
    for (const line of out.trim().split(/\r?\n/)) {
      const p = line.trim();
      if (p && existsSync(p) && !seen.has(p)) {
        seen.add(p);
        result.push({ type: 'firefox', executablePath: p, name: `Firefox (${dirname(p)})` });
      }
    }
  } catch {}
  return result;
}

function findPlaywrightCache(): BrowserInfo | null {
  if (!process.env.LOCALAPPDATA) return null;
  const cacheDir = join(process.env.LOCALAPPDATA, 'ms-playwright');
  if (!existsSync(cacheDir)) return null;

  try {
    for (const entry of readdirSync(cacheDir, { withFileTypes: true })) {
      if (!entry.isDirectory() || !entry.name.startsWith('chromium_')) continue;
      const exe = join(cacheDir, entry.name, 'chrome-win64', 'chrome.exe');
      if (existsSync(exe)) {
        return { type: 'chromium', executablePath: exe, name: `Playwright ${entry.name}` };
      }
    }
  } catch {}

  return null;
}

async function downloadChromium(browsersDir: string): Promise<BrowserInfo> {
  const browserVersion = '148.0.7778.96';
  const chromeDir = join(browsersDir, 'chrome-win64');
  const exePath = join(chromeDir, 'chrome.exe');

  if (existsSync(exePath)) {
    return { type: 'chromium', executablePath: exePath, name: 'Chromium descargado' };
  }

  const url = `https://cdn.playwright.dev/builds/cft/${browserVersion}/win64/chrome-win64.zip`;

  console.log(`\nNo se encontro navegador instalado.`);
  console.log(`Descargando Chromium ${browserVersion}...`);

  const zipPath = join(tmpdir(), `chromium-${Date.now()}.zip`);
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Error HTTP ${resp.status} al descargar Chromium`);
  }

  const totalBytes = parseInt(resp.headers.get('content-length') || '0', 10);
  const totalMB = totalBytes / (1024 * 1024);
  const bar = new cliProgress.SingleBar({
    format: '  [{bar}] {percentage}% | {value}/{total} MB | {speed}',
    barCompleteChar: '\u2588',
    barIncompleteChar: '\u2591',
    hideCursor: true,
  });
  bar.start(Math.round(totalMB), 0, { speed: '0 MB/s' });

  const reader = resp.body!.getReader();
  const chunks: Uint8Array[] = [];
  let downloadedBytes = 0;
  let lastTime = Date.now();
  let lastBytes = 0;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    downloadedBytes += value.length;
    bar.update(Math.round(downloadedBytes / (1024 * 1024)));

    const now = Date.now();
    const elapsed = (now - lastTime) / 1000;
    if (elapsed >= 0.5) {
      const speed = ((downloadedBytes - lastBytes) / elapsed / (1024 * 1024)).toFixed(1);
      bar.update(Math.round(downloadedBytes / (1024 * 1024)), { speed: `${speed} MB/s` });
      lastTime = now;
      lastBytes = downloadedBytes;
    }
  }

  bar.update(Math.round(totalMB), { speed: `${totalMB.toFixed(1)} MB total` });
  bar.stop();

  const buf = Buffer.concat(chunks);
  writeFileSync(zipPath, buf);

  if (!existsSync(chromeDir)) mkdirSync(chromeDir, { recursive: true });

  try {
    execSync(
      `powershell -NoProfile -NonInteractive -Command "& {Expand-Archive -LiteralPath '${zipPath}' -DestinationPath '${browsersDir}' -Force}"`,
      { timeout: 180000, stdio: 'pipe' },
    );
  } catch (e) {
    try { rmSync(zipPath); } catch {}
    throw new Error(
      `No se pudo extraer Chromium. Instalalo manualmente:\n` +
      `1. Descarga: ${url}\n` +
      `2. Extrae en: ${browsersDir}\n` +
      `3. Vuelve a ejecutar`,
    );
  }

  try { rmSync(zipPath); } catch {}

  if (!existsSync(exePath)) {
    throw new Error(`Error: no se encontro chrome.exe en ${chromeDir}`);
  }

  console.log(`  Instalado en: ${chromeDir}\n`);
  return { type: 'chromium', executablePath: exePath, name: 'Chromium descargado' };
}

export async function ensureBrowser(): Promise<BrowserInfo> {
  process.stdout.write('Buscando navegadores instalados...');
  const available: BrowserInfo[] = [
    ...findAllSystemChromium(),
    ...findAllSystemFirefox(),
  ];
  const pwCache = findPlaywrightCache();
  if (pwCache) available.push(pwCache);

  const forceType = process.env.SCRAPPER_BROWSER as BrowserInfo['type'] | undefined;
  const pool = forceType ? available.filter(b => b.type === forceType) : available;
  const picked = pool.find(b => b.channel === 'chrome')
    ?? pool.find(b => b.channel === 'msedge')
    ?? pool[0];
  if (picked) {
    console.log(` ${picked.name}`);
    return picked;
  }

  console.log(' ninguno encontrado.');

  const browsersDir = getBrowsersDir();
  if (!existsSync(browsersDir)) mkdirSync(browsersDir, { recursive: true });

  return await downloadChromium(browsersDir);
}
