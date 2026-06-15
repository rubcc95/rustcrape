use crate::types::{Coincidence, SearchContext};
use crate::verboser::Verboser;
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType,
};
use chromiumoxide::error::CdpError;
use chromiumoxide::{Browser, Element, Page};
use futures::future::{BoxFuture, select_ok};
use futures::prelude::*;
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

async fn wait_until<F: Future<Output = Result<Option<T>>>, T>(
    mut f: impl FnMut() -> F,
    interval: Duration,
    timeout: Duration,
) -> Result<T> {
    let start = std::time::Instant::now();
    loop {
        if let Some(d) = f().await? {
            return Ok(d);
        }
        if start.elapsed() >= timeout {
            return Err(anyhow::anyhow!("wait_until timed out"));
        }

        tokio::time::sleep(interval).await;
    }
}

fn clean_maps_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    base.strip_suffix('/').unwrap_or(base).to_string()
}

async fn accept_cookies(page: &Page) -> Result<()> {
    let el = accept_cookies_button(page).await?;
    if let Some(el) = el {
        el.click().await?;
    }
    Ok(())
}

async fn accept_cookies_button(page: &Page) -> Result<Option<Element>> {
    // Try buttons: button, [role="button"], input[type="submit"], input[type="button"]
    let buttons = page
        .find_elements("button, [role=\"button\"], input[type=\"submit\"], input[type=\"button\"]")
        .await?;

    for btn in buttons {
        let t = {
            let text = btn
                .string_property("textContent")
                .await?
                .unwrap_or_default();
            if !text.is_empty() {
                text.trim().to_lowercase()
            } else {
                btn.string_property("value")
                    .await?
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase()
            }
        };
        if t.contains("aceptar todo")
            || t.contains("accept all")
            || t.contains("aceptar")
            || t.contains("accept")
            || t.contains("accepteer")
            || t.contains("alle akzeptieren")
        {
            return Ok(Some(btn));
        }
    }

    // Try aria-label elements
    let aria_els = page.find_elements("[aria-label]").await?;
    for el in aria_els {
        if let Some(label) = el.attribute("aria-label").await? {
            let lower = label.to_lowercase();
            if lower.contains("aceptar")
                || lower.contains("accept")
                || lower.contains("aceptar todo")
                || lower.contains("accept all")
            {
                return Ok(Some(el));
                // el.click().await?;
                // eprintln!("[DEBUG] accept_cookies result: aria:{}", label);
                // tokio::time::sleep(Duration::from_millis(500)).await;
                // return Ok(());
            }
        }
    }

    // Try form submit buttons
    let forms = page.find_elements("form").await?;
    for form in &forms {
        if let Ok(submit_btn) = form
            .find_element("button[type=\"submit\"], input[type=\"submit\"]")
            .await
        {
            let t = {
                let text = submit_btn
                    .string_property("textContent")
                    .await?
                    .unwrap_or_default();
                if !text.is_empty() {
                    text.trim().to_lowercase()
                } else {
                    submit_btn
                        .string_property("value")
                        .await?
                        .unwrap_or_default()
                        .trim()
                        .to_lowercase()
                }
            };
            if t.contains("aceptar") || t.contains("accept") {
                return Ok(Some(submit_btn));
                // submit_btn.click().await?;
                // eprintln!("[DEBUG] accept_cookies result: form:{}", t);
                // tokio::time::sleep(Duration::from_millis(500)).await;
                // return Ok(None);
            }
        }
    }

    return Ok(None);
    // eprintln!("[DEBUG] accept_cookies result: notfound");
    // tokio::time::sleep(Duration::from_millis(500)).await;
    // Ok(())
}

fn strip_website(label: &str) -> String {
    let label = label.trim();
    let lower = label.to_lowercase();
    if lower.starts_with("sitio web") {
        let after = &label["sitio web".len()..];
        let after = if after.starts_with(':') {
            &after[1..]
        } else {
            after
        };
        after.trim().to_string()
    } else {
        label.to_string()
    }
}

fn extract_phone_suffix(label: &str) -> Option<String> {
    let trimmed = label.trim_end();
    let start = trimmed.rfind(|c: char| !c.is_ascii_digit() && !c.is_whitespace() && c != '+');
    let phone = match start {
        Some(pos) => trimmed[pos + 1..].trim(),
        None => trimmed,
    };
    if phone.is_empty() {
        None
    } else {
        Some(phone.to_string())
    }
}

