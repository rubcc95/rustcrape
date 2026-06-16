use std::path::Path;
use std::time::{Duration, Instant};

use crate::{
    browser::Browser,
    db::{DbConnection, DbManager, DbUtility},
    scrapper::scrape,
    types::{Config, SearchContext},
    verboser::Verboser,
};
use anyhow::Result;

pub async fn run(mut config: Config, verboser: impl Verboser) -> Result<()> {
    let db = DbManager::new(&config.db, &mut config.search, &verboser).await?;

    let window = Duration::from_secs(3600);
    let mut timestamps: Vec<Instant> = Vec::new();

    let mut i = 0;

    loop {
        if verboser.is_cancelled() {
            verboser.warn("Procesamiento cancelado por el usuario");
            return Ok(());
        }

        if config.iterations > 0 && i >= config.iterations {
            break;
        }
        i += 1;

        if let Some(ref vpn_path) = config.nordvpn_path {
            if config.ip_rotation_frequency > 0 && i % config.ip_rotation_frequency == 0 {
                let path = std::path::Path::new(vpn_path);
                if crate::vpn::nordvpn_available(path) {
                    crate::vpn::rotate_vpn(path, &verboser).await;
                } else {
                    verboser.vpn_not_available();
                }
            }
        }

        if config.rate_limit > 0 {
            let now = Instant::now();
            timestamps.retain(|t| now.duration_since(*t) < window);
            if timestamps.len() >= config.rate_limit as usize {
                let oldest = timestamps[0];
                let wait = window
                    .checked_sub(now.duration_since(oldest))
                    .unwrap_or_default();
                verboser.rate_limit_wait(wait);
                tokio::time::sleep(wait).await;
            }
            timestamps.push(Instant::now());
        }

        verboser.obtaining_bound();
        let mut conn = db.connect().await?;

        // No bound to process, exit the loop
        let Some((lat, lng)) = conn.read_bound().await? else {
            verboser.finished();
            return Ok(());
        };
        if !conn.claim_bound(lat, lng).await? {
            verboser.already_claimed_bound(lat, lng);
            continue;
        }
        drop(conn);

        let ctx = SearchContext {
            lat,
            lng,
            config: &config,
        };

        if let Err((conn, err)) = run_claimed(&db, ctx, config.browser_path.as_deref().map(Path::new), &verboser).await {
            eprintln!(
                "Error al procesar bound: lat = {}, lng = {}, error = {:?}",
                lat, lng, err
            );
            let mut conn = match conn {
                Some(conn) => conn,
                None => db.connect().await?,
            };
            eprintln!("Liberando bound: lat = {}, lng = {}", lat, lng);
            let modified = conn.release_bound(lat, lng, None).await?;
            println!("Release bound had effect: {modified}");
            return Err(err);
        }
    }
    Ok(())
}

async fn run_claimed(
    db: &DbManager,
    ctx: SearchContext<'_>,
    browser_path: Option<&Path>,
    verboser: &impl Verboser,
) -> std::result::Result<(), (Option<DbConnection>, anyhow::Error)> {
    if verboser.is_cancelled() {
        verboser.warn("Cancelado antes de abrir el navegador");
        return Ok(());
    }
    verboser.opening_browser(ctx.lat, ctx.lng);
    let browser = Browser::launch(ctx.headless, browser_path)
        .await
        .map_err(|err| (None, err))?;
    let output = scrape(&browser, ctx, verboser)
        .await
        .map_err(|err| (None, err))?;
    verboser.closing_browser();
    if let Err(err) = browser.close().await {
        return Err((None, err.into()));
    }
    verboser.writing_coincidences(&output);
    let mut conn = db.connect().await.map_err(|err| (None, err))?;

    let items = output.len() as i32;
    let written = match conn.write_coincidences(output).await {
        Ok(result) => result.rows_affected(),
        Err(err) => return Err((Some(conn), err.into())),
    } as i32;

    verboser.written_coincidences(written);
    // println!(
    //     "Bound lat = {}, lng = {}: {} coincidences encontradas, {} insertadas",
    //     ctx.lat, ctx.lng, items, written
    // );

    if let Err(err) = conn
        .release_bound(ctx.lat, ctx.lng, Some((items, items - written)))
        .await
    {
        return Err((Some(conn), err.into()));
    }
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
            rate_limit: 0,
            iterations: 0,
            db: DbConfig {
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
        run(config, DebugVerboser).await.unwrap();
    }
}
