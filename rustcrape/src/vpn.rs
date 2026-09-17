use anyhow::Result;
// use chromiumoxide::Browser;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use tokio::process::Command;

use crate::context::Context;
use crate::utils::wait_until;

/// Tiempo maximo a esperar a que la VPN sea realmente usable tras conectar.
const VPN_READY_TIMEOUT: Duration = Duration::from_secs(45);
/// Intervalo entre sondeos de disponibilidad de la VPN.
const VPN_POLL_INTERVAL: Duration = Duration::from_millis(500);
/// Servicio que devuelve la IP publica de salida.
const IP_PROBE_URL: &str = "https://api.ipify.org";

/// Pais al que se conecta la VPN en cada rotacion.
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
/// Consulta la IP publica de salida mediante `reqwest`.
///
/// El CLI de NordVPN no expone un comando de estado, asi que se usa la IP de
/// salida como senal observable de que el trafico ya sale por el nuevo tunel.
async fn public_ip(client: &reqwest::Client) -> Result<String> {
    let body = client
        .get(IP_PROBE_URL)
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(body.trim().to_string())
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

#[allow(async_fn_in_trait)]
trait VpnHandle {
    async fn disconect(&self, http: &reqwest::Client) -> Result<bool>;
    async fn connect(&self, http: &reqwest::Client) -> Result<bool>;
}

pub struct NoVpn;

impl VpnHandle for NoVpn {
    async fn disconect(&self, _: &reqwest::Client) -> Result<bool> {
        Ok(false)
    }

    async fn connect(&self, _: &reqwest::Client) -> Result<bool> {
        Ok(false)
    }
}

pub struct UnawaitedVpn<'a>(&'a Path);

impl VpnHandle for UnawaitedVpn<'_> {
    async fn disconect(&self, _: &reqwest::Client) -> Result<bool> {
        Command::new(self.0)
            .arg("-d")
            .kill_on_drop(true)
            .status()
            .await?;
        Ok(true)
    }

    async fn connect(&self, _: &reqwest::Client) -> Result<bool> {
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
    async fn disconect(&self, http: &reqwest::Client) -> Result<bool> {
        let prev = public_ip(http).await?;
        Command::new(self.path)
            .arg("-d")
            .kill_on_drop(true)
            .status()
            .await?;
        wait_until(
            || async {
                let curr = public_ip(http).await?;
                Ok(if curr == prev { None } else { Some(curr) })
            },
            VPN_POLL_INTERVAL,
            VPN_READY_TIMEOUT,
        )
        .await?;
        Ok(true)
    }

    async fn connect(&self, http: &reqwest::Client) -> Result<bool> {
        let prev = public_ip(http).await?;
        Command::new(self.path)
            .args(["-c", "-g", random_country()])
            .kill_on_drop(true)
            .status()
            .await?;
        wait_until(
            || async {
                let curr = public_ip(http).await?;
                Ok(if curr == prev { None } else { Some(curr) })
            },
            VPN_POLL_INTERVAL,
            VPN_READY_TIMEOUT,
        )
        .await?;

        Ok(true)
    }
}

async fn rotate_vpn(vpn_handle: &impl VpnHandle, ctx: &Context) -> Result<()> {
    ctx.verboser().vpn_rotating();
    let http = ctx.http();
    vpn_handle.disconect(http).await?;
    ctx.verboser().debug("VPN disconnected");
    vpn_handle.connect(http).await?;    
    ctx.verboser().debug("VPN connected");

    Ok(())
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
    pub async fn tick(&self, ctx: &Context) -> Result<bool> {
        let should_rotate = {
            let mut counter = self.counter.lock().unwrap();
            *counter = counter.wrapping_add(1);
            self.frequency > 0 && *counter >= self.frequency
        };

        if !should_rotate {
            return Ok(false);
        }

        self.force_rotate(ctx).await
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate(&self, ctx: &Context) -> Result<bool> {
        let Some(path) = &self.path else {
            ctx.verboser().vpn_not_available();
            return Ok(false);
        };

        self.force_rotate_internal(&UnawaitedVpn(path), ctx).await
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate_awaited(&self, ctx: &Context) -> Result<bool> {
        let Some(path) = &self.path else {
            ctx.verboser().vpn_not_available();
            return Ok(false);
        };

        self.force_rotate_internal(&AwaitedVpn { path: path }, ctx)
            .await
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    async fn force_rotate_internal(&self, handle: &impl VpnHandle, ctx: &Context) -> Result<bool> {
        // VPN desactivada: la casilla de la GUI va ligada a la frecuencia de
        // rotacion, de modo que `frequency == 0` significa que el usuario la
        // desactivo. En ese caso no se rota aunque NordVPN este instalado.
        if self.frequency == 0 {
            return Ok(false);
        }

        rotate_vpn(handle, ctx).await?;
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
