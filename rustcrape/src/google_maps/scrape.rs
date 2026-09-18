use crate::browser::Browser;
use crate::context::Context;
use crate::google_maps::config::GMapsParams;
use crate::scraper::ScrapeResult;
use crate::types::{Coincidence, Config, GMapsConfig};
use crate::utils::*;
use crate::verboser::Verboser;

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType,
};
use chromiumoxide::error::CdpError;
use chromiumoxide::{Element, Page};
use futures::future::{BoxFuture, select_ok};
use std::time::Duration;

fn clean_maps_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    base.strip_suffix('/').unwrap_or(base).to_string()
}

async fn accept_cookies(page: &Page, verboser: &dyn Verboser) -> Result<()> {
    verboser.debug("Cookies: looking for consent banner");
    let el = accept_cookies_button(page, verboser).await?;
    match el {
        Some(el) => {
            verboser.debug("Cookies: accept button located, clicking");
            el.click().await?;
            verboser.debug("Cookies: consent accepted");
        }
        None => verboser.debug("Cookies: no consent banner found"),
    }
    Ok(())
}

async fn accept_cookies_button(
    page: &Page,
    verboser: &dyn Verboser,
) -> Result<Option<Element>> {
    // Try buttons: button, [role="button"], input[type="submit"], input[type="button"]
    let buttons = page
        .find_elements("button, [role=\"button\"], input[type=\"submit\"], input[type=\"button\"]")
        .await?;
    verboser.debug(&format!(
        "Cookies: scanning {} candidate buttons",
        buttons.len()
    ));

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
            verboser.debug(&format!("Cookies: match by button text: {t:?}"));
            return Ok(Some(btn));
        }
    }

    // Try aria-label elements
    verboser.debug("Cookies: no text match; trying aria-label");
    let aria_els = page.find_elements("[aria-label]").await?;
    for el in aria_els {
        if let Some(label) = el.attribute("aria-label").await? {
            let lower = label.to_lowercase();
            if lower.contains("aceptar")
                || lower.contains("accept")
                || lower.contains("aceptar todo")
                || lower.contains("accept all")
            {
                verboser.debug(&format!("Cookies: match by aria-label: {lower:?}"));
                return Ok(Some(el));
            }
        }
    }

    // Try form submit buttons
    verboser.debug("Cookies: no aria-label match; trying form submits");
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
                verboser.debug(&format!("Cookies: match by form submit: {t:?}"));
                return Ok(Some(submit_btn));
            }
        }
    }

    Ok(None)
}

fn strip_website(label: &str) -> String {
    let label = label.trim();
    let lower = label.to_lowercase();

    let prefixes = [
        "visitar el sitio web de ",
        "visitar sitio web de ",
        "visitar sitio web ",
        "sitio web de ",
        "sitio web: ",
        "sitio web ",
    ];

    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            return label[prefix.len()..].trim().to_string();
        }
    }

    label.to_string()
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
    verboser: &dyn Verboser,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    verboser.debug("Extraction: looking for phone/email/website in the panel");
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

    verboser.debug(&format!(
        "Extraction: phone={}",
        phone.as_deref().unwrap_or("<ninguno>")
    ));

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

    verboser.debug(&format!(
        "Extraction: email={}",
        email.as_deref().unwrap_or("<ninguno>")
    ));

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
                let mut web = el.attribute("href").await?.filter(|s| !s.is_empty());

                if web.is_none() {
                    let tag = el.string_property("tagName").await?.unwrap_or_default();
                    if tag.eq_ignore_ascii_case("a") {
                        web = el.attribute("href").await?.filter(|s| !s.is_empty());
                    } else if let Ok(link) = el.find_element("a[href]").await {
                        web = link.attribute("href").await?.filter(|s| !s.is_empty());
                    }
                }

                if web.is_none() {
                    web = el
                        .attribute("aria-label")
                        .await?
                        .map(|a| strip_website(&a))
                        .filter(|s| !s.is_empty());
                }

                web
            }
            Err(err) => match err {
                CdpError::Chrome(chromiumoxide::types::Error { code: -32000, .. }) => None,
                err => return Err(anyhow::Error::new(err).context("failed to find phone element")),
            },
        }
    };

    verboser.debug(&format!(
        "Extraction: website={}",
        web.as_deref().unwrap_or("<ninguna>")
    ));

    Ok((phone, email, web))
}

