---
name: core-types
description: >-
  Use when you need to understand or modify the shared data types and progress
  reporting infrastructure: Tintoreria, SearchConfig, DbConfig, IterationStats,
  and the ProgressReporter trait. These are defined in types.rs and progress.rs
  and are consumed by scrapper.rs, future db.rs, and the future CLI/GUI crates.
---

# Core Types & Progress Reporting

## types.rs — `biz-scraping/src/types.rs`

### Tintoreria (scraped business data)

```rust
pub struct Tintoreria {
    pub nombre: String,           // Business name
    pub email: Option<String>,     // Email address (None if not found)
    pub web: Option<String>,       // Website URL (None if not found)
    pub tfno: Option<String>,      // Phone number (None if not found)
    pub maps_url: String,          // Clean Google Maps URL (query params stripped)
}
```

- Implements `Serialize`, `Deserialize` (serde) — usable with serde_json, Tauri, etc.
- `Option<String>` rather than empty string for missing data

### SearchConfig (scraper parameters)

```rust
pub struct SearchConfig {
    pub lat: f64,               // Quadrant center latitude
    pub lng: f64,               // Quadrant center longitude
    pub zoom: u32,              // Map zoom level (affects initial view)
    pub stop_threshold: u32,    // Consecutive out-of-range results before stopping
    pub search_query: String,    // e.g. "tintorerías" or "dry cleaners"
    pub delay_min: u64,         // Min delay between scrolls (ms)
    pub delay_max: u64,         // Max delay between scrolls (ms)
}
```

Implements `Default`:
- `lat: 40.4168`, `lng: -3.7038`, `zoom: 12`, `stop_threshold: 3`
- `delay_min: 500`, `delay_max: 2000`

### DbConfig (future DB connection)

```rust
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}
```

### IterationStats (per-quadrant scraping stats)

```rust
pub struct IterationStats {
    pub encontrados: u64,   // Total results with some data found
    pub insertados: u64,    // Successfully inserted (non-duplicate)
    pub duplicados: u64,    // Skipped (UNIQUE constraint)
    pub sin_datos: u64,    // Results with no email, web, or phone
}
```

Implements `Default` (all zeros).

## progress.rs — `biz-scraping/src/progress.rs`

### ProgressReporter trait

```rust
pub trait ProgressReporter: Send + Sync {
    fn on_status(&self, status: &str) {}
    fn on_error(&self, error: &str) {}
    fn on_quadrant_progress(&self, current: usize, total: usize, lat: f64, lng: f64, msg: &str) {}
    fn on_quadrant_complete(&self, current: usize, total: usize, lat: f64, lng: f64, count: usize) {}
}
```

All methods have default no-op implementations. Implementors override only what
they need. `Send + Sync` enables shared access across threads (GUI-friendly).

Typical status strings from `scrapper.rs`:
- "Navegando a Google Maps..."
- "Aceptando cookies..."
- "Buscando resultados..."
- "Cargando mas resultados..."
- "Procesando {nombre}..."
- "Error extrayendo resultado: {error}"

### NoopProgress (no-op implementation)

```rust
#[derive(Clone, Default)]
pub struct NoopProgress;
impl ProgressReporter for NoopProgress {}
```

Used when progress reporting is not needed (tests, headless batch processing).

## Usage patterns

### As parameter (dyn dispatch)
```rust
pub async fn buscar(
    browser: &Browser,
    config: &SearchConfig,
    progress: &dyn ProgressReporter,
) -> Result<Vec<Tintoreria>>
```

### As trait bound (monomorphized, for future use)
```rust
pub async fn some_fn(progress: impl ProgressReporter) { ... }
```

## Adding new types

- Always derive `Debug, Clone` (and optionally `Serialize, Deserialize`)
- Use `Option<T>` for nullable fields, not sentinel values
- New types that cross crate boundaries should implement `Serialize + Deserialize`
