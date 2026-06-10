mod spain_border;

struct Border {
    vertices: &'static [(f64, f64); 2978],
    polygons: &'static [usize; 24],
}

const SPAIN: Border = Border {
    vertices: spain_border::SPAIN_VERTEX,
    polygons: spain_border::SPAIN_POLYGONS,
};

const BOUNDS_SOUTH: f64 = 27.0;
const BOUNDS_NORTH: f64 = 44.2;
const BOUNDS_WEST: f64 = -18.5;
const BOUNDS_EAST: f64 = 5.0;

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

fn cell_intersects_spain(lat: f64, lng: f64, half_size: f64) -> bool {
    let checks = [
        (lat, lng),
        (lat - half_size, lng - half_size),
        (lat - half_size, lng + half_size),
        (lat + half_size, lng - half_size),
        (lat + half_size, lng + half_size),
    ];
    for (plat, plng) in checks {
        if plat < BOUNDS_SOUTH || plat > BOUNDS_NORTH || plng < BOUNDS_WEST || plng > BOUNDS_EAST {
            continue;
        }
        if SPAIN.contains(plat, plng) {
            return true;
        }
    }
    false
}

fn fmt6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

pub fn generate_grid(zoom: u32) -> Result<Vec<(f64, f64)>, String> {
    let cell_size = 360.0 / (1u64 << zoom) as f64;
    let half_size = cell_size / 2.0;

    let mut centers = Vec::new();

    let mut lat = BOUNDS_SOUTH + half_size;
    while lat <= BOUNDS_NORTH {
        let mut lng = BOUNDS_WEST + half_size;
        while lng <= BOUNDS_EAST {
            if cell_intersects_spain(lat, lng, half_size) {
                centers.push((fmt6(lat), fmt6(lng)));
            }
            lng += cell_size;
        }
        lat += cell_size;
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
        // all points should be within Spain's bounding box
        for (lat, lng) in &points {
            assert!(*lat >= 27.0 && *lat <= 44.2, "lat {} out of bounds", lat);
            assert!(*lng >= -18.5 && *lng <= 5.0, "lng {} out of bounds", lng);
        }
        eprintln!("zoom=12: {} points generated", points.len());
    }
}
