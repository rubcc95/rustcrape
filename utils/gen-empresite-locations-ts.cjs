#!/usr/bin/env node
"use strict";

// Genera el modulo TS del frontend a partir del JSON canonico de localidades:
//   utils/empresite_locations.json -> rustcrape-gui/src/lib/data/empresiteLocations.ts
// Re-ejecutar tras cada rastreo (utils/empresite-locations.cjs).

const fs = require("fs");
const path = require("path");

const SRC = path.join(__dirname, "empresite_locations.json");
const OUT = path.join(
  __dirname,
  "..",
  "rustcrape-gui",
  "src",
  "lib",
  "data",
  "empresiteLocations.ts"
);

function main() {
  if (!fs.existsSync(SRC)) {
    throw new Error(`No existe el JSON de origen: ${SRC}`);
  }
  const raw = JSON.parse(fs.readFileSync(SRC, "utf8"));
  if (!Array.isArray(raw)) {
    throw new Error("El JSON de origen no es un array");
  }

  const byProvince = {};
  for (const province of raw) {
    byProvince[province.id] = (province.towns || []).map((town) => ({
      id: town.id,
      name: town.name,
    }));
  }

  const content =
    "// Generado por utils/gen-empresite-locations-ts.cjs a partir de\n" +
    "// utils/empresite_locations.json. No editar a mano: re-ejecuta el generador.\n\n" +
    "export interface Town {\n" +
    "  id: string;\n" +
    "  name: string;\n" +
    "}\n\n" +
    "export const TOWNS_BY_PROVINCE: Record<string, Town[]> = " +
    JSON.stringify(byProvince, null, 2) +
    ";\n";

  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, content, "utf8");

  const provinces = Object.keys(byProvince).length;
  const towns = Object.values(byProvince).reduce((sum, list) => sum + list.length, 0);
  console.log(`Generado ${OUT} (${provinces} provincias, ${towns} localidades)`);
}

main();
