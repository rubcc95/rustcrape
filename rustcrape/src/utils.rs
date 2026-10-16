use anyhow::Result;
use chrono::NaiveDate;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Convierte una fecha de Empresite a `NaiveDate`. Empresite la devuelve en
/// formato `dia-mes-ano` sin ceros a la izquierda (`23-6-2005`); se aceptan
/// tambien `/` como separador y ceros a la izquierda. Devuelve `None` si el
/// texto esta vacio, no tiene el formato esperado o la fecha no existe.
pub fn parse_empresite_date(raw: &str) -> Option<NaiveDate> {
    let mut parts = raw.trim().split(['-', '/']);
    let day: u32 = parts.next()?.trim().parse().ok()?;
    let month: u32 = parts.next()?.trim().parse().ok()?;
    let year: i32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Separa la direccion de Empresite en `(direccion_completa, provincia)`. La
/// direccion tiene el formato `calle, numero, municipio, CP, provincia`, por lo
/// que la provincia es el ultimo segmento tras la ultima coma. Devuelve `None`
/// en cada campo que falte o este vacio.
pub fn split_address_province(raw: &str) -> (Option<String>, Option<String>) {
    let address = raw.trim();
    if address.is_empty() {
        return (None, None);
    }
    let province = address
        .rsplit_once(',')
        .map(|(_, tail)| tail.trim())
        .filter(|tail| !tail.is_empty())
        .map(str::to_string);
    (Some(address.to_string()), province)
}

/// Separa la actividad CNAE de Empresite (`codigo - descripcion`, p. ej.
/// `2829 - Fabricacion de otra maquinaria`) en su codigo numerico y su
/// descripcion textual. La descripcion conserva los guiones que pueda contener.
/// Devuelve `(None, None)` si el texto esta vacio.
pub fn split_cnae(raw: &str) -> (Option<i32>, Option<String>) {
    let text = raw.trim();
    if text.is_empty() {
        return (None, None);
    }
    match text.split_once('-') {
        Some((code, desc)) => {
            let desc = desc.trim();
            match code.trim().parse::<i32>() {
                Ok(code) => (
                    Some(code),
                    if desc.is_empty() {
                        None
                    } else {
                        Some(desc.to_string())
                    },
                ),
                Err(_) => (None, Some(text.to_string())),
            }
        }
        // Sin separador: si es puramente numerico lo tratamos como codigo,
        // en caso contrario como descripcion.
        None => match text.parse::<i32>() {
            Ok(code) => (Some(code), None),
            Err(_) => (None, Some(text.to_string())),
        },
    }
}

/// Retardo pseudo-aleatorio entre peticiones, dentro del rango configurado.
pub fn random_delay(min_ms: u64, max_ms: u64) -> Duration {
    if max_ms == 0 {
        return Duration::ZERO;
    }
    let range = max_ms.saturating_sub(min_ms);
    if range == 0 {
        return Duration::from_millis(min_ms);
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let extra = nanos % range;
    Duration::from_millis(min_ms + extra)
}

#[derive(Debug, thiserror::Error)]
#[error("wait_until reached timeout")]
pub struct WaitUntilTimeoutError;

/// Señal de cancelación cooperativa: distingue el aborto por el usuario de un
/// timeout normal, para que los llamadores no lo confundan con "no hubo suerte
/// pero seguimos".
#[derive(Debug, thiserror::Error)]
#[error("operación cancelada por el usuario")]
pub struct Cancelled;

/// Duerme `duration` salvo que el token se cancele antes, en cuyo caso aborta
/// de inmediato devolviendo `Cancelled`.
pub async fn sleep_cancellable(duration: Duration, cancel: &CancellationToken) -> Result<()> {
    tokio::select! {
        _ = cancel.cancelled() => Err(Cancelled.into()),
        _ = tokio::time::sleep(duration) => Ok(()),
    }
}

pub trait WaitUntilDuration {
    fn duration(&self) -> Option<Duration>;
}

impl WaitUntilDuration for Duration {
    #[inline]
    fn duration(&self) -> Option<Duration> {
        Some(*self)
    }
}

impl WaitUntilDuration for Option<Duration> {
    fn duration(&self) -> Option<Duration> {
        *self
    }
}

impl WaitUntilDuration for () {
    fn duration(&self) -> Option<Duration> {
        None
    }
}

/// Espera hasta que `f` devuelva `Some`, sondeando cada `interval` hasta
/// `timeout`. Aborta de inmediato con `Cancelled` si el token se cancela.
pub async fn wait_until<F, Fut, T>(
    mut f: F,
    interval: impl WaitUntilDuration,
    timeout: impl WaitUntilDuration,
    cancel: &CancellationToken,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Option<T>>>,
{
    let start = std::time::Instant::now();

    loop {
        if cancel.is_cancelled() {
            return Err(Cancelled.into());
        }
        if let Some(value) = f().await? {
            return Ok(value);
        }
        if let Some(timeout) = timeout.duration() {
            if start.elapsed() >= timeout {
                return Err(WaitUntilTimeoutError.into());
            }
        }
        if let Some(interval) = interval.duration() {
            tokio::time::sleep(interval).await;
        }
    }
}
