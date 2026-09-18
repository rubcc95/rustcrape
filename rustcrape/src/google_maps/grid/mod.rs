mod spain_border;
 
pub use spain_border::SPAIN;

use crate::verboser::Verboser;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Border {
    vertices: &'static [(f64, f64); 2978],
    polygons: &'static [usize; 24],
}

const BOUNDS_SOUTH: f64 = 27.0;
const BOUNDS_NORTH: f64 = 44.2;
const BOUNDS_WEST: f64 = -18.5;
const BOUNDS_EAST: f64 = 5.0;

impl Border {
    fn fmt6(v: f64) -> f64 {
        (v * 1_000_000.0).round() / 1_000_000.0
    }

    pub fn generate_grid(
        &self,
        zoom: u32,
        verboser: &dyn Verboser,
    ) -> Result<Vec<(f64, f64)>, String> {
        let cell_size = 360.0 / (1u64 << zoom) as f64;
        let half_size = cell_size / 2.0;

        let lat_count = ((BOUNDS_NORTH - BOUNDS_SOUTH - half_size) / cell_size) as usize + 1;
        let lng_count = ((BOUNDS_EAST - BOUNDS_WEST - half_size) / cell_size) as usize + 1;
        let total_cells = lat_count * lng_count;

        verboser.debug(&format!(
            "Grid: zoom={zoom} cell_size={cell_size}, lat_count={lat_count}, \
             lng_count={lng_count}, total_cells={total_cells}"
        ));

        verboser.seeding_tasks(0, 0, total_cells);

        let mut centers = Vec::new();

        let mut lat = BOUNDS_SOUTH + half_size;
        let mut total = 0;

        while lat <= BOUNDS_NORTH {
            let mut lng = BOUNDS_WEST + half_size;
            while lng <= BOUNDS_EAST {
                if self.intersects(lat, lng, half_size) {
                    centers.push((Self::fmt6(lat), Self::fmt6(lng)));
                }
                lng += cell_size;
            }
            total += lng_count;
            verboser.seeding_tasks(total, centers.len(), total_cells);
            lat += cell_size;
        }

        centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        centers.dedup();

        verboser.debug(&format!(
            "Grid: generated {} valid cell centers after dedup",
            centers.len()
        ));

        Ok(centers)
    }

    fn intersects(&self, lat: f64, lng: f64, half_size: f64) -> bool {
        let checks = [
            (lat, lng),
            (lat - half_size, lng - half_size),
            (lat - half_size, lng + half_size),
            (lat + half_size, lng - half_size),
            (lat + half_size, lng + half_size),
        ];
        for (plat, plng) in checks {
            if plat < BOUNDS_SOUTH
                || plat > BOUNDS_NORTH
                || plng < BOUNDS_WEST
                || plng > BOUNDS_EAST
            {
                continue;
            }
            if self.contains(plat, plng) {
                return true;
            }
        }
        false
    }

    fn contains(&self, lat: f64, lng: f64) -> bool {
        for item in self.polygons.windows(2) {
            let [start, end] = item.try_into().unwrap();
            let polygon = &self.vertices[start..end];
            if Self::point_in_polygon(lng, lat, polygon) {
                return true;
            }
        }
        false
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verboser::NoVerboser;

    #[test]
    fn test_generate_grid_zoom_12() {
        let result = SPAIN.generate_grid(12, &NoVerboser::default());
        assert!(result.is_ok());
        let points = result.unwrap();
        assert!(!points.is_empty(), "should generate at least one point");
        assert!(
            points.len() > 100,
            "expected more than 100 points for zoom=12, got {}",
            points.len()
        );
        for (lat, lng) in &points {
            assert!(*lat >= 27.0 && *lat <= 44.2, "lat {} out of bounds", lat);
            assert!(*lng >= -18.5 && *lng <= 5.0, "lng {} out of bounds", lng);
        }
        eprintln!("zoom=12: {} points generated", points.len());
    }
}