async fn extract_current_result(
    page: &Page,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let phone = {
        let sel = concat!(
            "button[data-tooltip*=\"teléfono\"], ",
            "button[data-tooltip*=\"phone\"], ",
            "button[aria-label*=\"Teléfono\"], ",
            "[data-item-id*=\"phone\"], ",
            "[data-item-id^=\"phone:tel:\"]",
        );
        match page.find_element(sel).await {
            Ok(el) => {
                let aria = el.attribute("aria-label").await?;
                match aria.as_deref().and_then(extract_phone_suffix) {
                    Some(p) => Some(p),
                    None => el.string_property("textContent").await?,
                }
            }
            Err(err) => match err {
                CdpError::Chrome(chromiumoxide::types::Error { code: -32000, .. }) => None,
                err => return Err(anyhow::Error::new(err).context("failed to find phone element")),
            },
        }
    };

    let email = match page.find_element("a[href^=\"mailto:\"]").await {
        Ok(el) => el
            .attribute("href")
            .await?
            .map(|h| h.replace("mailto:", ""))
            .filter(|s| !s.is_empty()),
        Err(err) => match err {
            CdpError::Chrome(chromiumoxide::types::Error { code: -32000, .. }) => None,
            err => return Err(anyhow::Error::new(err).context("failed to find phone element")),
        },
    };

    let web = {
        let sel = concat!(
            "button[data-tooltip*=\"sitio web\"], ",
            "button[data-tooltip*=\"website\"], ",
            "[data-item-id*=\"authority\"], ",
            "a[aria-label*=\"Sitio web\"], ",
            "a[aria-label*=\"sitio web\"]",
        );
        match page.find_element(sel).await {
            Ok(el) => {
                let mut web = el
                    .attribute("aria-label")
                    .await?
                    .map(|a| strip_website(&a))
                    .filter(|s| !s.is_empty());

                if web.is_none() {
                    let tag = el.string_property("tagName").await?.unwrap_or_default();
                    if tag.eq_ignore_ascii_case("a") {
                        web = el.attribute("href").await?.filter(|s| !s.is_empty());
                    } else if let Ok(link) = el.find_element("a[href]").await {
                        web = link.attribute("href").await?.filter(|s| !s.is_empty());
                    }
                }

                if web.is_none() {
                    web = el.attribute("href").await?.filter(|s| !s.is_empty());
                }

                web
            }
            Err(err) => match err {
                CdpError::Chrome(chromiumoxide::types::Error { code: -32000, .. }) => None,
                err => return Err(anyhow::Error::new(err).context("failed to find phone element")),
            },
        }
    };

    Ok((phone, email, web))
}

async fn scrape_single(
    page: &Page,
    ctx: SearchContext<'_>,
    verboser: &impl Verboser,
) -> Result<Vec<Coincidence>> {
    verboser.found_single_coincidence();
    tokio::time::sleep(random_delay(ctx.delay_min, ctx.delay_max)).await;
    if page.find_element("h1.DUwDvf").await.is_ok() {
        let name_js = r#"( () => { const el = document.querySelector('h1.DUwDvf'); return el ? el.textContent.trim() : ''; })() "#;
        let name: String = page.evaluate(name_js).await?.into_value()?;
        let (tfno, email, web) = extract_current_result(page).await?;
        let current_url: String = page.evaluate("window.location.href").await?.into_value()?;
        if !name.is_empty() {
            verboser.processed_coincidence(&name, 1);
            return Ok(vec![Coincidence {
                name,
                tfno,
                email,
                web,
                maps_url: clean_maps_url(&current_url),
            }]);
        }
    }
    Ok(Vec::new())
}

fn extract_name_from_href(href: &str) -> String {
    let start = "/maps/place/";
    if let Some(pos) = href.find(start) {
        let rest = &href[pos + start.len()..];
        let segment = rest
            .split('/')
            .next()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
            .split('&')
            .next()
            .unwrap_or("")
            .replace('+', " ");
        // manual URL percent-decode (no lazy, no regex)
        let mut decoded = String::with_capacity(segment.len());
        let mut chars = segment.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                decoded.push(char::from((hi * 16 + lo) as u8));
            } else {
                decoded.push(c);
            }
        }
        return decoded;
    }
    String::new()
}

fn extract_coords_from_href(href: &str) -> (f32, f32) {
    let parse_after = |prefix: &str| -> f32 {
        href.split(prefix)
            .nth(1)
            .and_then(|rest| {
                let end = rest
                    .find(|c: char| !matches!(c, '-' | '.' | '0'..='9'))
                    .unwrap_or(rest.len());
                rest[..end].parse().ok()
            })
            .unwrap_or_default()
    };

    let lat = parse_after("!3d").max(parse_after("&3d"));
    let lng = parse_after("!4d").max(parse_after("&4d"));
    (lat, lng)
}

