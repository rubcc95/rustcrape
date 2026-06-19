use std::path::Path;
use std::time::{Duration, Instant};

use crate::{
    browser::Browser,
    db::PersistenceKind,
    scraper::scrape,
    storage::Persistence,
    types::{Config, SearchContext},
    verboser::Verboser,
};
use anyhow::Result;

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(3600);

pub async fn run_dispatch(mut config: Config, verboser: impl Verboser) {
    let persist = loop {
        match PersistenceKind::create(&config.db, &mut config.search, &verboser).await {
            Ok(p) => break p,
            Err(err) => verboser.warn(&format!(
                "Failed to connect to database: {err}. Retrying..."
            )),
        }
    };
    run(config, verboser, persist).await;
}

pub async fn run<P: Persistence>(
    config: Config,
    verboser: impl Verboser,
    persist: P,
) {
    let mut iteration = 0;
    loop {
        match run_inner(&persist, config.clone(), &verboser, iteration).await {
            Ok(Output::Completed) => break,
            Ok(Output::Continue) => {}
            Err(err) => {
                verboser.error(&format!("Error during scraping: {err}. Retrying..."));
            }
        }
        iteration += 1;
    }
}

enum Output {
    Completed,
    Continue,
}

async fn run_inner<P: Persistence>(
    persist: &P,
    config: Config,
    verboser: &impl Verboser,
    iteration: u32,
) -> Result<Output> {
    let mut timestamps: Vec<Instant> = Vec::new();

    if verboser.is_cancelled() {
        verboser.warn("Operation canceled by the user");
        return Ok(Output::Completed);
    }

    if let Some(iterations) = config.iterations {
        if iterations.get() <= iteration {
            verboser.finished();
            return Ok(Output::Completed);
        }
    }

    if let Some(ref vpn_path) = config.nordvpn_path {
        if config.ip_rotation_frequency > 0 && iteration % config.ip_rotation_frequency == 0 {
            let path = std::path::Path::new(vpn_path);
            if crate::vpn::nordvpn_available(path) {
                crate::vpn::rotate_vpn(path, verboser).await;
            } else {
                verboser.vpn_not_available();
            }
        }
    }

    if let Some(rate_limit) = config.rate_limit {
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

    verboser.obtaining_bound();

    let Some((bound_id, lat, lng)) = persist.read_bound().await? else {
        verboser.finished();
        return Ok(Output::Completed);
    };
    if !persist.claim_bound(bound_id).await? {
        verboser.already_claimed_bound(lat, lng);
        return Ok(Output::Continue);
    }

    let ctx = SearchContext {
        lat,
        lng,
        config: &config,
    };

    if let Err(err) = run_claimed(persist, ctx, bound_id, config.browser_path.as_deref().map(Path::new), verboser).await {
        eprintln!(
            "Error al procesar bound: lat = {}, lng = {}, error = {:?}",
            lat, lng, err
        );
        verboser.warn(&format!("Liberando bound tras error: lat = {}, lng = {}", lat, lng));
        let _ = persist.release_bound(bound_id, None).await;
        return Err(err);
    }

    Ok(Output::Continue)
}

async fn run_claimed<P: Persistence>(
    persist: &P,
    ctx: SearchContext<'_>,
    bound_id: i64,
    browser_path: Option<&Path>,
    verboser: &impl Verboser,
) -> Result<()> {
    if verboser.is_cancelled() {
        verboser.warn("Cancelado antes de abrir el navegador");
        return Ok(());
    }
    verboser.opening_browser(ctx.lat, ctx.lng);
    let browser = Browser::launch(ctx.headless, browser_path).await?;
    let output = scrape(&browser, ctx, verboser).await?;
    verboser.closing_browser();
    if let Err(err) = browser.close().await {
        return Err(err.into());
    }
    let items = output.len() as i32;
    let phones = output.iter().filter(|c| c.tfno.is_some()).count() as i32;
    verboser.writing_coincidences(&output);
    let (written, phones) = persist.write_coincidences(output).await?;

    verboser.written_coincidences(written, phones);

    persist
        .release_bound(bound_id, Some((items, items - written as i32)))
        .await?;
    verboser.released_bound();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        types::{DbConfig, PersistentConfig, SearchConfig},
        verboser::DebugVerboser,
    };

    fn test_config() -> Config {
        Config {
            search: SearchConfig {
                persistent: PersistentConfig {
                    zoom: 12,
                    search_query: "Tintorería".to_string(),
                },
                stop_threshold: 3,
                delay_min: 500,
                delay_max: 2000,
                headless: false,
            },
            rate_limit: None,
            iterations: None,
            db: DbConfig::Mysql {
                host: "localhost".to_string(),
                port: 3306,
                user: "biz_user".to_string(),
                password: "biz_pass".to_string(),
                database: "rustcrape".to_string(),
            },
            nordvpn_path: None,
            browser_path: None,
            ip_rotation_frequency: 0,
        }
    }

    #[tokio::test]
    async fn test_run() {
        let config = test_config();
        run_dispatch(config, DebugVerboser).await;
    }
}
