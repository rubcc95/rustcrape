import { build } from 'esbuild';
import { mkdirSync, existsSync, copyFileSync, cpSync, rmSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DIST = join(__dirname, 'dist');
const OUT_EXE = join(DIST, 'scrapper.exe');
const SEA_BLOB = join(DIST, 'sea-prep.blob');
const SEA_CFG = join(__dirname, 'sea-config.json');

if (!existsSync(DIST)) mkdirSync(DIST);

// Plugin: reemplaza require('playwright') / require('playwright-core') por
// createRequire(process.execPath)('playwright') para que SEA resuelva
// desde node_modules junto al exe.
const playwrightExternalPlugin = {
  name: 'playwright-runtime-require',
  setup(b) {
    // Intercepta import de 'playwright' y 'playwright-core'
    b.onResolve({ filter: /^playwright(-core)?$/ }, args => ({
      path: args.path,
      namespace: 'playwright-runtime',
    }));
    b.onLoad({ filter: /.*/, namespace: 'playwright-runtime' }, args => ({
      contents: `
        const { createRequire } = require('module');
        const _req = createRequire(process.execPath);
        module.exports = _req(${JSON.stringify(args.path)});
      `,
      loader: 'js',
    }));
  },
};

// 1. Compilar bundle CJS
console.log('[1/4] Compilando bundle JS...');
await build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  platform: 'node',
  format: 'cjs',
  outfile: join(DIST, 'bundle.cjs'),
  target: 'node24',
  plugins: [playwrightExternalPlugin],
  sourcemap: false,
  minify: false,
  define: {
    'process.env.NODE_ENV': '"production"',
  },
  loader: { '.json': 'json' },
});
console.log('    bundle.cjs generado');

// 2. Generar blob SEA
console.log('[2/4] Generando blob SEA...');
execSync(`node --experimental-sea-config ${SEA_CFG}`, { stdio: 'inherit' });

// 3. Copiar node.exe como base
console.log('[3/4] Preparando ejecutable...');
copyFileSync(process.execPath, OUT_EXE);

// 4. Inyectar blob
console.log('[4/4] Inyectando blob en el ejecutable...');
execSync(
  `npx postject "${OUT_EXE}" NODE_SEA_BLOB "${SEA_BLOB}" --sentinel-fuse NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2`,
  { stdio: 'inherit', cwd: __dirname }
);

// Copiar node_modules de playwright al dist
const nmSrc = join(__dirname, 'node_modules');
const nmDist = join(DIST, 'node_modules');
if (!existsSync(nmDist)) mkdirSync(nmDist);

console.log('\nCopiando dependencias de playwright...');
for (const pkg of ['playwright', 'playwright-core', 'chromium-bidi', 'mitt', 'urlpattern-polyfill', 'devtools-protocol']) {
  const src = join(nmSrc, pkg);
  const dst = join(nmDist, pkg);
  if (existsSync(src)) {
    if (existsSync(dst)) rmSync(dst, { recursive: true, force: true });
    cpSync(src, dst, { recursive: true });
    console.log(`    copiado: ${pkg}`);
  }
}

// Limpiar artefactos intermedios
for (const tmp of [join(DIST, 'bundle.cjs'), SEA_BLOB]) {
  if (existsSync(tmp)) rmSync(tmp);
}

console.log(`
=== BUILD COMPLETADO ===
Distribuir la carpeta dist/ completa:
  dist/
    scrapper.exe         <- ejecutable principal (no requiere Node instalado)
    node_modules/        <- playwright (necesario junto al exe)
`);
