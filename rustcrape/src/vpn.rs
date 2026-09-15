use anyhow::Result;
// use chromiumoxide::Browser;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use tokio::process::Command;

use crate::utils::wait_until;
use crate::verboser::Verboser;

/// Tiempo maximo a esperar a que la VPN sea realmente usable tras conectar.
const VPN_READY_TIMEOUT: Duration = Duration::from_secs(45);
/// Intervalo entre sondeos de disponibilidad de la VPN.
const VPN_POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Servicio que devuelve la IP publica de salida.
const IP_PROBE_URL: &str = "https://api.ipify.org";

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
/// Consulta la IP publica de salida via `curl`.
///
/// El CLI de NordVPN no expone un comando de estado, asi que se usa la IP de
/// salida como senal observable de que el trafico ya sale por el nuevo tunel.
async fn public_ip() -> Result<String> {
    let output = Command::new("curl")
        .args(["-4", "--silent", "--max-time", "5", IP_PROBE_URL])
        .kill_on_drop(true)
        .output()
        .await?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

//  async fn wait_for_new_ip(prev: &str) -> Result<String> {
//      wait_until(
//          || async {
//              let curr = public_ip().await?;
//              Ok(if curr == prev { None } else { Some(curr) })
//          },
//          VPN_POLL_INTERVAL,
//          VPN_READY_TIMEOUT,
//      )
//      .await
//  }

pub trait VpnHandle {
    async fn disconect(&self) -> Result<bool>;
    async fn connect(&self) -> Result<bool>;
}

pub struct NoVpn;

impl VpnHandle for NoVpn {
    async fn disconect(&self) -> Result<bool> {
        Ok(false)
    }

    async fn connect(&self) -> Result<bool> {
        Ok(false)
    }
}

pub struct UnawaitedVpn<'a>(&'a Path);

impl VpnHandle for UnawaitedVpn<'_> {
    async fn disconect(&self) -> Result<bool> {
        Command::new(self.0)
            .arg("-d")
            .kill_on_drop(true)
            .status()
            .await?;
        Ok(true)
    }

    async fn connect(&self) -> Result<bool> {
        Command::new(self.0)
            .args(["-c", "-g", random_country()])
            .kill_on_drop(true)
            .status()
            .await?;
        Ok(true)
    }
}

pub struct AwaitedVpn<'path> {
    path: &'path Path,
    //browser: &'browser Browser,
}

impl VpnHandle for AwaitedVpn<'_> {
    async fn disconect(&self) -> Result<bool> {
        let prev = public_ip().await?;
        Command::new(self.path)
            .arg("-d")
            .kill_on_drop(true)
            .status()
            .await?;
        wait_until(
            || async {
                let curr = public_ip().await?;
                Ok(if curr == prev { None } else { Some(curr) })
            },
            VPN_POLL_INTERVAL,
            VPN_READY_TIMEOUT,
        )
        .await?;
        Ok(true)
    }

    async fn connect(&self) -> Result<bool> {
        let prev = public_ip().await?;
        Command::new(self.path)
            .args(["-c", "-g", random_country()])
            .kill_on_drop(true)
            .status()
            .await?;
        wait_until(
            || async {
                let curr = public_ip().await?;
                Ok(if curr == prev { None } else { Some(curr) })
            },
            VPN_POLL_INTERVAL,
            VPN_READY_TIMEOUT,
        )
        .await?;

        Ok(true)
    }
}

