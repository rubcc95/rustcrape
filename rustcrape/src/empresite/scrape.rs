use anyhow::Result;
use serde::Deserialize;

use crate::empresite::config::EmpresiteParams;
use crate::scraper::ScrapeResult;
use crate::types::{Coincidence, EmpresiteConfig};
use crate::verboser::Verboser;
use chromiumoxide::{Browser, Page};
use std::time::Duration;

/// Retardo pseudo-aleatorio entre peticiones, dentro del rango configurado.
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

/// Espera hasta que `f` devuelva `Some`, sondeando cada `interval` hasta `timeout`.
async fn wait_until<F, Fut, T>(mut f: F, interval: Duration, timeout: Duration) -> Result<T>
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

/// Acepta el banner de consentimiento de Didomi si aparece.
async fn accept_cookies(page: &Page) -> Result<()> {
    let buttons = page.find_elements("button").await?;
    for btn in buttons {
        let text = btn
            .string_property("textContent")
            .await?
            .unwrap_or_default();
        let lower = text.trim().to_lowercase();
        if lower.contains("aceptar") || lower.contains("accept") {
            let _ = btn.click().await;
            return Ok(());
        }
    }
    Ok(())
}

/// Extrae los enlaces a fichas de empresa del listado actual.
///
/// Se identifican como enlaces raiz que terminan en `.html` y que no apuntan a
/// paginas internas (`/Actividad/`, `/empresas-provincia`, legales, etc.).
async fn extract_company_links(page: &Page) -> Result<Vec<String>> {
    let js = r#"(() => {
        const out = [];
        for (const a of document.querySelectorAll('a[href*=".html"]')) {
            const h = a.getAttribute('href');
            if (!h) continue;
            if (h.includes('/Actividad/')) continue;
            if (h.includes('/empresas-provincia')) continue;
            if (h.includes('/informes-empresas')) continue;
            if (h.includes('FAQS')) continue;
            if (h.includes('Terms')) continue;
            if (h.includes('Privacy')) continue;
            if (h.includes('politica')) continue;
            if (!out.includes(h)) out.push(h);
        }
        return out;
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Comprueba si existe un enlace de paginacion a la pagina siguiente.
async fn has_next_page(page: &Page, current: u32) -> Result<bool> {
    let needle = format!("PgNum-{}/", current + 1);
    let js = format!(
        r#"(() => {{
            const needle = "{}";
            for (const a of document.querySelectorAll('a[href]')) {{
                if ((a.getAttribute('href') || '').includes(needle)) return true;
            }}
            return false;
        }})()"#,
        needle
    );
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Resultado crudo de una ficha de empresa.
#[derive(Debug, Default, Deserialize)]
struct DetailRaw {
    #[serde(default)]
    name: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    web: String,
    #[serde(default)]
    tfno: String,
}

