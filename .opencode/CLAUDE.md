# Instrucciones del proyecto biz-scraping

## NO compilar Rust
El toolchain Rust de este entorno NO permite compilar (toolchain GNU de Windows no es compatible con sqlx y otras dependencias). NO intentar `cargo check`, `cargo build`, `cargo test` ni ninguna verificacion de compilacion. Confiar en el criterio del desarrollador.

## Base de datos
- La BD runtime usa MySQL/MariaDB.
- El crate `rustcraper` usa `sqlx` para queries.
- Todas las operaciones SQL estan en `rustcraper/src/db.rs`.
- El error 1064 con `push_values` vacio ya esta solucionado (ver fix en `write_coincidences`).

## Skills disponibles en .opencode/skills/
- core-types: tipos compartidos (Tintoreria, SearchConfig, etc.)
- scrapper-module: logica del scraper de Google Maps
- generator-module: generacion de grid
- project-overview: vision general del proyecto

## Archivos temporales
Usar `C:\Users\Usuario\.claude\tmp\` para scripts/archivos temporales.
NO usar `z:\tmp\`.
