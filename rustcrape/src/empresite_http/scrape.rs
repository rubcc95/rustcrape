use std::time::Duration;

use anyhow::Result;
use scraper::{Html, Selector};

use crate::context::Context;
use crate::empresite::config::EmpresiteParams;
use crate::scraper::ScrapeResult;
use crate::types::{Coincidence, Config, EmpresiteConfig};
use crate::utils::random_delay;
use crate::utils::*;

/// Maximo de reintentos por listado/ficha antes de darla por inaccesible.
const MAX_LISTING_ATTEMPTS: usize = 3;
const MAX_DETAIL_ATTEMPTS: usize = 3;

const BASE_URL: &str = "https://empresite.eleconomista.es";

// --- Cliente HTTP -----------------------------------------------------------

/// Obtiene el HTML de un listado. El listado filtrado se sirve mediante un POST
/// (params en la query, cuerpo vacío); sin filtros basta un GET.
async fn fetch_listing(ctx: &Context, url: &str, config: &EmpresiteConfig) -> Result<String> {
    if config.filter_query().is_empty() {
        get(ctx, url).await
    } else {
        post(ctx, url).await
    }
}

async fn fetch_detail(ctx: &Context, url: &str) -> Result<String> {
    get(ctx, url).await
}

async fn get(ctx: &Context, url: &str) -> Result<String> {
    let resp = ctx.http().get(url).send().await?;
    Ok(resp.text().await?)
}

async fn post(ctx: &Context, url: &str) -> Result<String> {
    let resp = ctx
        .http()
        .post(url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded;charset=UTF-8",
        )
        .send()
        .await?;
    Ok(resp.text().await?)
}

// --- Detección de bloqueo ---------------------------------------------------

/// Indica si el HTML recibido corresponde a la página de bloqueo (429 + reCAPTCHA).
fn is_blocked(html: &str) -> bool {
    html.contains("Demasiadas peticiones") || html.contains("recaptcha/api2/anchor")
}

// --- Parseo del listado -----------------------------------------------------

fn sel(selector: &str) -> Selector {
    Selector::parse(selector).expect("selector CSS válido")
}

fn text_of(e: scraper::ElementRef<'_>) -> String {
    e.text().collect::<String>().trim().to_string()
}

/// Extrae los enlaces a fichas de empresa del listado actual.
fn extract_company_links(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let a_sel = sel("a[href*=\".html\"]");

    let mut out: Vec<String> = Vec::new();
    for a in doc.select(&a_sel) {
        let Some(h) = a.value().attr("href") else {
            continue;
        };
        if h.contains("/Actividad/")
            || h.contains("/empresas-provincia")
            || h.contains("/informes-empresas")
            || h.contains("FAQS")
            || h.contains("Terms")
            || h.contains("Privacy")
            || h.contains("politica")
        {
            continue;
        }
        if !out.contains(&h.to_string()) {
            out.push(h.to_string());
        }
    }
    out
}

/// Comprueba si existe un enlace de paginación a la página siguiente.
fn has_next_page(html: &str, current: u32) -> bool {
    let needle = format!("PgNum-{}/", current + 1);
    let doc = Html::parse_document(html);
    let a_sel = sel("a");
    for a in doc.select(&a_sel) {
        let hay = format!(
            "{} {}",
            a.value().attr("href").unwrap_or(""),
            a.value().attr("onclick").unwrap_or("")
        );
        if hay.contains(&needle) {
            return true;
        }
    }
    false
}

// --- Parseo de la ficha -----------------------------------------------------

#[derive(Debug, Default)]
struct DetailRaw {
    name: String,
    email: String,
    web: String,
    tfno: String,
}

fn extract_detail(html: &str) -> DetailRaw {
    let doc = Html::parse_document(html);

    let name = doc
        .select(&sel("h1"))
        .next()
        .map(text_of)
        .unwrap_or_default();

    let email = doc
        .select(&sel("a.email[href^=\"mailto:\"]"))
        .next()
        .and_then(|e| e.value().attr("href").map(str::to_string))
        .unwrap_or_default();

    let web = doc
        .select(&sel("a.url[href]"))
        .next()
        .and_then(|e| e.value().attr("href").map(str::to_string))
        .unwrap_or_default();

    let tfno = doc
        .select(&sel("span.tel span.value"))
        .next()
        .map(text_of)
        .unwrap_or_default();

    DetailRaw {
        name,
        email,
        web,
        tfno,
    }
}

/// Comprueba que la página sea un detalle real de Empresite con sus datos:
/// nombre presente y, además, sección de ficha o dato de contacto. Evita
/// confundir una página de error con una ficha.
fn is_detail_loaded(html: &str) -> bool {
    let doc = Html::parse_document(html);

    let name = doc
        .select(&sel("h1"))
        .next()
        .map(text_of)
        .unwrap_or_default();
    if name.is_empty() {
        return false;
    }

    let has_section = ["#infogeneral", "#dircont", "#datoscomerciales", "#rankings"]
        .iter()
        .any(|s| doc.select(&sel(s)).next().is_some());

    let has_contact = [
        "a.email[href^=\"mailto:\"]",
        "span.tel span.value",
        "a.url[href]",
    ]
    .iter()
    .any(|s| doc.select(&sel(s)).next().is_some());

    has_section || has_contact
}

