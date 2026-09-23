use anyhow::Result;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

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
