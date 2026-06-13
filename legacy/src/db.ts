import { createConnection, type Connection, type RowDataPacket, type ResultSetHeader } from 'mysql2/promise';
import type { DbConfig } from './types.js';
import { generateGrid } from './generator.js';

const NUMERIC_TYPES = new Set([
  'decimal', 'double', 'float', 'int', 'bigint', 'smallint',
  'tinyint', 'mediumint', 'dec', 'fixed', 'numeric', 'real',
]);

export class SafeConnection {
  private conn: Connection;
  private config: DbConfig;

  constructor(conn: Connection, config: DbConfig) {
    this.conn = conn;
    this.config = config;
  }

  async execute<T extends RowDataPacket[] | ResultSetHeader = RowDataPacket[]>(
    sql: string,
    params?: any[]
  ): Promise<[T, any]> {
    for (let attempt = 0; ; attempt++) {
      try {
        return await this.conn.execute<T>(sql, params);
      } catch (err) {
        if (attempt >= 2) throw err;
        console.error('Error BD, reconectando...', (err as Error).message);
        await this.reconnect();
      }
    }
  }

  async query<T extends RowDataPacket[] | ResultSetHeader = RowDataPacket[]>(
    sql: string,
    params?: any[]
  ): Promise<[T, any]> {
    for (let attempt = 0; ; attempt++) {
      try {
        return await this.conn.query<T>(sql, params);
      } catch (err) {
        if (attempt >= 2) throw err;
        console.error('Error BD, reconectando...', (err as Error).message);
        await this.reconnect();
      }
    }
  }

  async end(): Promise<void> {
    await this.conn.end();
  }

  private async reconnect(): Promise<void> {
    for (let attempt = 0; ; attempt++) {
      try {
        await this.conn.end().catch(() => {});
        this.conn = await createConnection(this.config);
        return;
      } catch (err) {
        if (attempt >= 2) throw err;
        console.error(`Reconexion fallida (intento ${attempt + 1}/3):`, (err as Error).message);
        await new Promise(r => setTimeout(r, 2000));
      }
    }
  }
}

export async function connect(config: DbConfig): Promise<SafeConnection> {
  const conn = await createConnection(config);
  return new SafeConnection(conn, config);
}

interface ColumnInfo {
  COLUMN_NAME: string;
  DATA_TYPE: string;
  COLUMN_KEY: string;
}

async function getColumns(
  conn: SafeConnection,
  dbName: string,
  table: string,
): Promise<ColumnInfo[]> {
  const [rows] = await conn.execute<RowDataPacket[]>(
    `SELECT COLUMN_NAME, DATA_TYPE, COLUMN_KEY
     FROM INFORMATION_SCHEMA.COLUMNS
     WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
     ORDER BY ORDINAL_POSITION`,
    [dbName, table],
  );
  return rows as ColumnInfo[];
}

function validateCuadrantes(cols: ColumnInfo[]): void {
  const names = new Set(cols.map((c) => c.COLUMN_NAME));
  if (!names.has('lat') || !names.has('lng')) {
    throw new Error('Tabla cuadrantes: faltan columnas lat o lng');
  }
  for (const col of cols) {
    if (
      (col.COLUMN_NAME === 'lat' || col.COLUMN_NAME === 'lng') &&
      !NUMERIC_TYPES.has(col.DATA_TYPE)
    ) {
      throw new Error(
        `Tabla cuadrantes: columna '${col.COLUMN_NAME}' debe ser numerica ` +
          `(tipo actual: ${col.DATA_TYPE})`,
      );
    }
  }
}

const NEW_CUADRANTES_COLUMNS = [
  { name: 'in_progress', def: 'TINYINT(1) NOT NULL DEFAULT 0', type: 'tinyint' },
  { name: 'resultados_encontrados', def: 'INT DEFAULT NULL', type: 'int' },
  { name: 'duplicados_encontrados', def: 'INT DEFAULT NULL', type: 'int' },
  { name: 'started_at', def: 'TIMESTAMP NULL DEFAULT NULL', type: 'timestamp' },
];

async function ensureCuadrantesColumns(conn: SafeConnection, dbName: string): Promise<void> {
  const cols = await getColumns(conn, dbName, 'cuadrantes');
  const existing = new Set(cols.map((c) => c.COLUMN_NAME));

  for (const col of NEW_CUADRANTES_COLUMNS) {
    if (!existing.has(col.name)) {
      await conn.execute(`ALTER TABLE cuadrantes ADD COLUMN ${col.name} ${col.def}`);
    }
  }
}

function validateResults(cols: ColumnInfo[]): void {
  const required = new Set(['nombre', 'email', 'web', 'tfno', 'maps']);
  const names = new Set(cols.map((c) => c.COLUMN_NAME));
  const missing = [...required].filter((n) => !names.has(n));
  if (missing.length > 0) {
    throw new Error(`Tabla results: faltan columnas: ${missing.join(', ')}`);
  }

  const nombreCol = cols.find((c) => c.COLUMN_NAME === 'nombre');
  if (nombreCol && nombreCol.COLUMN_KEY !== 'UNI' && nombreCol.COLUMN_KEY !== 'PRI') {
    throw new Error('Tabla results: columna nombre debe tener UNIQUE KEY');
  }
}

export async function ensureTables(conn: SafeConnection, config: DbConfig, zoom: number): Promise<void> {
  const [rows] = await conn.execute<RowDataPacket[]>(
    `SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES
     WHERE TABLE_SCHEMA = ? AND TABLE_NAME IN ('cuadrantes', 'results')`,
    [config.database],
  );
  const existing = new Set(
    (rows as RowDataPacket[]).map((r) => r.TABLE_NAME as string),
  );

  const cuadrantesOk = existing.has('cuadrantes');
  const resultsOk = existing.has('results');

  if (cuadrantesOk && resultsOk) {
    const cuadrantesCols = await getColumns(conn, config.database, 'cuadrantes');
    const resultsCols = await getColumns(conn, config.database, 'results');
    validateCuadrantes(cuadrantesCols);
    validateResults(resultsCols);
    await ensureCuadrantesColumns(conn, config.database);
    console.log('Esquemas de base de datos validos.');
    return;
  }

  if (cuadrantesOk || resultsOk) {
    const existente = cuadrantesOk ? 'cuadrantes' : 'results';
    const faltante = cuadrantesOk ? 'results' : 'cuadrantes';
    throw new Error(
      `La tabla '${existente}' existe pero '${faltante}' no. Deben existir ambas o ninguna.`,
    );
  }

  console.log('Creando tablas...');
  await conn.execute(`
    CREATE TABLE IF NOT EXISTS cuadrantes (
      lat DECIMAL(10,6) NOT NULL,
      lng DECIMAL(10,6) NOT NULL,
      in_progress TINYINT(1) NOT NULL DEFAULT 0,
      resultados_encontrados INT DEFAULT NULL,
      duplicados_encontrados INT DEFAULT NULL,
      started_at TIMESTAMP NULL DEFAULT NULL,
      PRIMARY KEY (lat, lng)
    )
  `);
  await conn.execute(`
    CREATE TABLE IF NOT EXISTS results (
      id INT AUTO_INCREMENT PRIMARY KEY,
      nombre VARCHAR(255) NOT NULL,
      email VARCHAR(255) DEFAULT '',
      web VARCHAR(255) DEFAULT '',
      tfno VARCHAR(50) DEFAULT '',
      maps TEXT DEFAULT '',
      creado TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      UNIQUE KEY uq_datos (nombre, email, web, tfno)
    )
  `);
  console.log('Tablas creadas.');

  await generateGrid(zoom, conn);
}