async fn scrape_single(
    page: &Page,
    params: &GMapsParams,
    config: &GMapsConfig,
    verboser: &dyn Verboser,
) -> Result<Vec<Coincidence>> {
    let _ = params;
    verboser.debug("Single mode: looking for a single-place panel");
    verboser.found_coincidences(Some(1));
    let delay = random_delay(config.delay_min, config.delay_max);
    verboser.debug(&format!("Single mode: waiting {delay:?} before extracting"));
    tokio::time::sleep(delay).await;
    if page.find_element("h1.DUwDvf").await.is_ok() {
        verboser.debug("Single mode: h1.DUwDvf title found");
        let name_js = r#"( () => { const el = document.querySelector('h1.DUwDvf'); return el ? el.textContent.trim() : ''; })() "#;
        let name: String = page.evaluate(name_js).await?.into_value()?;
        let (tfno, email, web) = extract_current_result(page, verboser).await?;
        let current_url: String = page.evaluate("window.location.href").await?.into_value()?;
        if !name.is_empty() {
            verboser.debug(&format!(
                "Single mode: result '{name}' (url={current_url})"
            ));
            verboser.processed_coincidence(&name, 1);
            return Ok(vec![Coincidence {
                name,
                tfno,
                email,
                web,
                source_url: clean_maps_url(&current_url),
                ..Default::default()
            }]);
        }
        verboser.debug("Single mode: empty title, no result");
    } else {
        verboser.debug("Single mode: place title not found");
    }
    Ok(Vec::new())
}

#[allow(dead_code)]
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
        // manual URL percent-decode, collecting bytes first for correct UTF-8
        let mut bytes = Vec::with_capacity(segment.len());
        let mut chars = segment.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hi = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                let lo = chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0);
                bytes.push((hi * 16 + lo) as u8);
            } else {
                // push each byte of the UTF-8 representation
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
            }
        }
        return String::from_utf8(bytes).unwrap_or_default();
    }
    String::new()
}

fn extract_coords_from_href(href: &str) -> (f32, f32) {
    // Formato esperado: /maps/place/Nombre/@lat,lng,zoom/...  o  ...!3dlat!4dlng
    // Prioridad: @lat,lng en la ruta (mas fiable)

    if let Some(at_pos) = href.find('@') {
        let after_at = &href[at_pos + 1..];
        let mut parts = after_at.splitn(3, ',');
        if let (Some(lat_str), Some(lng_str)) = (parts.next(), parts.next()) {
            if let (Ok(lat), Ok(lng)) = (lat_str.parse::<f32>(), {
                // El lng termina en la siguiente coma, /, o fin de cadena
                let end = lng_str
                    .find(|c: char| c == ',' || c == '/')
                    .unwrap_or(lng_str.len());
                lng_str[..end].parse::<f32>()
            }) {
                return (lat, lng);
            }
        }
    }

    // Fallback: !3d/!4d en data= (formato antiguo/menos comun)
    let parse_after = |prefix: &str| -> Option<f32> {
        href.split(prefix).nth(1).and_then(|rest| {
            let end = rest
                .find(|c: char| !matches!(c, '-' | '.' | '0'..='9'))
                .unwrap_or(rest.len());
            rest[..end].parse().ok()
        })
    };

    let lat = parse_after("!3d")
        .or_else(|| parse_after("&3d"))
        .unwrap_or(0.0);
    let lng = parse_after("!4d")
        .or_else(|| parse_after("&4d"))
        .unwrap_or(0.0);
    (lat, lng)
}

