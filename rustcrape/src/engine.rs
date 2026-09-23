use std::time::{Duration, Instant};

use crate::context::Context;
use crate::db::PersistenceKind;
use crate::empresite_http::EmpresiteHttpScraper;
use crate::google_maps::GoogleMapsScraper;
use crate::scraper::Scraper;
use crate::storage::Persistence;
use crate::types::{Config, ExecutionMode};
use crate::utils::sleep_cancellable;
use crate::verboser::Verboser;

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(3600);

pub async fn run_dispatch(mut config: Config, verboser: impl Verboser) {
    let db_kind = match &config.db {
        crate::types::DbConfig::Sqlite { path } => match path {
            Some(p) => format!("sqlite({p})"),
            None => "sqlite(default)".to_string(),
        },
        crate::types::DbConfig::Mysql { host, port, database, .. } => {
            format!("mysql({host}:{port}/{database})")
        }
    };
    verboser.debug(&format!(
        "Engine: starting dispatch (mode={:?}, gmaps={}, empresite={}, db={db_kind}, \
         ip_rotation_frequency={}, vpn_configured={})",
        config.execution_mode,
        config.gmaps.enabled,
        config.empresite.enabled,
        config.ip_rotation_frequency,
        config.nordvpn_path.is_some(),
    ));

    let persist = loop {
        if verboser.is_cancelled() {
            verboser.warn("Cancelado mientras se conectaba a la base de datos");
            return;
        }
        match PersistenceKind::create(&config.db, &mut config.gmaps, &verboser).await {
            Ok(p) => {
                verboser.debug("Engine: database ready");
                break p;
            }
            Err(err) => verboser.warn(&format!(
                "Failed to connect to database: {err}. Retrying..."
            )),
        }
    };

    let ctx = Context::new(
        verboser,
        config.nordvpn_path.clone(),
        config.ip_rotation_frequency,
    );

    let gmaps = config.gmaps.enabled.then(GoogleMapsScraper::new);
    let empresite = config
        .empresite
        .enabled
        .then(EmpresiteHttpScraper::new);

    match config.execution_mode {
        ExecutionMode::Sequential => {
            ctx.verboser().debug("Engine: sequential execution mode");
            if let Some(scraper) = gmaps {
                run_target(scraper, &config, &persist, &ctx).await;
            }
            if let Some(scraper) = empresite {
                run_target(scraper, &config, &persist, &ctx).await;
            }
        }
        ExecutionMode::Parallel => {
            todo!()
        }
    }
    ctx.verboser().debug("Engine: dispatch finished");
}

