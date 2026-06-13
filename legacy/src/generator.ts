import type { SafeConnection } from './db.js';
import cliProgress from 'cli-progress';
import spainBorderData from '../data/spain-border.json' with { type: 'json' };

const BOUNDS = {
  south: 27.0,
  north: 44.2,
  west: -18.5,
  east: 5.0,
};

function ensureSpainBorder(): any {
  return spainBorderData;
}

function pointInRing(lng: number, lat: number, ring: number[][]): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const a = ring[i];
    const b = ring[j];
    if (!a || !b) continue;
    const [xi, yi] = a;
    const [xj, yj] = b;
    if (yi === undefined || yj === undefined || xi === undefined || xj === undefined) continue;

    if ((yi > lat) !== (yj > lat) && lng < ((xj - xi) * (lat - yi)) / (yj - yi) + xi) {
      inside = !inside;
    }
  }
  return inside;
}

function pointInPolygon(lat: number, lng: number, rings: number[][][]): boolean {
  const outerRing = rings[0];
  if (!outerRing) return false;
  if (!pointInRing(lng, lat, outerRing)) return false;

  for (let r = 1; r < rings.length; r++) {
    const hole = rings[r];
    if (!hole) continue;
    if (pointInRing(lng, lat, hole)) return false;
  }
  return true;
}

function isInsideSpain(lat: number, lng: number, spain: any): boolean {
  const g = spain.geometry;
  if (g.type === 'Polygon') {
    return pointInPolygon(lat, lng, g.coordinates);
  }
  if (g.type === 'MultiPolygon') {
    for (const polygon of g.coordinates) {
      if (pointInPolygon(lat, lng, polygon)) return true;
    }
    return false;
  }
  return false;
}

function cellIntersectsSpain(lat: number, lng: number, halfSize: number, spain: any): boolean {
  const checks: [number, number][] = [
    [lat, lng],
    [lat - halfSize, lng - halfSize],
    [lat - halfSize, lng + halfSize],
    [lat + halfSize, lng - halfSize],
    [lat + halfSize, lng + halfSize],
  ];
  for (const [plat, plng] of checks) {
    if (plat < BOUNDS.south || plat > BOUNDS.north || plng < BOUNDS.west || plng > BOUNDS.east) {
      continue;
    }
    if (isInsideSpain(plat, plng, spain)) return true;
  }
  return false;
}

function fmt6(v: number): number {
  return Math.round(v * 1_000_000) / 1_000_000;
}

export async function generateGrid(zoom: number, conn: SafeConnection): Promise<void> {
  const spain = ensureSpainBorder();

  const cellSize = 360 / Math.pow(2, zoom);
  const halfSize = cellSize / 2;

  const latSteps = Math.ceil((BOUNDS.north - BOUNDS.south) / cellSize);
  const lngSteps = Math.ceil((BOUNDS.east - BOUNDS.west) / cellSize);
  const totalCells = latSteps * lngSteps;

  console.log(`Generando cuadricula (zoom ${zoom}): ${latSteps}x${lngSteps} = ${totalCells} celdas a evaluar`);

  const bar = new cliProgress.SingleBar({
    format: '  [{bar}] {percentage}% | {value}/{total} celdas | {label}',
    barCompleteChar: '\u2588',
    barIncompleteChar: '\u2591',
    hideCursor: true,
  });
  bar.start(totalCells, 0, { label: 'Evaluando...' });

  await conn.execute('TRUNCATE TABLE cuadrantes');

  let batch: [number, number][] = [];
  let total = 0;
  let cellsChecked = 0;

  let lat = BOUNDS.south + halfSize;
  while (lat <= BOUNDS.north) {
    let lng = BOUNDS.west + halfSize;
    while (lng <= BOUNDS.east) {
      if (cellIntersectsSpain(lat, lng, halfSize, spain)) {
        batch.push([fmt6(lat), fmt6(lng)]);
        if (batch.length >= 500) {
          await conn.query('INSERT INTO cuadrantes (lat, lng, in_progress, resultados_encontrados, duplicados_encontrados) VALUES ?', [batch.map(([l, n]) => [l, n, 0, null, null])]);
          total += batch.length;
          batch = [];
        }
      }
      cellsChecked++;
      if (cellsChecked % 50 === 0) {
        bar.update(cellsChecked, { label: `${total} insertadas` });
      }
      lng += cellSize;
    }
    lat += cellSize;
  }

  if (batch.length > 0) {
    await conn.query('INSERT INTO cuadrantes (lat, lng, in_progress, resultados_encontrados, duplicados_encontrados) VALUES ?', [batch.map(([l, n]) => [l, n, 0, null, null])]);
    total += batch.length;
  }

  bar.update(totalCells, { label: `${total} insertadas` });
  bar.stop();
  console.log(`Total celdas en tierra firme: ${total}`);
}
