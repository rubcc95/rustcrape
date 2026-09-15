pub mod browser;
pub mod db;
pub mod empresite;
pub mod engine;
pub mod google_maps;
pub mod scraper;
pub mod storage;
pub mod types;
pub mod verboser;
pub mod vpn;

pub mod prelude {
    pub use crate::engine::run_dispatch;
    pub use crate::scraper::Scraper;
    pub use crate::storage::Persistence;
    pub use crate::types::*;
}

mod utils {

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

    /// Espera hasta que `f` devuelva `Some`, sondeando cada `interval` hasta `timeout`.
    pub async fn wait_until<F, Fut, T>(mut f: F, interval: Duration, timeout: Duration) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<Option<T>>>,
    {
        let start = std::time::Instant::now();
        loop {
            if let Some(value) = f().await? {
                return Ok(value);
            }
            if start.elapsed() >= timeout {
                return Err(anyhow::anyhow!("wait_until agotó el tiempo"));
            }
            tokio::time::sleep(interval).await;
        }
    }
}