async fn run_target<S: Scraper>(
    scraper: S,
    config: &Config,
    persist: &impl Persistence,
    ctx: &Context,
) {
    let name = scraper.name();
    ctx.verboser().debug(&format!(
        "Engine[{name}]: seeding work queue (iterations={:?}, rate_limit={:?})",
        scraper.iterations(config).map(|n| n.get()),
        scraper.rate_limit(config).map(|n| n.get()),
    ));
    if let Err(err) = scraper.seed(config, persist, ctx.verboser()).await {
        ctx.verboser()
            .error(&format!("Error seeding tasks to scraper: {err}"));
        return;
    }
    ctx.verboser().debug(&format!("Engine[{name}]: queue seeded"));

    let mut iteration: u32 = 0;
    let mut timestamps: Vec<Instant> = Vec::new();

    loop {
        if ctx.verboser().is_cancelled() {
            ctx.verboser()
                .debug(&format!("Engine[{name}]: cancellation requested, stopping"));
            ctx.verboser().warn("Operation canceled by the user");
            break;
        }

        if let Some(iterations) = scraper.iterations(config) {
            if iterations.get() <= iteration {
                ctx.verboser().debug(&format!(
                    "Engine[{name}]: iteration limit reached ({}), stopping",
                    iterations.get()
                ));
                ctx.verboser().finished();
                break;
            }
        }

        ctx.verboser()
            .debug(&format!("Engine[{name}]: starting iteration {iteration}"));

        if let Some(rate_limit) = scraper.rate_limit(config) {
            let now = Instant::now();
            timestamps.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);
            if timestamps.len() >= rate_limit.get() as usize {
                let oldest = timestamps[0];
                let wait = RATE_LIMIT_WINDOW
                    .checked_sub(now.duration_since(oldest))
                    .unwrap_or_default();
                ctx.verboser().debug(&format!(
                    "Engine[{name}]: rate limit reached ({}/{} in window), waiting {wait:?}",
                    timestamps.len(),
                    rate_limit.get()
                ));
                ctx.verboser().rate_limit_wait(wait);
                if sleep_cancellable(wait, ctx.cancellation()).await.is_err() {
                    break;
                }
            }
            timestamps.push(Instant::now());
        }

        match ctx.vpn_tick().await {
            Ok(rotated) => ctx
                .verboser()
                .debug(&format!("Engine[{name}]: vpn tick rotated={rotated}")),
            Err(err) => {
                ctx.verboser().error(&format!("Error rotating vpn: {err}"));
            }
        }

        ctx.verboser().obtaining_task();

        let claimed = match scraper.claim(persist).await {
            Ok(claimed) => claimed,
            Err(err) => {
                ctx.verboser().error(&format!("Error claiming task: {err}"));
                let _ = sleep_cancellable(Duration::from_secs(1), ctx.cancellation()).await;
                continue;
            }
        };

        let Some((task_id, params)) = claimed else {
            ctx.verboser()
                .debug(&format!("Engine[{name}]: no pending tasks, stopping"));
            ctx.verboser().finished();
            break;
        };

        ctx.verboser().debug(&format!(
            "Engine[{name}]: claimed task id={task_id} at {}",
            scraper.describe(&params)
        ));
        ctx.verboser().claimed_task(&scraper.describe(&params));

        if ctx.verboser().is_cancelled() {
            ctx.verboser().warn("Cancelado antes de abrir el navegador");
            let _ = scraper.release(persist, task_id, None).await;
            break;
        }

        ctx.verboser()
            .debug(&format!("Engine[{name}]: scraping task id={task_id}..."));
        let started = Instant::now();
        let result = scraper.scrape(config, &params, ctx, persist).await;
        ctx.verboser()
            .debug(&format!(
                "Engine[{name}]: scrape of task id={task_id} took {:?}",
                started.elapsed()
            ));

        match result {
            Ok(result) => {
                ctx.verboser().debug(&format!(
                    "Engine[{name}]: task id={task_id} produced {} results (has_more={})",
                    result.coincidences.len(),
                    result.has_more
                ));
                ctx.verboser().writing_coincidences(&result.coincidences);
                let items = result.coincidences.len() as i32;
                let outcome = match persist
                    .write_coincidences(scraper.name(), result.coincidences)
                    .await
                {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        ctx.verboser().error(&format!("Error writing coincidences: {err}"));
                        let _ = scraper.release(persist, task_id, None).await;
                        iteration += 1;
                        continue;
                    }
                };
                ctx.verboser()
                    .written_coincidences(outcome.inserted, outcome.inserted_with_phone);
                let duplicated = items - outcome.inserted as i32;
                ctx.verboser().debug(&format!(
                    "Engine[{name}]: task id={task_id} write outcome: inserted={}, \
                     inserted_with_phone={}, duplicated={duplicated}",
                    outcome.inserted, outcome.inserted_with_phone
                ));
                scraper
                    .release(persist, task_id, Some((items, duplicated)))
                    .await
                    .ok();
                ctx.verboser().released_task();

                if let Err(err) = scraper.advance(persist, &params, result.has_more).await {
                    ctx.verboser().error(&format!("Error advancing queue: {err}"));
                } else {
                    ctx.verboser().debug(&format!(
                        "Engine[{name}]: task id={task_id} advanced (has_more={})",
                        result.has_more
                    ));
                }
            }
            Err(err) => {
                ctx.verboser().error(&format!("Error during scraping: {err}"));
                let _ = scraper.release(persist, task_id, None).await;
                iteration += 1;
                continue;
            }
        }

        iteration += 1;
    }

    ctx.verboser()
        .debug(&format!("Engine[{name}]: run_target finished after {iteration} iterations"));


    let mut iteration: u32 = 0;
    let mut timestamps: Vec<Instant> = Vec::new();

    loop {
        if ctx.verboser().is_cancelled() {
            ctx.verboser().warn("Operation canceled by the user");
            break;
        }

        if let Some(iterations) = scraper.iterations(config) {
            if iterations.get() <= iteration {
                ctx.verboser().finished();
                break;
            }
        }

        if let Some(rate_limit) = scraper.rate_limit(config) {
            let now = Instant::now();
            timestamps.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);
            if timestamps.len() >= rate_limit.get() as usize {
                let oldest = timestamps[0];
                let wait = RATE_LIMIT_WINDOW
                    .checked_sub(now.duration_since(oldest))
                    .unwrap_or_default();
                ctx.verboser().rate_limit_wait(wait);
                if sleep_cancellable(wait, ctx.cancellation()).await.is_err() {
                    break;
                }
            }
            timestamps.push(Instant::now());
        }

        if let Err(err) = ctx.vpn_tick().await {
            ctx.verboser().error(&format!("Error rotating vpn: {err}"));
        }

        ctx.verboser().obtaining_task();

        let claimed = match scraper.claim(persist).await {
            Ok(claimed) => claimed,
            Err(err) => {
                ctx.verboser().error(&format!("Error claiming task: {err}"));
                let _ = sleep_cancellable(Duration::from_secs(1), ctx.cancellation()).await;
                continue;
            }
        };

        let Some((task_id, params)) = claimed else {
            ctx.verboser().finished();
            break;
        };

        ctx.verboser().claimed_task(&scraper.describe(&params));

        if ctx.verboser().is_cancelled() {
            ctx.verboser().warn("Cancelado antes de abrir el navegador");
            let _ = scraper.release(persist, task_id, None).await;
            break;
        }
        let result = scraper.scrape(config, &params, ctx, persist).await;

        match result {
            Ok(result) => {
                ctx.verboser().writing_coincidences(&result.coincidences);
                let items = result.coincidences.len() as i32;
                let outcome = match persist
                    .write_coincidences(scraper.name(), result.coincidences)
                    .await
                {
                    Ok(outcome) => outcome,
                    Err(err) => {
                        ctx.verboser().error(&format!("Error writing coincidences: {err}"));
                        let _ = scraper.release(persist, task_id, None).await;
                        iteration += 1;
                        continue;
                    }
                };
                ctx.verboser()
                    .written_coincidences(outcome.inserted, outcome.inserted_with_phone);
                let duplicated = items - outcome.inserted as i32;
                scraper
                    .release(persist, task_id, Some((items, duplicated)))
                    .await
                    .ok();
                ctx.verboser().released_task();

                if let Err(err) = scraper.advance(persist, &params, result.has_more).await {
                    ctx.verboser().error(&format!("Error advancing queue: {err}"));
                }
            }
            Err(err) => {
                ctx.verboser().error(&format!("Error during scraping: {err}"));
                let _ = scraper.release(persist, task_id, None).await;
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
    use crate::types::{DbConfig, EmpresiteConfig, GMapsConfig};
    use crate::verboser::DebugVerboser;

    fn test_config() -> Config {
        Config {
            gmaps: GMapsConfig {
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
                iterations: Some(std::num::NonZeroU32::new(1).unwrap()),
                ..Default::default()
            },
            execution_mode: ExecutionMode::Sequential,
            db: DbConfig::Sqlite { path: None },
            nordvpn_path: None,
            browser_path: None,
            browser_profile_dir: None,
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
