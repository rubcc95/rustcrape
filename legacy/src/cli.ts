import { Command } from 'commander';
import { readFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import type { RowDataPacket, ResultSetHeader } from 'mysql2/promise';
import { connect, ensureTables, type SafeConnection } from './db.js';
import { buscar } from './scrapper.js';
import { nordvpnDisponible, rotarVpn } from './vpn.js';

let currentQuadrant: { lat: number; lng: number } | null = null;
let shuttingDown = false;

const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const program = new Command();

interface VpnConfig {
  path: string;
  cadaIteraciones: number;
}

interface ScrapingContext {
  conn: SafeConnection;
  maxIterations: number;
  searchQuery: string;
  zoom: number;
  threshold: number;
  headless: boolean;
  delayMin?: number;
  delayMax?: number;
  rateLimit?: number;
  vpn?: VpnConfig;
}

program
  .name('scrapper')
  .description('Scrapper de Google Maps')
  .option('-u, --user <user>', 'Usuario MySQL')
  .option('--password <password>', 'Password MySQL')
  .option('-d, --database <db>', 'Base de datos')
  .option('-H, --host <host>', 'Host MySQL (default: localhost)')
  .option('-P, --port <port>', 'Puerto MySQL')
  .option('-s, --search <query>', 'Termino de busqueda (ej: tintorerias, restaurantes)')
  .option('-i, --iterations <n>', 'Maximo de iteraciones')
  .option('-z, --zoom <n>', 'Zoom para generar cuadricula')
  .option('-t, --threshold <n>', 'Threshold de parada para busqueda')
  .option('--headless', 'Modo headless (sin ventana grafica)')
  .option('--delayMin <ms>', 'Espera minima entre clicks en ms (default: 0)')
  .option('--delayMax <ms>', 'Espera maxima entre clicks en ms (default: 1400)')
  .option('--rateLimit <n>', 'Maximo de cuadrantes por hora (default: sin limite)')
  .option('--nordvpnPath <path>', 'Ruta al directorio de NordVPN (default: C:\\Program Files\\NordVPN)')
  .option('--rotarVpnCadaIteraciones <n>', 'Rotar VPN cada N iteraciones (requiere nordvpnPath)')
  .option('-c, --config <path>', 'Ruta a archivo de configuracion JSON')
  .action(async (opts) => {
    const explicitArgs = process.argv.slice(2).length > 0;
    const hasConfigArg = opts.config !== undefined;

    let configValues: Record<string, string> = {};

    if (hasConfigArg) {
      configValues = loadConfigFile(opts.config!);
    } else if (!explicitArgs) {
      configValues = loadConfigFile();
    }

    const { config: _config, ...cliValues } = opts as typeof opts & { config?: string };
    const cliDefined = Object.fromEntries(
      Object.entries(cliValues).filter(([_, v]) => v !== undefined)
    );
    const merged = { ...configValues, ...cliDefined } as Record<string, string>;

    const user = merged.user;
    const password = merged.password;
    const database = merged.database;
    const search = merged.search;
    const host = merged.host || 'localhost';
    const port = parseInt(merged.port || '3306', 10);
    const iterations = parseInt(merged.iterations || '0', 10);
    const zoom = parseInt(merged.zoom || '12', 10);
    const threshold = parseInt(merged.threshold || '3', 10);
    const headless = String(merged.headless) === 'true';
    const delayMin = merged.delayMin !== undefined ? parseInt(merged.delayMin, 10) : undefined;
    const delayMax = merged.delayMax !== undefined ? parseInt(merged.delayMax, 10) : undefined;
    const rateLimit = merged.rateLimit !== undefined ? parseInt(merged.rateLimit, 10) : undefined;
    const nordvpnPath = merged.nordvpnPath || 'C:\\Program Files\\NordVPN';
    const rotarVpnCadaIteraciones = merged.rotarVpnCadaIteraciones !== undefined
      ? parseInt(merged.rotarVpnCadaIteraciones, 10)
      : undefined;
    const vpnActivo = rotarVpnCadaIteraciones !== undefined && nordvpnDisponible(nordvpnPath);
    if (rotarVpnCadaIteraciones !== undefined && !vpnActivo) {
      console.warn(`Aviso: nordvpn.exe no encontrado en "${nordvpnPath}", rotacion VPN desactivada`);
    }

    if (!user || !password || !database || !search) {
      console.error('Error: --user, --password, --database y --search son requeridos');
      process.exit(1);
    }

    const dbConfig = { host, port, user, password, database };

    const conn = await connect(dbConfig);
    try {
      await ensureTables(conn, dbConfig, zoom);
      const ctx: ScrapingContext = {
        conn,
        maxIterations: iterations,
        searchQuery: search,
        zoom,
        threshold,
        headless,
        delayMin,
        delayMax,
        rateLimit,
        vpn: vpnActivo ? { path: nordvpnPath, cadaIteraciones: rotarVpnCadaIteraciones! } : undefined,
      };
      await runScrapingLoop(ctx);
    } finally {
      await conn.end();
    }
  });

function loadConfigFile(configPath?: string): Record<string, string> {
  const path = configPath || 'config.json';
  const resolvedPath = resolve(path);
  if (!existsSync(resolvedPath)) {
    if (configPath) {
      console.error(`Error: archivo de configuracion "${configPath}" no encontrado`);
      process.exit(1);
    }
    return {};
  }
  try {
    const raw = readFileSync(resolvedPath, 'utf-8');
    return JSON.parse(raw);
  } catch {
    console.error(`Error: archivo de configuracion "${resolvedPath}" no es JSON valido`);
    process.exit(1);
  }
}

export function run(): void {
  program.parse(process.argv);
}

async function runScrapingLoop(ctx: ScrapingContext): Promise<void> {
  const timestamps: number[] = [];
  let maxIterations = ctx.maxIterations;
  if (maxIterations === 0) maxIterations = Infinity;

  for (let i = 0; i < maxIterations; i++) {
    if (shuttingDown) break;

    if (ctx.vpn && i > 0 && i % ctx.vpn.cadaIteraciones === 0) {
      await rotarVpn(ctx.vpn.path, (msg) => process.stdout.write(`\r${msg}                    \n`));
    }

    if (ctx.rateLimit) {
      const now = Date.now();
      const windowStart = now - 3600000;
      while (timestamps.length > 0 && timestamps[0]! < windowStart) timestamps.shift();
      if (timestamps.length >= ctx.rateLimit) {
        const waitMs = timestamps[0]! + 3600000 - now;
        console.log(`\nRate limit alcanzado (${ctx.rateLimit}/h). Esperando ${Math.ceil(waitMs / 60000)} min...`);
        await new Promise(r => setTimeout(r, waitMs));
      }
      timestamps.push(Date.now());
    }

    let success = false;

    while (!success) {
      try {
        if (!(await iteration(ctx.conn, ctx.searchQuery, i + 1, maxIterations, ctx.zoom, ctx.threshold, ctx.headless, ctx.delayMin, ctx.delayMax))) {
          return;
        }
        success = true;
      } catch (err) {
        console.error('Error', err instanceof Error ? err.message : String(err));
        await new Promise(r => setTimeout(r, 2000));
      }
    }
  }
  console.log(shuttingDown ? '\nProceso interrumpido.' : '\nProceso completado.');
}

interface IterationStats {
  encontrados: number;
  insertados: number;
  duplicados: number;
  sinDatos: number;
}

async function iteration(conn: SafeConnection, searchQuery: string, current: number, total: number, zoom: number, threshold: number, headless: boolean, delayMin?: number, delayMax?: number): Promise<boolean> {
  const [rows] = await conn.execute<RowDataPacket[]>(
    'SELECT lat, lng FROM cuadrantes WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND resultados_encontrados IS NULL LIMIT 1',
  );
  const row = rows[0];
  if (!row) {
    console.log('\nNo hay cuadrantes pendientes en la tabla.');
    return false;
  }

  const lat = row.lat as number;
  const lng = row.lng as number;

  const [claimResult] = await conn.execute<ResultSetHeader>(
    'UPDATE cuadrantes SET in_progress = 1, started_at = NOW() WHERE lat = ? AND lng = ? AND (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR)',
    [lat, lng],
  );
  if (claimResult.affectedRows === 0) return true;

  currentQuadrant = { lat, lng };

  let statusMsg = 'Iniciando...';
  let spinnerTimer: ReturnType<typeof setInterval> | null = null;
  let frameIdx = 0;

  function startSpinner() {
    spinnerTimer = setInterval(() => {
      const frame = SPINNER_FRAMES[frameIdx % SPINNER_FRAMES.length];
      frameIdx++;
      process.stdout.write(`\r${frame} ${current}/${total} (${lat}, ${lng}) | ${statusMsg}`);
    }, 80);
  }

  function stopSpinner() {
    if (spinnerTimer) {
      clearInterval(spinnerTimer);
      spinnerTimer = null;
    }
  }

  function onScrapperStatus(msg: string) {
    statusMsg = msg;
  }

  try {
    startSpinner();

    const raw = await buscar(
      { lat, lng, zoom, stopThreshold: threshold, searchQuery, headless, delayMin, delayMax },
      onScrapperStatus,
    );

    process.stdout.write(`\r\u2713 ${current}/${total} (${lat}, ${lng}) — ${raw.length} resultados\n`);

    const conDatos = raw.filter((r) => r.email || r.web || r.tfno);
    const encontrados = conDatos.length;
    const sinDatos = raw.length - encontrados;

    let insertados = 0;
    let duplicados = 0;

    if (encontrados > 0) {
      const [infoResult] = await conn.query<ResultSetHeader>(
        'INSERT IGNORE INTO results (nombre, email, web, tfno, maps) VALUES ?',
        [conDatos.map((r) => [r.nombre, r.email ?? '', r.web ?? '', r.tfno ?? '', r.maps_url])],
      );
      insertados = infoResult.affectedRows;
      duplicados = encontrados - insertados;
    }

    await conn.execute<ResultSetHeader>(
      `UPDATE cuadrantes SET in_progress = 0, resultados_encontrados = ?, duplicados_encontrados = ?
       WHERE lat = ? AND lng = ?`,
      [encontrados, duplicados, lat, lng],
    );

    const stats: IterationStats = { encontrados, insertados, duplicados, sinDatos };
    printIterationStats(stats);
  } catch {
    await conn.execute('UPDATE cuadrantes SET in_progress = 0 WHERE lat = ? AND lng = ?', [lat, lng]);
  } finally {
    stopSpinner();
    if (currentQuadrant) {
      await conn.execute('UPDATE cuadrantes SET in_progress = 0 WHERE lat = ? AND lng = ?', [lat, lng]).catch(() => {});
    }
    currentQuadrant = null;
  }

  return true;
}

function printIterationStats(stats: IterationStats): void {
  console.log(`  Insertados: ${stats.insertados}  |  Duplicados: ${stats.duplicados}  |  Omitidos: ${stats.sinDatos}`);
  console.log('');
}