async fn scrape_feed(
    page: &Page,
    params: &GMapsParams,
    config: &GMapsConfig,
    verboser: &dyn Verboser,
) -> Result<Vec<Coincidence>> {
    verboser.debug("Feed mode: starting listing walk");
    verboser.found_coincidences(None);
    let mut coincidences = Vec::new();
    let mut scrolls_without_new = 0u32;
    let mut fuera = 0u32;

    let radio = 180.0 / (2u32.pow(config.zoom) as f32);
    verboser.debug(&format!(
        "Feed mode: zoom={} stop radius={radio}, stop_threshold={}",
        config.zoom, config.stop_threshold
    ));

    loop {
        if verboser.is_cancelled() {
            verboser.debug("Feed mode: cancelled by user, returning partial results");
            return Ok(coincidences);
        }

        let elements = page.find_elements("a[href*=\"/maps/place/\"]").await?;
        let total = elements.len();
        verboser.debug(&format!(
            "Feed mode: {total} place links in DOM ({} already processed)",
            coincidences.len()
        ));

        for el in &elements[coincidences.len()..] {
            if verboser.is_cancelled() {
                verboser.debug("Feed mode: cancelled while walking links");
                return Ok(coincidences);
            }

            let delay = random_delay(config.delay_min, config.delay_max);
            if verboser.debug_enabled() {
                verboser.debug(&format!(
                    "Feed mode: waiting {delay:?} before next link"
                ));
            }
            tokio::time::sleep(delay).await;

            let Some(href) = el.attribute("href").await? else {
                verboser.debug("Feed mode: link without href, skipping");
                continue;
            };

            let (lat, lng) = extract_coords_from_href(&href);

            if lat != 0.0 && lng != 0.0 {
                let dist = (lat - params.lat).abs().max((lng - params.lng).abs());
                if dist > radio {
                    fuera += 1;
                    verboser.debug(&format!(
                        "Feed mode: {href} out of radius (dist={dist} > {radio}), \
                         out counter={fuera}/{}",
                        config.stop_threshold
                    ));
                    if fuera >= config.stop_threshold {
                        verboser.debug(
                            "Feed mode: out-of-radius stop threshold reached, stopping",
                        );
                        return Ok(coincidences);
                    }
                } else {
                    if fuera > 0 {
                        verboser
                            .debug("Feed mode: result inside radius, resetting out counter");
                    }
                    fuera = 0;
                }
            } else {
                verboser.debug(&format!(
                    "Feed mode: no coordinates in {href}, skipping radius filter"
                ));
            }

            let old_name: String = page
                .evaluate("document.querySelector('h1.DUwDvf')?.textContent?.trim() ?? ''")
                .await?
                .into_value()?;

            verboser.debug(&format!(
                "Feed mode: clicking link (previous name: {old_name:?})"
            ));
            el.click().await?;

            let _ = wait_until(
                || async {
                    let current: String = page
                        .evaluate("document.querySelector('h1.DUwDvf')?.textContent?.trim() ?? ''")
                        .await?
                        .into_value()?;
                    Ok((current != old_name && !current.is_empty()).then_some(()))
                },
                Duration::from_millis(100),
                Duration::from_secs(5),
            )
            .await;

            let panel_name: String = page
                .evaluate("document.querySelector('h1.DUwDvf')?.textContent?.trim() ?? ''")
                .await?
                .into_value()?;

            if panel_name.is_empty() {
                verboser.debug("Feed mode: panel has no title after click, skipping");
                continue;
            }

            if panel_name == old_name {
                verboser.debug(&format!(
                    "Feed mode: panel did not change from '{old_name}' (possible timeout)"
                ));
            }

            match extract_current_result(page, verboser).await {
                Ok((tfno, email, web)) => {
                    verboser.debug(&format!(
                        "Feed mode: result '{panel_name}' (phone={}, email={}, website={})",
                        tfno.as_deref().unwrap_or("-"),
                        email.as_deref().unwrap_or("-"),
                        web.as_deref().unwrap_or("-"),
                    ));
                    verboser.processed_coincidence(&panel_name, coincidences.len() + 1);
                    coincidences.push(Coincidence {
                        name: panel_name,
                        tfno,
                        email,
                        web,
                        source_url: clean_maps_url(&href),
                        ..Default::default()
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
                verboser.debug(&format!(
                    "Feed mode: scrolling feed at ({x:.0},{y:.0}) delta_y={delta_y:.0}"
                ));
                let cmd = DispatchMouseEventParams::builder()
                    .r#type(DispatchMouseEventType::MouseWheel)
                    .x(x)
                    .y(y)
                    .delta_x(0.0)
                    .delta_y(delta_y)
                    .build()
                    .unwrap();
                let _ = page.execute(cmd).await;
            } else {
                verboser.debug("Feed mode: could not get feed bounding box");
            }
        } else {
            verboser.debug("Feed mode: [role=feed] container not found");
        }

        let delay = random_delay(config.delay_min, config.delay_max);
        if verboser.debug_enabled() {
            verboser.debug(&format!("Feed mode: waiting {delay:?} after scroll"));
        }
        tokio::time::sleep(delay).await;

        let new_total = page.find_elements("a[href*=\"/maps/place/\"]").await?.len();

        if new_total <= total {
            scrolls_without_new += 1;
        } else {
            scrolls_without_new = 0;
        }
        verboser.debug(&format!(
            "Feed mode: links after scroll {new_total} (before {total}), \
             scrolls without new={scrolls_without_new}/5"
        ));

        if scrolls_without_new >= 5 {
            verboser.debug("Feed mode: 5 scrolls without new links, end of listing");
            break;
        }
    }

    verboser.debug(&format!(
        "Feed mode: walk finished with {} results",
        coincidences.len()
    ));
    Ok(coincidences)
}

pub async fn scrape(
    params: &GMapsParams,
    config: &Config,
    ctx: &Context,
) -> Result<ScrapeResult> {
    let verboser = ctx.verboser();
    verboser.debug(&format!(
        "Google Maps: starting scrape at ({}, {}) with query '{}' zoom={}",
        params.lat, params.lng, config.gmaps.search_query, config.gmaps.zoom
    ));
    verboser.opening_browser(&format!("({}, {})", params.lat, params.lng));
    let mut instance = Browser::gmaps(config, verboser).await?;

    let scrape = scrape_internal(&instance, params, config, verboser).await;

    verboser.closing_browser();
    instance.close(verboser).await?;
    scrape
}

async fn scrape_internal(
    browser: &Browser,
    params: &GMapsParams,
    config: &Config,
    verboser: &dyn Verboser,
) -> Result<ScrapeResult> {
    let url = format!(
        "https://www.google.com/maps/search/{}/@{},{},{}z",
        config.gmaps.search_query, params.lat, params.lng, config.gmaps.zoom
    );
    verboser.debug(&format!("Google Maps: search URL {url}"));
    let page = browser.new_page(&url, verboser).await?;
    verboser.debug("Google Maps: waiting for initial navigation");
    page.wait_for_navigation().await?;
    verboser.debug("Google Maps: navigation completed, waiting for render (3s)");
    tokio::time::sleep(Duration::from_secs(3)).await;

    if verboser.is_cancelled() {
        verboser.debug("Google Maps: cancelled before accepting cookies");
        return Ok(ScrapeResult::empty(false));
    }

    verboser.accepting_cookies();
    let _ = accept_cookies(&page, verboser).await;

    verboser.searching_coincidences();
    if verboser.is_cancelled() {
        verboser.debug("Google Maps: cancelled before searching results");
        return Ok(ScrapeResult::empty(false));
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

    verboser.debug("Google Maps: detecting mode (feed listing vs single place), timeout 10s");
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

    let coincidences = match mode {
        Mode::Feed => {
            verboser.debug("Google Maps: Feed mode detected");
            scrape_feed(&page, params, &config.gmaps, verboser).await?
        }
        Mode::Single => {
            verboser.debug("Google Maps: Single mode detected");
            scrape_single(&page, params, &config.gmaps, verboser).await?
        }
    };

    verboser.debug(&format!(
        "Google Maps: scrape finished with {} results",
        coincidences.len()
    ));
    Ok(ScrapeResult::new(coincidences, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_coords_from_href_con_coordenadas_negativas_y_positivas() {
        // Espana peninsular: lat positiva, lng negativa
        let (lat, lng) = extract_coords_from_href(
            "https://www.google.com/maps/place/Tintorer%C3%ADa+Centro/@40.4168,-3.7038,17z/data=!3m1!4b1",
        );
        assert!(
            (lat - 40.4168).abs() < 0.001,
            "lat esperada 40.4168, got {}",
            lat
        );
        assert!(
            (lng - -3.7038).abs() < 0.001,
            "lng esperada -3.7038, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_desde_at_con_zoom_y_data() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Dry+Cleaners/@-34.6037,-58.3816,15z/data=!4m6!3m5!1s0x95bcc9:0x12345",
        );
        assert!(
            (lat - -34.6037).abs() < 0.001,
            "lat esperada -34.6037, got {}",
            lat
        );
        assert!(
            (lng - -58.3816).abs() < 0.001,
            "lng esperada -58.3816, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_sin_data_solo_at() {
        let (lat, lng) = extract_coords_from_href(
            "https://www.google.com/maps/place/Tintorer%C3%ADa+Nombre/@41.3879,2.1699,17z",
        );
        assert!(
            (lat - 41.3879).abs() < 0.001,
            "lat esperada 41.3879, got {}",
            lat
        );
        assert!(
            (lng - 2.1699).abs() < 0.001,
            "lng esperada 2.1699, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_sin_arroba_fallback_a_3d_4d() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Tintorer%C3%ADa+Premium/data=!4m7!3m6!1s0xdead:0xbeef!8m2!3d41.3800!4d2.1700",
        );
        assert!(
            (lat - 41.3800).abs() < 0.001,
            "lat esperada 41.3800, got {}",
            lat
        );
        assert!(
            (lng - 2.1700).abs() < 0.001,
            "lng esperada 2.1700, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_fallback_3d_con_longitud_negativa() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Negocio/data=!3m1!4b1!4m6!3m5!1s0xdead:0xbeef!8m2!3d40.4168!4d-3.7038",
        );
        assert!(
            (lat - 40.4168).abs() < 0.001,
            "lat esperada 40.4168, got {}",
            lat
        );
        assert!(
            (lng - -3.7038).abs() < 0.001,
            "lng esperada -3.7038, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_fallback_3d_con_longitud_positiva() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Negocio/data=!4m7!3m6!1s0xdead:0xbeef!8m2!3d41.3800!4d2.1700",
        );
        assert!(
            (lat - 41.3800).abs() < 0.001,
            "lat esperada 41.3800, got {}",
            lat
        );
        assert!(
            (lng - 2.1700).abs() < 0.001,
            "lng esperada 2.1700, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_fallback_3d_con_coordenadas_negativas() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Negocio/data=!4m6!3m5!1s0xdead:0xbeef!8m2!3d-34.6037!4d-58.3816",
        );
        assert!(
            (lat - -34.6037).abs() < 0.001,
            "lat esperada -34.6037, got {}",
            lat
        );
        assert!(
            (lng - -58.3816).abs() < 0.001,
            "lng esperada -58.3816, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_sin_coordenadas_devuelve_cero() {
        let (lat, lng) = extract_coords_from_href("/maps/place/Tintorer%C3%ADa+Sin+Coords");
        assert!(
            (lat - 0.0).abs() < f32::EPSILON,
            "lat esperada 0.0, got {}",
            lat
        );
        assert!(
            (lng - 0.0).abs() < f32::EPSILON,
            "lng esperada 0.0, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_con_ampersand_3d() {
        let (lat, lng) = extract_coords_from_href(
            "https://www.google.com/maps/place/Negocio/@40.4168,-3.7038,17z?&3d40.4168&4d-3.7038",
        );
        assert!(
            (lat - 40.4168).abs() < 0.001,
            "lat esperada 40.4168, got {}",
            lat
        );
        assert!(
            (lng - -3.7038).abs() < 0.001,
            "lng esperada -3.7038, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_href_relativo() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Tintorer%C3%ADa+Limpieza+Express/@40.4168,-3.7038,17z/data=!3m1!4b1",
        );
        assert!(
            (lat - 40.4168).abs() < 0.001,
            "lat esperada 40.4168, got {}",
            lat
        );
        assert!(
            (lng - -3.7038).abs() < 0.001,
            "lng esperada -3.7038, got {}",
            lng
        );
    }

    #[test]
    fn test_extract_coords_canarias() {
        let (lat, lng) = extract_coords_from_href(
            "/maps/place/Tinte+Canarias/@28.1234,-15.4567,14z/data=!3m1!4b1",
        );
        assert!(
            (lat - 28.1234).abs() < 0.001,
            "lat esperada 28.1234, got {}",
            lat
        );
        assert!(
            (lng - -15.4567).abs() < 0.001,
            "lng esperada -15.4567, got {}",
            lng
        );
    }
}