/// Extrae los datos crudos de una ficha via evaluacion JS.
async fn extract_detail(page: &Page) -> Result<DetailRaw> {
    let js = r#"(() => {
        const name = (document.querySelector('h1')?.textContent || '').trim();
        const emailEl = document.querySelector('a.email[href^="mailto:"]');
        const webEl = document.querySelector('a.url[href]');
        const telEl = document.querySelector('span.tel span.value');
        return {
            name: name,
            email: emailEl ? emailEl.getAttribute('href') : '',
            web: webEl ? webEl.getAttribute('href') : '',
            tfno: telEl ? telEl.textContent.trim() : ''
        };
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Limpia el email: quita `mailto:` y cualquier query (`?subject=...`).
fn clean_email(raw: &str) -> Option<String> {
    let s = raw.strip_prefix("mailto:").unwrap_or(raw);
    let s = s.split('?').next().unwrap_or(s).trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Limpia la web: quita `//` inicial, query y barra final.
fn clean_web(raw: &str) -> Option<String> {
    let s = raw.trim().trim_start_matches("//");
    let s = s.split('?').next().unwrap_or(s);
    let s = s.strip_suffix('/').unwrap_or(s);
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Limpia el telefono: recorta espacios.
fn clean_tfno(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Normaliza la URL de origen de Empresite.
fn clean_empresite_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    base.trim().strip_suffix('/').unwrap_or(base.trim()).to_string()
}

/// Scrapea una ficha de empresa abriendo una pestaña nueva.
async fn scrape_detail(
    browser: &Browser,
    link: &str,
    config: &EmpresiteConfig,
    verboser: &dyn Verboser,
    count: usize,
) -> Result<Option<Coincidence>> {
    let detail_page = browser.new_page(link).await?;
    let _ = detail_page.wait_for_navigation().await;

    // Espera a que el h1 de la ficha este disponible.
    let _ = wait_until(
        || async {
            let name: String = detail_page
                .evaluate("document.querySelector('h1')?.textContent?.trim() ?? ''")
                .await?
                .into_value()?;
            Ok((!name.is_empty()).then_some(()))
        },
        Duration::from_millis(200),
        Duration::from_secs(15),
    )
    .await;

    let raw = extract_detail(&detail_page).await?;
    let _ = detail_page.close().await;

    if raw.name.is_empty() {
        return Ok(None);
    }

    let email = clean_email(&raw.email);
    let web = clean_web(&raw.web);
    let tfno = clean_tfno(&raw.tfno);

    verboser.processed_coincidence(&raw.name, count);

    Ok(Some(Coincidence {
        name: raw.name,
        email,
        web,
        tfno,
        source_url: clean_empresite_url(link),
    }))
}

/// Scrapea una pagina del listado de Empresite: extrae los enlaces a las fichas
/// y, para cada una, abre su detalle y registra los datos de contacto.
pub async fn scrape(
    browser: &Browser,
    params: &EmpresiteParams,
    config: &EmpresiteConfig,
    verboser: &dyn Verboser,
) -> Result<ScrapeResult> {
    verboser.searching_coincidences();

    let activity = config.search_query.trim().to_uppercase();
    let url = if params.page <= 1 {
        format!("https://empresite.eleconomista.es/Actividad/{activity}/")
    } else {
        format!(
            "https://empresite.eleconomista.es/Actividad/{activity}/PgNum-{}/",
            params.page
        )
    };

    let page = browser.new_page(&url).await?;
    let _ = page.wait_for_navigation().await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    verboser.accepting_cookies();
    let _ = accept_cookies(&page).await;

    if verboser.is_cancelled() {
        return Ok(ScrapeResult::empty(false));
    }

    let links = extract_company_links(&page).await?;
    verboser.found_multiple_coincidences();

    let has_more = has_next_page(&page, params.page).await?;

    let mut coincidences = Vec::new();
    for (idx, link) in links.into_iter().enumerate() {
        if verboser.is_cancelled() {
            break;
        }

        tokio::time::sleep(random_delay(config.delay_min, config.delay_max)).await;

        match scrape_detail(browser, &link, config, verboser, idx + 1).await {
            Ok(Some(coincidence)) => coincidences.push(coincidence),
            Ok(None) => {}
            Err(err) => verboser.warn(&format!("Error extrayendo ficha {}: {}", link, err)),
        }
    }

    Ok(ScrapeResult::new(coincidences, has_more))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_email_con_query() {
        assert_eq!(
            clean_email("mailto:info@empresa.com?subject=Contacto"),
            Some("info@empresa.com".to_string())
        );
    }

    #[test]
    fn test_clean_email_sin_query() {
        assert_eq!(
            clean_email("mailto:info@empresa.com"),
            Some("info@empresa.com".to_string())
        );
    }

    #[test]
    fn test_clean_email_vacio() {
        assert_eq!(clean_email(""), None);
    }

    #[test]
    fn test_clean_web_protocol_relative() {
        assert_eq!(
            clean_web("//www.empresa.com"),
            Some("www.empresa.com".to_string())
        );
    }

    #[test]
    fn test_clean_web_con_barra_final() {
        assert_eq!(
            clean_web("//www.empresa.com/"),
            Some("www.empresa.com".to_string())
        );
    }

    #[test]
    fn test_clean_web_vacio() {
        assert_eq!(clean_web(""), None);
    }

    #[test]
    fn test_clean_tfno() {
        assert_eq!(clean_tfno(" 945290500 "), Some("945290500".to_string()));
    }

    #[test]
    fn test_clean_empresite_url_con_query() {
        assert_eq!(
            clean_empresite_url("https://empresite.eleconomista.es/BICICLETAS-MENDIZ.html?x=1"),
            "https://empresite.eleconomista.es/BICICLETAS-MENDIZ.html".to_string()
        );
    }

    #[test]
    fn test_clean_empresite_url_simple() {
        assert_eq!(
            clean_empresite_url("https://empresite.eleconomista.es/BICICLETAS-MENDIZ.html"),
            "https://empresite.eleconomista.es/BICICLETAS-MENDIZ.html".to_string()
        );
    }
}
