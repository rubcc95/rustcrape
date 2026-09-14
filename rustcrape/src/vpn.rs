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

pub async fn rotate_vpn(nordvpn_path: &Path, verboser: &dyn Verboser) -> std::io::Result<bool> {
    verboser.vpn_rotating();

    let status = Command::new(nordvpn_path)
        .arg("-d")
        .kill_on_drop(true)
        .status()
        .await?;

    if !status.success() {
        return Ok(false);
    }

    let connected = Command::new(nordvpn_path)
        .args(["-c", "-g", random_country()])
        .kill_on_drop(true)
        .status()
        .await
        .map(|status| status.success())?;

    if connected {
        verboser.vpn_rotated();
    }

    Ok(connected)
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
        if let Some(path) = &self.path {
            let _ = rotate_vpn(path, verboser).await;
        } else {
            verboser.vpn_not_available();
            return;
        }
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate(&self, verboser: &dyn Verboser) -> std::io::Result<bool> {
        // VPN desactivada: la casilla de la GUI va ligada a la frecuencia de
        // rotacion, de modo que `frequency == 0` significa que el usuario la
        // desactivo. En ese caso no se rota aunque NordVPN este instalado.
        if self.frequency == 0 {
            return Ok(false);
        }

        let Some(path) = &self.path else {
            verboser.vpn_not_available();
            return Ok(false);
        };

        let rotated = rotate_vpn(path, verboser).await?;
        if rotated {
            let mut counter = self.counter.lock().unwrap();
            *counter = 0;
        }
        Ok(rotated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verboser::DebugProgress;

    #[tokio::test]
    async fn test_force_rotate_con_vpn_desactivada_no_rota() {
        // VPN desactivada (frecuencia 0): no debe rotar aunque haya ruta,
        // sin importar que NordVPN este instalado.
        let rotator = VpnRotator::new(Some("C:/nordvpn-irrelevante.exe".to_string()), 0);
        assert!(!rotator.force_rotate(&DebugProgress).await.unwrap());
    }

    #[tokio::test]
    async fn test_force_rotate_sin_ruta_no_rota() {
        let rotator = VpnRotator::new(None, 5);
        assert!(!rotator.force_rotate(&DebugProgress).await.unwrap());
    }
}
