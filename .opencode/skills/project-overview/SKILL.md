---
name: project-overview
description: >-
  Use when you need a high-level understanding of the biz-scraping project,
  its workspace layout, architectural decisions, or file structure.
  Use when you need to know how modules are organized, what dependencies are
  used, or what conventions to follow. Do NOT use for detailed implementation
  of a specific module — use scrapper-module, generator-module, or core-types instead.
---

# biz-scraping — Project Overview

## Workspace layout

```
biz-scraping/
├── Cargo.toml                          # Workspace root
├── biz-scraping/                       # Core library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # Module declarations
│       ├── types.rs                    # Shared types (Tintoreria, SearchConfig, etc.)
│       ├── progress.rs                 # ProgressReporter trait (agnostic)
│       ├── scrapper.rs                 # Google Maps scraper (chromiumoxide)
│       └── generator/                  # Grid generation module
│           ├── mod.rs                  # Border struct + generate_grid()
│           └── spain_border.rs         # Spain border GeoJSON as Rust const
├── biz-scraping-gui/                   # Tauri GUI (future crate)
│   └── src-tauri/
│       └── Cargo.toml
└── legacy/                             # Original TypeScript/Playwright codebase
```

## Key architectural decisions

| Decision | Choice |
|----------|--------|
| Runtime | `tokio` (async) |
| Browser automation | `chromiumoxide` (CDP, not WebDriver) |
| Error handling | `anyhow` |
| DB layer (future) | `sqlx` + MySQL |
| Progress reporting | Trait-based (`ProgressReporter`), usable by CLI and GUI |
| Browser lifecycle | Passed as argument — core lib does NOT launch browsers |

## Cargo.toml (biz-scraping)

```toml
[package]
name = "biz-scraping"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1"
chromiumoxide = "0.9"
futures = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
```

All versions are minimal-bound (e.g. `"0.9"` = `^0.9`). Cargo resolves the latest compatible.

## Module responsibilities

| Module | Responsibility |
|--------|---------------|
| `types.rs` | Data structures: `Tintoreria`, `SearchConfig`, `DbConfig`, `IterationStats` |
| `progress.rs` | `ProgressReporter` trait + `NoopProgress` impl |
| `scrapper.rs` | `buscar()` — navigate GMaps, accept cookies, scrape feed or single result |
| `generator/` | `Border::generate_grid()` — generate bound grid from Spain border GeoJSON |

## Convention notes

- Private helper functions are `snake_case` without `pub`, prefixed at module level
- `anyhow::Result<T>` is the return type for fallible operations
- JS evaluation (`page.evaluate()`) is preferred over Element API for DOM interaction (avoids stale element references after scrolling)
- `chromiumoxide::Page::evaluate()` returns `EvaluationResult`; use `.into_value::<T>()?` to extract typed values
- All async functions are `pub async fn` or `async fn` at module level
- Strings use `String` (owned) in public types, `&str` in function parameters

## Related skills

- `scrapper-module` — Details on `buscar()`, feed scraping, JS selectors, chromiumoxide patterns
- `generator-module` — Details on Border, generate_grid, point-in-polygon algorithm
- `core-types` — Details on Tintoreria, SearchConfig, DbConfig, IterationStats, ProgressReporter

## Legacy reference

The original TypeScript codebase is at `legacy/` and uses Playwright + MySQL. The Rust port follows the same logic but with chromiumoxide instead of Playwright and a trait-based progress system instead of callbacks.