async fn scrape_feed(
    page: &Page,
    ctx: SearchContext<'_>,
    verboser: &impl Verboser,
) -> Result<Vec<Coincidence>> {
    verboser.found_multiple_coincidences();
    let mut coincidences = Vec::new();
    let mut scrolls_without_new = 0u32;
    let mut fuera = 0u32;

    let radio = 180.0 / (2u32.pow(ctx.zoom) as f32);

    loop {
        if verboser.is_cancelled() {
            return Ok(coincidences);
        }

        let elements = page.find_elements("a[href*=\"/maps/place/\"]").await?;
        let total = elements.len();

        for el in &elements[coincidences.len()..] {
            if verboser.is_cancelled() {
                return Ok(coincidences);
            }

            tokio::time::sleep(random_delay(ctx.delay_min, ctx.delay_max)).await;

            let Some(href) = el.attribute("href").await? else {
                continue;
            };

            let name = extract_name_from_href(&href);

            let (lat, lng) = extract_coords_from_href(&href);

            if lat != 0.0 && lng != 0.0 {
                let dist = (lat - ctx.lat).abs().max((lng - ctx.lng).abs());
                if dist > radio {
                    fuera += 1;
                    if fuera >= ctx.stop_threshold {
                        return Ok(coincidences);
                    }
                } else {
                    fuera = 0;
                }
            }

            el.click().await?;
            match extract_current_result(page).await {
                Ok((tfno, email, web)) => {
                    verboser.processed_coincidence(&name, coincidences.len() + 1);
                    coincidences.push(Coincidence {
                        name,
                        tfno,
                        email,
                        web,
                        maps_url: clean_maps_url(&href),
                    });
                }
                Err(e) => {
                    verboser.warn(&format!("Error extrayendo resultado: {}", e));
                }
            }
        }

        if let Ok(feed) = page.find_element("[role=\"feed\"]").await {
            if let Ok(bbox) = feed.bounding_box().await {
                let x = bbox.x + rand::random::<f64>() * bbox.width;
                let y = bbox.y + rand::random::<f64>() * bbox.height;
                let delta_y = 30.0 + rand::random::<f64>() * 400.0;
                let cmd = DispatchMouseEventParams::builder()
                    .r#type(DispatchMouseEventType::MouseWheel)
                    .x(x)
                    .y(y)
                    .delta_x(0.0)
                    .delta_y(delta_y)
                    .build()
                    .unwrap();
                let _ = page.execute(cmd).await;
            }
        }

        tokio::time::sleep(random_delay(ctx.delay_min, ctx.delay_max)).await;
        //tokio::time::sleep(Duration::from_secs(1)).await;

        let new_total = page.find_elements("a[href*=\"/maps/place/\"]").await?.len();

        if new_total <= total {
            scrolls_without_new += 1;
        } else {
            scrolls_without_new = 0;
        }

        if scrolls_without_new >= 5 {
            break;
        }
    }

    Ok(coincidences)
}

pub async fn scrape(
    instance: &Browser,
    ctx: SearchContext<'_>,
    verboser: &impl Verboser,
) -> Result<Vec<Coincidence>> {
    verboser.accepting_cookies();
    let url = format!(
        "https://www.google.com/maps/search/{}/@{},{},{}z",
        ctx.search_query, ctx.lat, ctx.lng, ctx.zoom
    );
    let page = instance.new_page(&url).await?;
    page.wait_for_navigation().await?;
    tokio::time::sleep(Duration::from_secs(3)).await;

    verboser.accepting_cookies();
    let _ = accept_cookies(&page).await;

    verboser.searching_coincidences();

    if verboser.is_cancelled() {
        return Ok(Vec::new());
    }

    enum Mode {
        Feed,
        Single,
    }

    enum SkipOrError {
        Skip,
        Error(anyhow::Error),
    }

    impl<T> From<T> for SkipOrError
    where
        anyhow::Error: From<T>,
    {
        fn from(err: T) -> Self {
            let e: anyhow::Error = anyhow::Error::from(err);
            SkipOrError::Error(e)
        }
    }

    type Output = std::result::Result<Mode, SkipOrError>;

    let mode = wait_until(
        || async {
            let a: BoxFuture<Output> = Box::pin(async {
                match page.find_element("[role=\"feed\"]").await {
                    Ok(_) => Ok(Mode::Feed),
                    Err(e) => match e {
                        CdpError::Chrome(chromiumoxide::types::Error { code: -32000, .. }) => {
                            Err(SkipOrError::Skip)
                        }
                        _ => Err(SkipOrError::Error(anyhow::Error::new(e))),
                    },
                }
            });

            let b: BoxFuture<Output> = Box::pin(async {
                let a = page.url().await?;

                if a.is_some_and(|a| a.contains("/maps/place/")) {
                    Ok(Mode::Single)
                } else {
                    Err(SkipOrError::Skip)
                }
            });

            match select_ok([a, b]).await {
                Ok((mode, _)) => Ok(Some(mode)),
                Err(SkipOrError::Error(err)) => Err(err),
                Err(SkipOrError::Skip) => Ok(None),
            }
        },
        Duration::from_millis(100),
        Duration::from_secs(10),
    )
    .await?;
    match mode {
        Mode::Feed => scrape_feed(&page, ctx, verboser).await,
        Mode::Single => scrape_single(&page, ctx, verboser).await,
    }
}
