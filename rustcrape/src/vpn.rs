use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tokio::process::Command;

use crate::verboser::Verboser;

const VPN_COUNTRIES: &[&str] = &[
    "Spain",
    "France",
    "Germany",
    "Italy",
    "Portugal",
    "switzerland",
    "Belgium",
    "India",
    "Sweden",
    "Ireland",
    "Brazil",
    "Mexico",
    "Albania",
    "Mexico",
    "Poland",
    "United States",
    "South Korea",
    "Hong Kong",
    "Singapore",
    "Austria",
    "Norway",
];

fn random_country() -> &'static str {
    let idx = rand::random::<usize>() % VPN_COUNTRIES.len();
    VPN_COUNTRIES[idx]
}

pub fn nordvpn_available(nordvpn_path: &Path) -> bool {
    nordvpn_path.exists()
}

pub async fn rotate_vpn(nordvpn_path: &Path, verboser: &dyn Verboser) -> bool {
    verboser.vpn_rotating();

    let _ = Command::new(nordvpn_path).arg("-d").kill_on_drop(true).status().await;

    let connected = Command::new(nordvpn_path)
        .args(["-c", "-g", random_country()])
        .kill_on_drop(true)
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false);

    if connected {
        verboser.vpn_rotated();
    }

    connected
}

/// Rotador de VPN compartido por todos los targets. Rota globalmente cada
/// `frequency` tareas procesadas, sea cual sea el target que las ejecute.
pub struct VpnRotator {
    path: Option<PathBuf>,
    frequency: u32,
    counter: Mutex<u32>,
}

impl VpnRotator {
    pub fn new(path: Option<String>, frequency: u32) -> Self {
        Self {
            path: path.map(PathBuf::from),
            frequency,
            counter: Mutex::new(0),
        }
    }

    /// Debe llamarse una vez por tarea procesada (por cualquier target).
    pub async fn tick(&self, verboser: &dyn Verboser) {
        let should_rotate = {
            let mut counter = self.counter.lock().unwrap();
            let value = *counter;
            *counter = counter.wrapping_add(1);
            self.frequency > 0 && value % self.frequency == 0
        };

        if !should_rotate {
            return;
        }

        let Some(path) = &self.path else {
            return;
        };

        if nordvpn_available(path) {
            let _ = rotate_vpn(path, verboser).await;
        } else {
            verboser.vpn_not_available();
        }
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si no hay VPN configurada, no esta
    /// disponible o fallo la conexion.
    pub async fn force_rotate(&self, verboser: &dyn Verboser) -> bool {
        let Some(path) = &self.path else {
            return false;
        };

        if !nordvpn_available(path) {
            verboser.vpn_not_available();
            return false;
        }

        let rotated = rotate_vpn(path, verboser).await;
        if rotated {
            let mut counter = self.counter.lock().unwrap();
            *counter = 0;
        }
        rotated
    }
}
