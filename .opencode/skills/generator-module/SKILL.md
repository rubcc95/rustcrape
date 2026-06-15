---
name: generator-module
description: >-
  Use when working on the grid generator at biz-scraping/src/generator/.
  Covers the Border struct, generate_grid algorithm, Spain border GeoJSON
  constants, point-in-polygon testing, and bound grid logic.
  NOT for the scraper or shared types.
---

# Generator Module

Files:
- `biz-scraping/src/generator/mod.rs` — `Border` struct + grid generation
- `biz-scraping/src/generator/spain_border.rs` — Spain border vertices + polygon indices

## Border struct

```rust
pub struct Border {
    vertices: &'static [(f64, f64); 2978],   // 2978 (lng, lat) points
    polygons: &'static [usize; 24],           // 24 indices defining polygon ranges
}
```

`polygons` is used with `.windows(2)` to create `[start, end)` ranges into
`vertices`. Each range is one polygon. 24 entries → 23 polygon ranges.

The singleton instance is exported as:
```rust
pub use spain_border::SPAIN;
```

## Grid generation

```rust
impl Border {
    pub fn generate_grid(&self, zoom: u32) -> Result<Vec<(f64, f64)>, String>
}
```

### Algorithm

1. **Cell size** = `360.0 / 2^zoom` degrees (e.g. zoom=12 → ~0.088°)
2. **Bounding box**: `lat [27.0, 44.2]`, `lng [-18.5, 5.0]` (covers Iberian peninsula)
3. **Iterate** over bounding box in `cell_size` steps
4. **Intersection test** (`Border::intersects`):
   - Check center + 4 corners of each cell
   - Each point is tested via `Border::contains` (ray-casting point-in-polygon)
   - If any point is inside Spain → cell is included
5. **Output**: sorted, deduped `Vec<(lat, lng)>` of cell centers rounded to 6 decimals (`fmt6`)

### Coordinate system

- Points in `vertices` are stored as `(lng, lat)` pairs (GeoJSON convention)
- Grid output uses `(lat, lng)` pairs (geographic convention)
- The `point_in_polygon(lng, lat, polygon)` method takes `(lng, lat)` coords

## Constants

```rust
const BOUNDS_SOUTH: f64 = 27.0;
const BOUNDS_NORTH: f64 = 44.2;
const BOUNDS_WEST: f64 = -18.5;
const BOUNDS_EAST: f64 = 5.0;
```

## spain_border.rs

The Spain border GeoJSON was converted to Rust constants:

```rust
pub const SPAIN: Border = Border {
    vertices: SPAIN_VERTEX,    // &[(f64, f64); 2978] — all border vertices
    polygons: SPAIN_POLYGONS,  // &[usize; 24] — polygon boundary indices
};
```

`SPAIN_VERTEX` and `SPAIN_POLYGONS` are private constants in the same file.
The file imports `use super::Border;` since `Border` is defined in the parent
module.

## Testing

```rust
#[test]
fn test_generate_grid_zoom_12() {
    let result = SPAIN.generate_grid(12);
    assert!(result.unwrap().len() > 100);
}
```

Run with: `cargo test -p biz-scraping -- generator`

The test currently takes ~20s for zoom=12 (realistic: ~4000+ cells).

## Usage notes

- `generate_grid()` is called by the DB layer (future `db.rs`) to populate the
  `bounds` table when it's first created
- Can also be called standalone to inspect which bound centers cover Spain
- The method takes `&self` (accesses border data) — always use `SPAIN.generate_grid(z)`
