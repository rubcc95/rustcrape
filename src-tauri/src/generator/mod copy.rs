use std::collections::HashSet;

mod spain_border;

struct Border {
    vertices: &'static [(f64, f64); 2978],
    polygons: &'static [usize; 24],
}

const SPAIN: Border = Border {
    vertices: spain_border::SPAIN_VERTEX,
    polygons: spain_border::SPAIN_POLYGONS,
};

impl Border {
    fn contains(&self, lat: f64, lng: f64) -> bool {
        for item in self.polygons.windows(2) {
            let [start, end] = item.try_into().unwrap();
            let polygon = &self.vertices[start..end];

            if point_in_polygon(lng, lat, polygon) {
                return true;
            }
        }
    
        false
    }
}

fn point_in_polygon(lng: f64, lat: f64, polygon: &[(f64, f64)]) -> bool {
    let mut inside = false;
    for edge in polygon.windows(2) {
        let [(xi, yi), (xj, yj)] = edge.try_into().unwrap();
        if ((yi > lat) != (yj > lat)) && (lng < (xj - xi) * (lat - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
    }
    inside
}

fn tile_to_bounds(tx: u64, ty: u64, zoom: u8) -> (f64, f64, f64, f64) {
    let n = (1u64 << zoom) as f64;
    let west = tx as f64 / n * 360.0 - 180.0;
    let east = (tx + 1) as f64 / n * 360.0 - 180.0;
    let lat_rad_north = (std::f64::consts::PI * (1.0 - 2.0 * ty as f64 / n))
        .sinh()
        .atan();
    let north = lat_rad_north.to_degrees();
    let lat_rad_south = (std::f64::consts::PI * (1.0 - 2.0 * (ty + 1) as f64 / n))
        .sinh()
        .atan();
    let south = lat_rad_south.to_degrees();
    (north, south, west, east)
}

fn tile_center(tx: u64, ty: u64, zoom: u8) -> (f64, f64) {
    let (north, south, west, east) = tile_to_bounds(tx, ty, zoom);
    let lat = (north + south) / 2.0;
    let lng = (west + east) / 2.0;
    (lat, lng)
}

pub fn generate_grid(zoom: u8) -> Result<Vec<(f64, f64)>, String> {
    let mut tiles = HashSet::new();
    let mut queue = vec![(1u64, 1u64)];

    let max_tile = (1u64 << zoom) - 1;

    while let Some((tx, ty)) = queue.pop() {
        if tx > max_tile || ty > max_tile {
            continue;
        }
        if !tiles.insert((tx, ty)) {
            continue;
        }

        let (north, south, west, east) = tile_to_bounds(tx, ty, zoom);

        let corners = [(north, west), (north, east), (south, east), (south, west)];

        let inside_count = corners
            .iter()
            .filter(|(lat, lng)| SPAIN.contains(*lat, *lng))
            .count();

        if inside_count > 0 {
            if inside_count < 4 && zoom > 10 {
                let child_tx = tx << 1;
                let child_ty = ty << 1;
                for cx in child_tx..=child_tx + 1 {
                    for cy in child_ty..=child_ty + 1 {
                        if cx <= max_tile * 2 && cy <= max_tile * 2 {
                            queue.push((cx, cy));
                        }
                    }
                }
            }
        }
    }

    let mut centers = Vec::new();
    for (tx, ty) in &tiles {
        let (lat, lng) = tile_center(*tx, *ty, zoom);
        if SPAIN.contains(lat, lng) {
            let lat_rounded = (lat * 1_000_000.0).round() / 1_000_000.0;
            let lng_rounded = (lng * 1_000_000.0).round() / 1_000_000.0;
            centers.push((lat_rounded, lng_rounded));
        }
    }

    centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    centers.dedup();

    Ok(centers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_grid_zoom_12() {
        let result = generate_grid(12);
        assert!(result.is_ok());
        let points = result.unwrap();
        assert!(!points.is_empty(), "should generate at least one point");
        assert!(
            points.len() > 100,
            "expected more than 100 points for zoom=12, got {}",
            points.len()
        );
        for (lat, lng) in &points {
            assert!(
                SPAIN.contains(*lat, *lng),
                "point ({}, {}) should be inside Spain",
                lat,
                lng
            );
            assert!(*lat >= 27.0 && *lat <= 44.0, "lat {} out of bounds", lat);
            assert!(*lng >= -19.0 && *lng <= 5.0, "lng {} out of bounds", lng);
        }
        eprintln!("zoom=12: {} points generated", points.len());
    }
}