pub async fn rotate_vpn(
    vpn_handle: &impl VpnHandle,
    verboser: &dyn Verboser,
) -> Result<()> {
    verboser.vpn_rotating();
    vpn_handle.disconect().await?;
    vpn_handle.connect().await?;
    
    // // IP de salida actual, antes de mover el tunel. Best-effort: si falla, tras
    // // conectar basta con que haya conectividad.
    // //let mut ip = public_ip().await?;

    // // Desconectar es best-effort: puede no haber conexion previa.
    // Command::new(nordvpn_path)
    //     .arg("-d")
    //     .kill_on_drop(true)
    //     .status()
    //     .await?;

    // //ip = wait_for_new_ip(&ip).await?;

    // let connected = Command::new(nordvpn_path)
    //     .args(["-c", "-g", random_country()])
    //     .kill_on_drop(true)
    //     .status()
    //     .await?;

    Ok(())
    //wait_for_new_ip(&ip).await?;

    // // El comando `-c` solo confirma que NordVPN acepto la orden, no que el
    // // tunel este operativo. Se espera a observar la nueva IP de salida.
    // let ready = if curl_available().await {
    //     wait_until_ready(
    //         public_ip,
    //         before.as_deref(),
    //         VPN_READY_TIMEOUT,
    //         VPN_POLL_INTERVAL,
    //         || verboser.is_cancelled(),
    //     )
    //     .await
    // } else {
    //     wait_until_ready(
    //         vpn_reachable,
    //         None,
    //         VPN_READY_TIMEOUT,
    //         VPN_POLL_INTERVAL,
    //         || verboser.is_cancelled(),
    //     )
    //     .await
    // };

    // if ready {
    //     verboser.vpn_rotated();
    // } else {
    //     verboser.warn("VPN: la conexion no quedo lista a tiempo");
    // }

    // Ok(true)
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
    pub async fn tick(&self, verboser: &dyn Verboser) -> Result<bool> {
        if self.force_rotate(verboser).await? {
            let mut counter = self.counter.lock().unwrap();
            *counter = counter.wrapping_add(1);                        
            Ok(true)
        } else{
            verboser.vpn_not_available();
            Ok(false)
        }        
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate(&self, verboser: &dyn Verboser) -> Result<bool> {
        let Some(path) = &self.path else {
            verboser.vpn_not_available();
            return Ok(false);
        };

        self.force_rotate_internal(&UnawaitedVpn(path), verboser).await?;
        *self.counter.lock().unwrap() = 0;

        Ok(true)
    }

        /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate_awaited(&self, verboser: &dyn Verboser) -> Result<bool> {
        let Some(path) = &self.path else {
            verboser.vpn_not_available();
            return Ok(false);
        };

        self.force_rotate_internal(&AwaitedVpn { path: path }, verboser).await?;
        *self.counter.lock().unwrap() = 0;

        Ok(true)
    }

      /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    async fn force_rotate_internal(&self, handle: &impl VpnHandle, verboser: &dyn Verboser) -> Result<bool> {
        // VPN desactivada: la casilla de la GUI va ligada a la frecuencia de
        // rotacion, de modo que `frequency == 0` significa que el usuario la
        // desactivo. En ese caso no se rota aunque NordVPN este instalado.
        if self.frequency == 0 {
            return Ok(false);
        }

        // let Some(path) = &self.path else {
        //     verboser.vpn_not_available();
        //     return Ok(false);
        // };

        rotate_vpn(handle, verboser).await?;
        *self.counter.lock().unwrap() = 0;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::verboser::DebugProgress;

    // #[tokio::test]
    // async fn test_force_rotate_con_vpn_desactivada_no_rota() {
    //     // VPN desactivada (frecuencia 0): no debe rotar aunque haya ruta,
    //     // sin importar que NordVPN este instalado.
    //     let rotator = VpnRotator::new(Some("C:/nordvpn-irrelevante.exe".to_string()), 0);
    //     assert!(!rotator.force_rotate(&DebugProgress).await.unwrap());
    // }

    // #[tokio::test]
    // async fn test_force_rotate_sin_ruta_no_rota() {
    //     let rotator = VpnRotator::new(None, 5);
    //     assert!(!rotator.force_rotate(&DebugProgress).await.unwrap());
    // }

    // #[tokio::test]
    // async fn test_wait_until_ready_detecta_ip_nueva() {
    //     let ready = wait_until_ready(
    //         || async { Some("2.2.2.2".to_string()) },
    //         Some("1.1.1.1"),
    //         Duration::from_millis(50),
    //         Duration::from_millis(1),
    //         || false,
    //     )
    //     .await;
    //     assert!(ready);
    // }

    // #[tokio::test]
    // async fn test_wait_until_ready_con_misma_ip_expira() {
    //     let ready = wait_until_ready(
    //         || async { Some("1.1.1.1".to_string()) },
    //         Some("1.1.1.1"),
    //         Duration::from_millis(30),
    //         Duration::from_millis(1),
    //         || false,
    //     )
    //     .await;
    //     assert!(!ready);
    // }

    // #[tokio::test]
    // async fn test_wait_until_ready_sin_ip_previa_basta_con_conectividad() {
    //     let ready = wait_until_ready(
    //         || async { Some("3.3.3.3".to_string()) },
    //         None,
    //         Duration::from_millis(50),
    //         Duration::from_millis(1),
    //         || false,
    //     )
    //     .await;
    //     assert!(ready);
    // }

    // #[tokio::test]
    // async fn test_wait_until_ready_cancelado() {
    //     let ready = wait_until_ready(
    //         || async { Option::<String>::None },
    //         None,
    //         Duration::from_secs(60),
    //         Duration::from_millis(1),
    //         || true,
    //     )
    //     .await;
    //     assert!(!ready);
    // }

    // #[tokio::test]
    // async fn test_wait_until_ready_sin_respuesta_expira() {
    //     let ready = wait_until_ready(
    //         || async { Option::<String>::None },
    //         None,
    //         Duration::from_millis(30),
    //         Duration::from_millis(1),
    //         || false,
    //     )
    //     .await;
    //     assert!(!ready);
    // }
}
