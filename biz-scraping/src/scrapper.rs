use chromiumoxide::Page;
use crate::browser_manager::BrowserInstance;
use crate::types::{Tintoreria, SearchConfig};
use crate::progress::ProgressReporter;
use anyhow::{Result, Context};
use std::time::Duration;

fn random_delay(min_ms: u64, max_ms: u64) -> Duration {
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

fn clean_maps_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    base.strip_suffix('/').unwrap_or(base).to_string()
}

async fn accept_cookies(page: &Page) -> Result<()> {
    tokio::time::sleep(Duration::from_secs(2)).await;

    let js = r#"
        (() => {
            const buttons = document.querySelectorAll('button');
            for (const btn of buttons) {
                const t = btn.textContent.toLowerCase().trim();
                if (t.includes('aceptar todo') || t.includes('accept all') ||
                    t === 'aceptar' || t === 'accept') {
                    btn.click();
                    return true;
                }
            }
            const ariaEls = document.querySelectorAll('[aria-label]');
            for (const el of ariaEls) {
                const label = el.getAttribute('aria-label').toLowerCase();
                if (label.includes('aceptar') || label.includes('accept')) {
                    el.click();
                    return true;
                }
            }
            return false;
        })()
    "#;

    let _ = page.evaluate(js).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}

async fn extract_current_result(page: &Page) -> Result<Tintoreria> {
    let js = r#"
        (() => {
            const name = document.querySelector('.fontHeadline')?.textContent?.trim() || '';

            const phoneBtn = document.querySelector(
                'button[data-tooltip*="tel\u00e9fono"], button[data-tooltip*="phone"], [data-item-id*="phone"]'
            );
            let phone = '';
            if (phoneBtn) {
                const ariaLabel = phoneBtn.querySelector('[aria-label]');
                if (ariaLabel) phone = ariaLabel.getAttribute('aria-label') || '';
                if (!phone) phone = phoneBtn.textContent?.trim() || '';
            }

            const emailLink = document.querySelector('a[href^="mailto:"]');
            const email = emailLink ? emailLink.href.replace('mailto:', '') : '';

            const webBtn = document.querySelector(
                'button[data-tooltip*="sitio web"], [data-item-id*="authority"]'
            );
            let web = '';
            if (webBtn) {
                const link = webBtn.querySelector('a');
                if (link) web = link.href;
                if (!web) web = webBtn.getAttribute('aria-label') || '';
            }
            if (!web) {
                const alt = document.querySelector(
                    'a[aria-label*="Sitio web"], a[aria-label*="Website"]'
                );
                if (alt) web = alt.href;
            }

            return JSON.stringify({ name, phone, email, web });
        })()
    "#;

    let json_str: String = page
        .evaluate(js)
        .await
        .context("failed to evaluate extraction JS")?
        .into_value()
        .context("failed to extract result value")?;

    let data: serde_json::Value =
        serde_json::from_str(&json_str).context("failed to parse extraction JSON")?;

    let current_url: String = page
        .evaluate("window.location.href")
        .await
        .context("failed to get current URL")?
        .into_value()
        .context("failed to extract URL value")?;

    let maps_url = clean_maps_url(&current_url);

    Ok(Tintoreria {
        nombre: data["name"].as_str().unwrap_or_default().to_string(),
        tfno: data["phone"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        email: data["email"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        web: data["web"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        maps_url,
    })
}

async fn scroll_feed(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        const feed = document.querySelector('[role="feed"]');
        if (feed) feed.scrollTop = feed.scrollHeight;
    "#,
    )
    .await?;
    Ok(())
}

async fn scrape_feed(
    page: &Page,
    config: &SearchConfig,
    progress: &dyn ProgressReporter,
) -> Result<Vec<Tintoreria>> {
    let mut results = Vec::new();
    let mut scrolls_without_new = 0u32;
    let mut processed = 0usize;

    page.find_element("[role=\"feed\"]")
        .await
        .context("feed container not found")?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    loop {
        let total: i32 = page
            .evaluate("document.querySelectorAll('[role=\"feed\"] a').length")
            .await
            .context("failed to count feed items")?
            .into_value()
            .context("failed to extract count")?;
        let total = total as usize;

        while processed < total {
            let js = format!(
                "document.querySelectorAll('[role=\"feed\"] a')[{}].click()",
                processed
            ); 
            page.evaluate(js.as_str()).await?;
            tokio::time::sleep(Duration::from_millis(800)).await;

            match extract_current_result(page).await {
                Ok(t) => {
                    if !t.nombre.is_empty() {
                        progress.on_status(&format!("Procesando {}...", t.nombre));
                        results.push(t);
                    }
                }
                Err(e) => {
                    progress.on_error(&format!("Error extrayendo resultado: {}", e));
                }
            }

            processed += 1;
        }

        scroll_feed(page).await?;
        tokio::time::sleep(random_delay(config.delay_min, config.delay_max)).await;
        tokio::time::sleep(Duration::from_secs(1)).await;

        let new_total: i32 = page
            .evaluate("document.querySelectorAll('[role=\"feed\"] a').length")
            .await?
            .into_value()?;

        if (new_total as usize) <= total {
            scrolls_without_new += 1;
        } else {
            scrolls_without_new = 0;
        }

        if scrolls_without_new >= 5 {
            break;
        }
    }

    Ok(results)
}

async fn scrape_single(page: &Page) -> Result<Vec<Tintoreria>> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    if page.find_element(".fontHeadline").await.is_ok() {
        let t = extract_current_result(page).await?;
        if !t.nombre.is_empty() {
            return Ok(vec![t]);
        }
    }
    Ok(Vec::new())
}

pub async fn buscar(
    instance: &BrowserInstance,
    config: &SearchConfig,
    progress: &dyn ProgressReporter,
) -> Result<Vec<Tintoreria>> {
    let url = format!(
        "https://www.google.com/maps/search/{}/@{},{},{}z",
        config.search_query, config.lat, config.lng, config.zoom
    );

    progress.on_status("Navegando a Google Maps...");
    let page = instance.browser().new_page(&url).await.context("failed to create page")?;

    page.wait_for_navigation().await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    progress.on_status("Aceptando cookies...");
    let _ = accept_cookies(&page).await;

    progress.on_status("Buscando resultados...");

    let has_feed = page.find_element("[role=\"feed\"]").await.is_ok();
    let results = if has_feed {
        progress.on_status("Cargando mas resultados...");
        scrape_feed(&page, config, progress).await?
    } else {
        scrape_single(&page).await?
    };

    Ok(results)
}
