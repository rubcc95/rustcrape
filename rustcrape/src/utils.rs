use anyhow::Result;
use std::time::Duration;

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

/// Espera hasta que `f` devuelva `Some`, sondeando cada `interval` hasta `timeout`.
pub async fn wait_until<F, Fut, T>(
    mut f: F,
    interval: impl WaitUntilDuration,
    timeout: impl WaitUntilDuration,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<Option<T>>>,
{
    let start = std::time::Instant::now();

    loop {
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