// --- Limpieza de campos -----------------------------------------------------

fn clean_email(raw: &str) -> Option<String> {
    let s = raw.strip_prefix("mailto:").unwrap_or(raw);
    let s = s.split('?').next().unwrap_or(s).trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

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

fn clean_tfno(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn clean_empresite_url(url: &str) -> String {
    let base = url.split('?').next().unwrap_or(url);
    base.trim()
        .strip_suffix('/')
        .unwrap_or(base.trim())
        .to_string()
}

/// Convierte el texto de la actividad en un slug apto para la URL de Empresite.
fn activity_slug(query: &str) -> String {
    let mut slug = String::with_capacity(query.len());
    for ch in query.trim().to_uppercase().chars() {
        let ch = match ch {
            'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'Å' => 'A',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' => 'O',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'Ç' => 'C',
            'Ñ' => 'N',
            other => other,
        };
        if ch.is_ascii_alphanumeric() || ch == '_' {
            slug.push(ch);
        } else if ch.is_whitespace() || ch == '-' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    slug.trim_matches('-').to_string()
}

/// Genera la URL del listado para una página concreta anexando los filtros.
fn listing_url(activity: &str, page: u32, cfg: &EmpresiteConfig) -> String {
    let base = if page <= 1 {
        format!("{BASE_URL}/Actividad/{activity}/")
    } else {
        format!("{BASE_URL}/Actividad/{activity}/PgNum-{page}/")
    };

    let filters = cfg.filter_query();
    if filters.is_empty() {
        base
    } else {
        format!("{base}?testfiltros=1&{filters}")
    }
}

/// Convierte un `href` (absoluto, relativo o protocol-relative) en URL absoluta.
fn absolutize(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(stripped) = href.strip_prefix("//") {
        format!("https:{stripped}")
    } else if let Some(stripped) = href.strip_prefix('/') {
        format!("{BASE_URL}/{stripped}")
    } else {
        format!("{BASE_URL}/{href}")
    }
}

// --- Orquestación -----------------------------------------------------------

enum DetailOutcome {
    Found(Coincidence),
    Skipped,
}

#[derive(Debug, thiserror::Error)]
#[error("Captcha locked the page. VPN unable to rotate.")]
struct CaptchaError;

async fn unblock<F: Future<Output = Result<String>>>(
    ctx: &Context,
    then: impl Fn() -> F,
) -> Result<String> {
    if !ctx.vpn_rotate().await? {
        return Err(CaptchaError.into());
    }
    Ok(wait_until(
        || async { Ok(Some(then().await?)) },
        Duration::from_millis(500),
        Duration::from_secs(10),
    )
    .await?)
}

/// Scrapea una ficha de empresa por HTTP: reintenta (rotando IP) si está
/// bloqueada o no carga, hasta `MAX_DETAIL_ATTEMPTS`.
async fn scrape_detail(
    ctx: &Context,
    link: &str,
    config: &Config,
    count: usize,
) -> Result<DetailOutcome> {
    let verboser = ctx.verboser();
    let url = absolutize(link);

    for attempt in 1..=MAX_DETAIL_ATTEMPTS {
        let mut html = fetch_detail(ctx, &url).await?;

        if is_blocked(&html) {
            verboser.warn(&format!(
                "Ficha {} bloqueada (429); rotando IP (intento {attempt})",
                link
            ));
            html = unblock(ctx, || fetch_detail(ctx, &url)).await?;
        }

        if !is_detail_loaded(&html) {
            verboser.warn(&format!(
                "Ficha {} no cargó correctamente (intento {attempt}); reintentando",
                link
            ));
            tokio::time::sleep(random_delay(
                config.empresite.delay_min,
                config.empresite.delay_max,
            ))
            .await;
            continue;
        }

        let raw = extract_detail(&html);
        if raw.name.is_empty() {
            return Ok(DetailOutcome::Skipped);
        }

        let email = clean_email(&raw.email);
        let web = clean_web(&raw.web);
        let tfno = clean_tfno(&raw.tfno);

        verboser.processed_coincidence(&raw.name, count);

        return Ok(DetailOutcome::Found(Coincidence {
            name: raw.name,
            email,
            web,
            tfno,
            source_url: clean_empresite_url(link),
        }));
    }

    Ok(DetailOutcome::Skipped)
}

/// Scrapea una página del listado: obtiene el HTML, extrae los enlaces a las
/// fichas y scrapea cada una por HTTP.
async fn scrape_internal(
    ctx: &Context,
    params: &EmpresiteParams,
    config: &Config,
) -> Result<ScrapeResult> {
    let verboser = ctx.verboser();
    verboser.searching_coincidences();

    let activity = activity_slug(&config.empresite.search_query);

    // Carga el listado, rotando IP mientras esté bloqueado.
    let mut listing: Option<(Vec<String>, bool)> = None;
    for _ in 0..MAX_LISTING_ATTEMPTS {
        let url = listing_url(&activity, params.page, &config.empresite);
        let Ok(mut html) = fetch_listing(ctx, &url, &config.empresite).await else {
            continue;
        };

        if is_blocked(&html) {
            verboser.warn("Listado bloqueado (429); rotando IP y reintentando");
            
            html = unblock(ctx, || fetch_listing(ctx, &url, &config.empresite)).await?;
        }

        let links = extract_company_links(&html);
        let has_more = has_next_page(&html, params.page);
        listing = Some((links, has_more));
        break;
    }

    let Some((links, has_more)) = listing else {
        return Err(anyhow::anyhow!(
            "no se pudo cargar el listado tras {} intentos",
            MAX_LISTING_ATTEMPTS
        ));
    };

    verboser.found_multiple_coincidences();

    let mut coincidences = Vec::new();
    for (idx, link) in links.into_iter().enumerate() {
        if verboser.is_cancelled() {
            break;
        }

        tokio::time::sleep(random_delay(
            config.empresite.delay_min,
            config.empresite.delay_max,
        ))
        .await;

        match scrape_detail(ctx, &link, config, idx + 1).await? {
            DetailOutcome::Found(coincidence) => coincidences.push(coincidence),
            DetailOutcome::Skipped => {}
        }
    }

    Ok(ScrapeResult::new(coincidences, has_more))
}

pub async fn scrape(
    params: &EmpresiteParams,
    config: &Config,
    ctx: &Context,
) -> Result<ScrapeResult> {
    scrape_internal(ctx, params, config).await
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

    #[test]
    fn test_activity_slug_acentos_y_espacios() {
        assert_eq!(
            activity_slug("  fontanería y calefacción "),
            "FONTANERIA-Y-CALEFACCION"
        );
    }

    #[test]
    fn test_activity_slug_elimina_especiales() {
        assert_eq!(activity_slug("café/bar (centro)"), "CAFEBAR-CENTRO");
    }

    #[test]
    fn test_activity_slug_mantiene_guion_y_bajo() {
        assert_eq!(activity_slug("auto_escuela-test"), "AUTO_ESCUELA-TEST");
    }

    #[test]
    fn test_listing_url_sin_filtros() {
        let cfg = EmpresiteConfig::default();
        assert_eq!(
            listing_url("BARCOS-DE-VELA", 1, &cfg),
            "https://empresite.eleconomista.es/Actividad/BARCOS-DE-VELA/"
        );
        assert_eq!(
            listing_url("BARCOS-DE-VELA", 3, &cfg),
            "https://empresite.eleconomista.es/Actividad/BARCOS-DE-VELA/PgNum-3/"
        );
    }

    #[test]
    fn test_is_blocked_detecta_429() {
        assert!(is_blocked("<html>Demasiadas peticiones detectadas</html>"));
        assert!(is_blocked(
            "<iframe src=\"https://www.google.com/recaptcha/api2/anchor\"></iframe>"
        ));
        assert!(!is_blocked("<html><h1>Ficha de empresa</h1></html>"));
    }

    #[test]
    fn test_extract_detail() {
        let html = r#"
            <html><body>
              <h1>Empresa Test</h1>
              <a class="email" href="mailto:info@test.com">email</a>
              <a class="url" href="//www.test.com">web</a>
              <span class="tel"><span class="value">911234567</span></span>
            </body></html>
        "#;
        let d = extract_detail(html);
        assert_eq!(d.name, "Empresa Test");
        assert_eq!(d.email, "mailto:info@test.com");
        assert_eq!(d.web, "//www.test.com");
        assert_eq!(d.tfno, "911234567");
    }

    #[test]
    fn test_extract_company_links_filtra_internas() {
        let html = r#"
            <html><body>
              <a href="https://empresite.eleconomista.es/TINTORERIA-NINOT.html">A</a>
              <a href="/Actividad/TINTORERIA/">B</a>
              <a href="/empresas-provincia">C</a>
              <a href="/informes-empresas">D</a>
              <a href="https://empresite.eleconomista.es/TINTOALCA.html">E</a>
            </body></html>
        "#;
        let links = extract_company_links(html);
        assert_eq!(links.len(), 2);
        assert!(
            links.contains(&"https://empresite.eleconomista.es/TINTORERIA-NINOT.html".to_string())
        );
        assert!(links.contains(&"https://empresite.eleconomista.es/TINTOALCA.html".to_string()));
    }

    #[test]
    fn test_absolutize() {
        assert_eq!(
            absolutize("https://empresite.eleconomista.es/A.html"),
            "https://empresite.eleconomista.es/A.html"
        );
        assert_eq!(
            absolutize("//empresite.eleconomista.es/A.html"),
            "https://empresite.eleconomista.es/A.html"
        );
        assert_eq!(
            absolutize("/A.html"),
            "https://empresite.eleconomista.es/A.html"
        );
        assert_eq!(
            absolutize("A.html"),
            "https://empresite.eleconomista.es/A.html"
        );
    }
}
