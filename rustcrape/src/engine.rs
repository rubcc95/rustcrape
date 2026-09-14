use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::browser::Browser;
use crate::db::PersistenceKind;
use crate::empresite::EmpresiteScraper;
use crate::google_maps::GoogleMapsScraper;
use crate::scraper::Scraper;
use crate::storage::Persistence;
use crate::types::{Config, ExecutionMode};
use crate::verboser::Verboser;
use crate::vpn::VpnRotator;

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(3600);

pub async fn run_dispatch(mut config: Config, verboser: impl Verboser) {
    let persist = loop {
        match PersistenceKind::create(&config.db, &mut config.google_maps, &verboser).await {
            Ok(p) => break p,
            Err(err) => verboser.warn(&format!(
                "Failed to connect to database: {err}. Retrying..."
            )),
        }
    };

    let verboser: Arc<dyn Verboser> = Arc::new(verboser);
    let vpn = Arc::new(VpnRotator::new(
        config.nordvpn_path.clone(),
        config.ip_rotation_frequency,
    ));
    let browser_path = config.browser_path.clone();

    let gmaps = config
        .google_maps
        .enabled
        .then(|| GoogleMapsScraper::new(config.google_maps.clone()));
    let empresite = config
        .empresite
        .enabled
        .then(|| EmpresiteScraper::new(config.empresite.clone()));

    match config.execution_mode {
        ExecutionMode::Sequential => {
            if let Some(scraper) = gmaps {
                run_target(
                    scraper,
                    persist.clone(),
                    verboser.clone(),
                    vpn.clone(),
                    browser_path.clone(),
                )
                .await;
            }
            if let Some(scraper) = empresite {
                run_target(
                    scraper,
                    persist.clone(),
                    verboser.clone(),
                    vpn.clone(),
                    browser_path.clone(),
                )
                .await;
            }
        }
        ExecutionMode::Parallel => {
            let mut handles = Vec::new();
            if let Some(scraper) = gmaps {
                let persist = persist.clone();
                let verboser = verboser.clone();
                let vpn = vpn.clone();
                let browser_path = browser_path.clone();
                handles.push(tokio::spawn(async move {
                    run_target(scraper, persist, verboser, vpn, browser_path).await;
                }));
            }
            if let Some(scraper) = empresite {
                let persist = persist.clone();
                let verboser = verboser.clone();
                let vpn = vpn.clone();
                let browser_path = browser_path.clone();
                handles.push(tokio::spawn(async move {
                    run_target(scraper, persist, verboser, vpn, browser_path).await;
                }));
            }
            for handle in handles {
                let _ = handle.await;
            }
        }
    }
}

async fn run_target<S: Scraper>(
    scraper: S,
    persist: PersistenceKind,
    verboser: Arc<dyn Verboser>,
    vpn: Arc<VpnRotator>,
    browser_path: Option<String>,
) {
    if let Err(err) = scraper.seed(&persist, &*verboser).await {
        verboser.error(&format!("Error seeding tasks for {}: {err}", scraper.name()));
        return;
    }

    let mut iteration: u32 = 0;
    let mut timestamps: Vec<Instant> = Vec::new();

    loop {
        if verboser.is_cancelled() {
            verboser.warn("Operation canceled by the user");
            break;
        }

        if let Some(iterations) = scraper.iterations() {
            if iterations.get() <= iteration {
                verboser.finished();
                break;
            }
        }

        if let Some(rate_limit) = scraper.rate_limit() {
            let now = Instant::now();
            timestamps.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);
            if timestamps.len() >= rate_limit.get() as usize {
                let oldest = timestamps[0];
                let wait = RATE_LIMIT_WINDOW
                    .checked_sub(now.duration_since(oldest))
                    .unwrap_or_default();
                verboser.rate_limit_wait(wait);
                tokio::time::sleep(wait).await;
            }
            timestamps.push(Instant::now());
        }

        vpn.tick(&*verboser).await;

        verboser.obtaining_task();

        let claimed = match scraper.claim(&persist).await {
            Ok(claimed) => claimed,
            Err(err) => {
                verboser.error(&format!("Error claiming task: {err}"));
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let Some((task_id, params)) = claimed else {
            verboser.finished();
            break;
        };

        let label = scraper.describe(&params);
        verboser.claimed_task(&label);

        if verboser.is_cancelled() {
            verboser.warn("Cancelado antes de abrir el navegador");
            let _ = scraper.release(&persist, task_id, None).await;
            break;
        }

        verboser.opening_browser(&label);
        let browser =
            match Browser::launch(scraper.headless(), browser_path.as_deref().map(Path::new)).await
            {
                Ok(browser) => browser,
                Err(err) => {
                    verboser.error(&format!("Error launching browser: {err}"));
                    let _ = scraper.release(&persist, task_id, None).await;
                    iteration += 1;
                    continue;
                }
            };

        let result = scraper.scrape(&browser, &params, &*verboser).await;
        verboser.closing_browser();
        let _ = browser.close().await;

        match result {
            Ok(result) => {
                verboser.writing_coincidences(&result.coincidences);
                let items = result.coincidences.len() as i32;
                let written = match persist
                    .write_coincidences(scraper.name(), result.coincidences)
                    .await
                {
                    Ok(written) => written as i32,
                    Err(err) => {
                        verboser.error(&format!("Error writing coincidences: {err}"));
                        let _ = scraper.release(&persist, task_id, None).await;
                        iteration += 1;
                        continue;
                    }
                };
                verboser.written_coincidences(written);
                scraper
                    .release(&persist, task_id, Some((items, items - written)))
                    .await
                    .ok();
                verboser.released_task();

                if let Err(err) = scraper.advance(&persist, &params, result.has_more).await {
                    verboser.error(&format!("Error advancing queue: {err}"));
                }
            }
            Err(err) => {
                verboser.error(&format!("Error during scraping: {err}"));
                let _ = scraper.release(&persist, task_id, None).await;
                iteration += 1;
                continue;
            }
        }

        iteration += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DbConfig, EmpresiteConfig, GoogleMapsConfig};
    use crate::verboser::DebugVerboser;

    fn test_config() -> Config {
        Config {
            google_maps: GoogleMapsConfig {
                enabled: true,
                search_query: "Tintorería".to_string(),
                zoom: 12,
                stop_threshold: 3,
                delay_min: 500,
                delay_max: 2000,
                headless: false,
                rate_limit: None,
                iterations: Some(std::num::NonZeroU32::new(1).unwrap()),
            },
            empresite: EmpresiteConfig {
                enabled: false,
                search_query: "Tintorería".to_string(),
                delay_min: 500,
                delay_max: 2000,
                headless: false,
                rate_limit: None,
                iterations: Some(std::num::NonZeroU32::new(1).unwrap()),
            },
            execution_mode: ExecutionMode::Sequential,
            db: DbConfig::Sqlite { path: None },
            nordvpn_path: None,
            browser_path: None,
            ip_rotation_frequency: 0,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_run() {
        let config = test_config();
        run_dispatch(config, DebugVerboser).await;
    }
}
 