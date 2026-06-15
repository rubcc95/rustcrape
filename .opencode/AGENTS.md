# Development notes

## NO compilar Rust
No ejecutar `cargo check`, `cargo build`, `cargo test` ni ninguna verificación de compilación. Solo buildear frontend con `npm run build` en `rustcrape-gui/`.

## Project layout
- `rustcrape/` — Rust library (scraper engine, types, VPN)
- `rustcrape-gui/` — Tauri app (Vite + TypeScript frontend, Rust backend)
  - `src/main.ts` — Frontend logic
  - `index.html` — HTML template
  - `src/styles.css` — Styles
  - `src-tauri/src/` — Rust commands (commands.rs), persistence (persistence.rs), verboser (verboser.rs)

## Frontend build
```
cd rustcrape-gui && npm run build
```
Corre `tsc && vite build`, output en `rustcrape-gui/dist/`.

## VPN
- `rustcrape/src/vpn.rs` — rotación de NordVPN
- `nordvpn_path` apunta directamente al ejecutable
- GUI usa `pick_executable` (comando Tauri): diálogo nativo, valida ejecutable (permiso bit en Unix, extensión en Windows), emite evento `executable-picked`
- Frontend escucha `executable-picked` y actualiza el campo
