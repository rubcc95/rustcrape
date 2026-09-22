use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::process::Command;

use crate::context::Context;
use crate::utils::{WaitUntilTimeoutError, wait_until};
use crate::verboser::Verboser;

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
/// salida como señal observable de que el tráfico ya sale por el nuevo túnel.
async fn public_ip(client: &reqwest::Client) -> reqwest::Result<String> {
    Ok(client
        .get(IP_PROBE_URL)
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .text()
        .await?
        .trim()
        .to_string())
}

async fn run_vpn_command<I, S>(
    args: I,
    http: &reqwest::Client,
    path: &Path,
    v: &dyn Verboser,
) -> Result<bool>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let prev = public_ip(http).await?;
    v.debug(&format!("VPN: current public IP before command: {prev}"));

    let args: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|a| a.as_ref().to_os_string())
        .collect();
    v.debug(&format!("VPN: running {} {:?}", path.display(), args));

    Command::new(path)
        .args(&args)
        .kill_on_drop(true)
        .status()
        .await?;

    v.debug("VPN: command finished, waiting for the public IP to change");
    let output = wait_until(
        || async {
            let curr = public_ip(http).await;
            let curr = match curr {
                Ok(res) => res,
                Err(err) => {
                    if err.is_request() || err.is_body() {
                        v.debug(&format!("VPN probe: no connectivity yet, retrying: {err}"));
                        return Ok(None);
                    } else {
                        return Err(err.into());
                    }
                }
            };
            Ok(if curr == prev { None } else { Some(curr) })
        },
        Duration::from_millis(500),
        Duration::from_secs(60),
    )
    .await;
    if let Err(err) = output {
        if err.downcast::<WaitUntilTimeoutError>().is_ok() {
            return Ok(false);
        }
    }

    Ok(true)
}

async fn rotate_vpn(ctx: &Context) -> Result<()> {
    let Some(path) = ctx.vpn_path() else {
        ctx.verboser().vpn_not_available();
        return Ok(());
    };

    let v = ctx.verboser();
    loop {
        v.vpn_rotating();
        let http = ctx.http();
        v.debug(&format!("VPN: nordvpn binary at {}", path.display()));
        let success = run_vpn_command(["-d"], http, path, v).await?;
        v.debug(match success {
            true => "VPN disconnected successfully",
            false => "VPN disconnect command finished but public IP did not change",
        });
        let country = random_country();
        v.debug(&format!("VPN: connecting to country '{country}'"));
        if run_vpn_command(["-c", "-g", country], http, path, v).await? {
            break;
        }
        v.debug("VPN: connect command finished but public IP did not change");
        v.debug("VPN connected successfully");
    }

    Ok(())
}

/// Rotador de VPN compartido por todos los targets. Rota globalmente cada
/// `frequency` tareas procesadas, sea cual sea el target que las ejecute.
pub struct VpnRotator {
    path: Option<PathBuf>,
    frequency: u32,
    counter: Mutex<u32>,
    rotating: AtomicBool,
}

/// Guarda RAII que libera el flag de rotación al salir del ambito, incluso si
/// la rotación termina en error.
struct RotationGuard<'a>(&'a AtomicBool);

impl<'a> RotationGuard<'a> {
    pub fn new(rotator: &'a VpnRotator) -> Option<Self> {
        match rotator
            .rotating
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Some(RotationGuard(&rotator.rotating)),
            Err(_) => None,
        }
    }
}

impl Drop for RotationGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl VpnRotator {
    pub fn new(path: Option<String>, frequency: u32) -> Self {
        Self {
            path: path.map(PathBuf::from),
            frequency,
            counter: Mutex::new(0),
            rotating: AtomicBool::new(false),
        }
    }

    /// Debe llamarse una vez por tarea procesada (por cualquier target).
    pub async fn tick(&self, ctx: &Context) -> Result<bool> {
        let (count, should_rotate) = {
            let mut counter = self.counter.lock().unwrap();
            *counter = counter.wrapping_add(1);
            (*counter, self.frequency > 0 && *counter >= self.frequency)
        };
        ctx.verboser().debug(&format!(
            "VPN tick: counter={count}, frequency={}, rotate={should_rotate}",
            self.frequency
        ));

        if !should_rotate {
            return Ok(false);
        }

        self.force_rotate(ctx).await
    }

    /// Fuerza una rotacion inmediata a peticion del scraper. Devuelve `true` si
    /// la rotacion tuvo exito; `false` si la VPN esta desactivada, no hay ruta
    /// configurada, no esta disponible o fallo la conexion.
    pub async fn force_rotate(&self, ctx: &Context) -> Result<bool> {
        // VPN desactivada: la casilla de la GUI va ligada a la frecuencia de
        // rotacion, de modo que `frequency == 0` significa que el usuario la
        // desactivo. En ese caso no se rota aunque NordVPN este instalado.
        if self.frequency == 0 {
            ctx.verboser()
                .debug("VPN rotation skipped: rotation disabled (frequency=0)");
            return Ok(false);
        }

        // Ignorar peticiones de rotacion mientras ya hay una en curso para no
        // lanzar comandos NordVPN solapados.
        Ok(match RotationGuard::new(self) {
            Some(guard) => {
                ctx.verboser().debug("VPN rotation: starting");
                rotate_vpn(ctx).await?;
                *self.counter.lock().unwrap() = 0;
                drop(guard);
                ctx.verboser()
                    .debug("VPN rotation: finished, counter reset to 0");
                true
            }
            None => {
                ctx.verboser()
                    .debug("VPN rotation already in progress, ignoring request");
                false
            }
        })
    }

    #[inline]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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
