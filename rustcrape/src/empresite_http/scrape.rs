use std::time::Duration;

use anyhow::Result;
use scraper::{Html, Selector};

use crate::context::Context;
use crate::empresite::config::{
    activity_action, activity_slug, province_path, ActivityAction, EmpresiteParams,
};
use crate::scraper::ScrapeResult;
use crate::storage::Persistence;
use crate::types::{Coincidence, Config, EmpresiteConfig};
use crate::utils::random_delay;
use crate::utils::*;

/// Maximo de reintentos por listado/ficha antes de darla por inaccesible.
const MAX_LISTING_ATTEMPTS: usize = 3;
const MAX_DETAIL_ATTEMPTS: usize = 3;

/// Tiempo maximo por peticion HTTP. El cliente compartido no lleva timeout
/// global, asi que cada peticion lo fija aqui para no quedarse colgada.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

const BASE_URL: &str = "https://empresite.eleconomista.es";

// --- Cliente HTTP -----------------------------------------------------------

/// Respuesta HTTP de una peticion: cuerpo y URL final tras redirecciones.
struct Fetched {
    html: String,
    final_url: String,
}

/// Obtiene el HTML de un listado. El listado filtrado se sirve mediante un POST
/// (params en la query, cuerpo vacío); sin filtros basta un GET.
async fn fetch_listing(ctx: &Context, url: &str, config: &EmpresiteConfig) -> Result<Fetched> {
    if config.filter_query().is_empty() {
        ctx.verboser()
            .debug(&format!("Empresite HTTP listing: GET (no filters) {url}"));
        get(ctx, url).await
    } else {
        ctx.verboser()
            .debug(&format!("Empresite HTTP listing: POST (filters) {url}"));
        post(ctx, url).await
    }
}

async fn fetch_detail(ctx: &Context, url: &str) -> Result<Fetched> {
    ctx.verboser()
        .debug(&format!("Empresite HTTP detail: fetching {url}"));
    get(ctx, url).await
}

async fn get(ctx: &Context, url: &str) -> Result<Fetched> {
    ctx.verboser().debug(&format!("HTTP GET {url}"));
    let resp = ctx.http().get(url).timeout(HTTP_TIMEOUT).send().await?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    let html = resp.text().await?;
    ctx.verboser().debug(&format!(
        "HTTP GET {url} -> {status}, {} bytes (final {final_url})",
        html.len()
    ));
    Ok(Fetched { html, final_url })
}

async fn post(ctx: &Context, url: &str) -> Result<Fetched> {
    ctx.verboser().debug(&format!("HTTP POST {url}"));
    let resp = ctx
        .http()
        .post(url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded;charset=UTF-8",
        )
        .timeout(HTTP_TIMEOUT)
        .send()
        .await?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    let html = resp.text().await?;
    ctx.verboser().debug(&format!(
        "HTTP POST {url} -> {status}, {} bytes (final {final_url})",
        html.len()
    ));
    Ok(Fetched { html, final_url })
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
    legal_name: String,
    tax_id: String,
    legal_form: String,
    sector: String,
    incorporation_date: String,
    last_change_date: String,
    corporate_purpose: String,
    activity: String,
    cnae_activity: String,
    company_status: String,
}

/// Devuelve el texto del `<span>` hermano siguiente al `h3` cuyo texto coincide
/// exactamente con `label`. Devuelve vacio si la etiqueta no existe.
fn field_by_label(doc: &scraper::Html, label: &str) -> String {
    for h3 in doc.select(&sel("h3")) {
        if text_of(h3) == label {
            let mut sib = h3.next_sibling();
            while let Some(s) = sib {
                if let Some(el) = scraper::ElementRef::wrap(s) {
                    if el.value().name() == "span" {
                        return text_of(el);
                    }
                }
                sib = s.next_sibling();
            }
        }
    }
    String::new()
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
        legal_name: field_by_label(&doc, "Razón social"),
        tax_id: field_by_label(&doc, "CIF"),
        legal_form: field_by_label(&doc, "Forma jurídica"),
        sector: field_by_label(&doc, "Sector"),
        incorporation_date: field_by_label(&doc, "Fecha de constitución"),
        last_change_date: field_by_label(&doc, "Fecha último cambio"),
        corporate_purpose: field_by_label(&doc, "Objeto social"),
        activity: field_by_label(&doc, "Actividad"),
        cnae_activity: field_by_label(&doc, "Actividad CNAE"),
        company_status: field_by_label(&doc, "Estado de la empresa"),
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

fn clean_text(raw: &str) -> Option<String> {
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

/// Genera la URL del listado para una página concreta anexando los filtros.
fn listing_url(activity: &str, page: u32, cfg: &EmpresiteConfig) -> String {
    let province = province_path(cfg.province);
    let base = if page <= 1 {
        format!("{BASE_URL}/Actividad/{activity}/{province}")
    } else {
        format!("{BASE_URL}/Actividad/{activity}/{province}PgNum-{page}/")
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
        format!("https://{stripped}")
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

async fn unblock<F: Future<Output = Result<Fetched>>>(
    ctx: &Context,
    then: impl Fn() -> F,
) -> Result<Fetched> {
    ctx.verboser()
        .debug("Empresite HTTP unblock: rotating VPN to get a new IP");
    if !ctx.vpn_rotate().await? {
        ctx.verboser()
            .debug("Empresite HTTP unblock: VPN unavailable, cannot rotate");
        return Err(CaptchaError.into());
    }
    ctx.verboser()
        .debug("Empresite HTTP unblock: VPN rotated, retrying request until unblocked");
    Ok(wait_until(
        || async { Ok(Some(then().await?)) },
        Duration::from_millis(500),
        Duration::from_secs(10),
        ctx.cancellation(),
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
    verboser.debug(&format!(
        "Empresite HTTP detail #{count}: {url} (max {MAX_DETAIL_ATTEMPTS} attempts)"
    ));

    for attempt in 1..=MAX_DETAIL_ATTEMPTS {
        verboser.debug(&format!(
            "Empresite HTTP detail #{count}: attempt {attempt}/{MAX_DETAIL_ATTEMPTS}"
        ));
        let mut fetched = fetch_detail(ctx, &url).await?;

        if is_blocked(&fetched.html) {
            verboser.warn(&format!(
                "Ficha {} bloqueada (429); rotando IP (intento {attempt})",
                link
            ));
            fetched = unblock(ctx, || fetch_detail(ctx, &url)).await?;
            verboser.debug(&format!(
                "Empresite HTTP detail #{count}: unblocked on attempt {attempt}"
            ));
        }

        if !is_detail_loaded(&fetched.html) {
            verboser.debug(&format!(
                "Empresite HTTP detail #{count}: page not a valid detail on attempt {attempt}"
            ));
            verboser.warn(&format!(
                "Ficha {} no cargó correctamente (intento {attempt}); reintentando",
                link
            ));
            sleep_cancellable(
                random_delay(config.empresite.delay_min, config.empresite.delay_max),
                ctx.cancellation(),
            )
            .await?;
            continue;        }

        let raw = extract_detail(&fetched.html);
        if raw.name.is_empty() {
            verboser.debug(&format!(
                "Empresite HTTP detail #{count}: empty name, skipping record"
            ));
            return Ok(DetailOutcome::Skipped);
        }

        let email = clean_email(&raw.email);
        let web = clean_web(&raw.web);
        let tfno = clean_tfno(&raw.tfno);
        let legal_name = clean_text(&raw.legal_name);
        let tax_id = clean_text(&raw.tax_id);
        let legal_form = clean_text(&raw.legal_form);
        let sector = clean_text(&raw.sector);
        let incorporation_date = clean_text(&raw.incorporation_date);
        let last_change_date = clean_text(&raw.last_change_date);
        let corporate_purpose = clean_text(&raw.corporate_purpose);
        let activity = clean_text(&raw.activity);
        let cnae_activity = clean_text(&raw.cnae_activity);
        let company_status = clean_text(&raw.company_status);

        verboser.debug(&format!(
            "Empresite HTTP detail #{count}: extracted '{}' (email={}, website={}, phone={})",
            raw.name,
            email.is_some(),
            web.is_some(),
            tfno.is_some(),
        ));
        verboser.processed_coincidence(&raw.name, count);

        return Ok(DetailOutcome::Found(Coincidence {
            name: raw.name,
            email,
            web,
            tfno,
            source_url: clean_empresite_url(link),
            legal_name,
            tax_id,
            legal_form,
            sector,
            incorporation_date,
            last_change_date,
            corporate_purpose,
            activity,
            cnae_activity,
            company_status,
        }));
    }

    verboser.debug(&format!(
        "Empresite HTTP detail #{count}: giving up after {MAX_DETAIL_ATTEMPTS} attempts"
    ));
    Ok(DetailOutcome::Skipped)
}

/// Scrapea una página del listado: obtiene el HTML, extrae los enlaces a las
/// fichas y scrapea cada una por HTTP.
async fn scrape_internal<P: Persistence>(
    ctx: &Context,
    params: &EmpresiteParams,
    config: &Config,
    persist: &P,
) -> Result<ScrapeResult> {
    let verboser = ctx.verboser();
    verboser.searching_coincidences();

    let requested = activity_slug(&config.empresite.search_query);
    // Empresite puede canonicalizar el nombre de la actividad. Si ya lo
    // descubrimos antes para este mismo termino, usamos el canonico.
    let mut activity = match persist.empresite_activity().await? {
        Some((source, canonical)) if source == requested => {
            verboser.debug(&format!(
                "Empresite HTTP listing: using persisted canonical activity \
                 '{canonical}' (requested '{requested}')"
            ));
            canonical
        }
        _ => requested.clone(),
    };
    verboser.debug(&format!(
        "Empresite HTTP listing: activity slug='{activity}', page={}",
        params.page
    ));

    // Carga el listado, rotando IP mientras esté bloqueado.
    let mut listing: Option<(Vec<String>, bool)> = None;
    for attempt in 1..=MAX_LISTING_ATTEMPTS {
        let url = listing_url(&activity, params.page, &config.empresite);
        verboser.debug(&format!(
            "Empresite HTTP listing: attempt {attempt}/{MAX_LISTING_ATTEMPTS} -> {url}"
        ));
        let mut fetched = match fetch_listing(ctx, &url, &config.empresite).await {
            Ok(fetched) => fetched,
            Err(_) => {
                verboser.debug(&format!(
                    "Empresite HTTP listing: attempt {attempt} failed to fetch, retrying"
                ));
                continue;
            }
        };

        // El bloqueo (429) no redirige, asi que hay que desbloquear antes de
        // leer la URL final: es en la respuesta real donde aparece el activity
        // canonico que decidio Empresite.
        if is_blocked(&fetched.html) {
            verboser.warn("Listado bloqueado (429); rotando IP y reintentando");
            fetched = unblock(ctx, || fetch_listing(ctx, &url, &config.empresite)).await?;
            verboser.debug("Empresite HTTP listing: listing unblocked after VPN rotation");
            if is_blocked(&fetched.html) {
                verboser.warn("Listado aun bloqueado tras rotar IP; reintentando");
                continue;
            }
        }

        // Si Empresite renombra la actividad, la redireccion pierde el PgNum y
        // devuelve siempre la pagina 1. Fijamos el nombre canonico y, si no
        // estabamos en la primera pagina, recargamos la pagina correcta.
        match activity_action(
            &activity,
            &fetched.final_url,
            params.page,
            config.empresite.province.is_some(),
        ) {
            ActivityAction::Keep => {}
            ActivityAction::Renamed { canonical, reload } => {
                verboser.warn(&format!(
                    "Empresite cambio el activity '{activity}' -> '{canonical}'; \
                     fijandolo para las siguientes paginas"
                ));
                if let Err(err) = persist.set_empresite_activity(&requested, &canonical).await {
                    verboser.error(&format!(
                        "No se pudo persistir el activity canonico '{canonical}': {err}"
                    ));
                }
                activity = canonical;

                if reload {
                    verboser.debug(&format!(
                        "Empresite HTTP listing: reloading page {} with canonical \
                         activity '{activity}'",
                        params.page
                    ));
                    continue;
                }
            }
        }

        let html = fetched.html;
        let links = extract_company_links(&html);
        let has_more = has_next_page(&html, params.page);
        verboser.debug(&format!(
            "Loaded list: {} links, next page: {has_more}",
            links.len(),
        ));
        listing = Some((links, has_more));
        break;
    }

    let Some((links, has_more)) = listing else {
        verboser.debug(&format!(
            "Empresite HTTP listing: giving up after {MAX_LISTING_ATTEMPTS} attempts"
        ));
        return Err(anyhow::anyhow!(
            "no se pudo cargar el listado tras {} intentos",
            MAX_LISTING_ATTEMPTS
        ));
    };

    verboser.found_coincidences(Some(links.len()));

    let mut coincidences = Vec::new();
    let total_links = links.len();
    for (idx, link) in links.into_iter().enumerate() {
        if verboser.is_cancelled() {
            break;
        }

        verboser.debug(&format!(
            "Empresite HTTP listing: link {}/{} -> {link}",
            idx + 1,
            total_links
        ));

        let delay = random_delay(config.empresite.delay_min, config.empresite.delay_max);
        if verboser.debug_enabled() {
            verboser.debug(&format!("Empresite HTTP listing: waiting {delay:?} before detail"));
        }
        if sleep_cancellable(delay, ctx.cancellation()).await.is_err() {
            break;
        }

        match scrape_detail(ctx, &link, config, idx + 1).await? {
            DetailOutcome::Found(coincidence) => coincidences.push(coincidence),
            DetailOutcome::Skipped => {}
        }
    }

    verboser.debug(&format!(
        "Empresite HTTP listing: page {} finished with {} results (has_more={has_more})",
        params.page,
        coincidences.len()
    ));
    Ok(ScrapeResult::new(coincidences, has_more))
}

pub async fn scrape<P: Persistence>(
    params: &EmpresiteParams,
    config: &Config,
    ctx: &Context,
    persist: &P,
) -> Result<ScrapeResult> {
    ctx.verboser().debug(&format!(
        "Empresite HTTP: starting scrape of page {} (query='{}')",
        params.page, config.empresite.search_query
    ));
    scrape_internal(ctx, params, config, persist).await
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
    fn test_listing_url_con_provincia() {
        use crate::types::Province;

        let cfg = EmpresiteConfig {
            province: Some(Province::Madrid),
            ..Default::default()
        };
        assert_eq!(
            listing_url("BARCOS-DE-VELA", 1, &cfg),
            "https://empresite.eleconomista.es/Actividad/BARCOS-DE-VELA/provincia/MADRID/"
        );
        assert_eq!(
            listing_url("BARCOS-DE-VELA", 2, &cfg),
            "https://empresite.eleconomista.es/Actividad/BARCOS-DE-VELA/provincia/MADRID/PgNum-2/"
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
    fn test_extract_detail_datos_mercantiles() {
        let html = r#"
            <html><body>
              <h1>Barcos Y Amarres Sl</h1>
              <a class="email" href="mailto:info@test.com">email</a>
              <div class="flex flex-col gap-2"><h3>Razón social</h3><span>Barcos Y Amarres Sl</span></div>
              <div class="flex flex-col gap-2"><h3>CIF</h3><span>B97564058</span></div>
              <div class="flex flex-col gap-2"><h3>Forma jurídica</h3><span>Sociedad limitada unipersonal</span></div>
              <div class="flex flex-col gap-2"><h3>Sector</h3><span>Industria</span></div>
              <div class="flex flex-col gap-2"><h3>Fecha de constitución</h3><span>23-6-2005</span></div>
              <div class="flex flex-col gap-2"><h3>Fecha último cambio</h3><span>23-8-2026</span></div>
              <div class="flex flex-col gap-2"><h3>Objeto social</h3><span>Explotacion y mantenimiento</span></div>
              <div class="flex flex-col gap-2"><h3>Actividad</h3><span>Reparación, mantenimiento de buques</span></div>
              <div class="flex flex-col gap-2"><h3>Actividad CNAE</h3><span>3315 - Reparación y mantenimiento</span></div>
              <div class="flex flex-col gap-2"><h3>Estado de la empresa</h3><span>Viva</span></div>
            </body></html>
        "#;
        let d = extract_detail(html);
        assert_eq!(d.name, "Barcos Y Amarres Sl");
        assert_eq!(d.legal_name, "Barcos Y Amarres Sl");
        assert_eq!(d.tax_id, "B97564058");
        assert_eq!(d.legal_form, "Sociedad limitada unipersonal");
        assert_eq!(d.sector, "Industria");
        assert_eq!(d.incorporation_date, "23-6-2005");
        assert_eq!(d.last_change_date, "23-8-2026");
        assert_eq!(d.corporate_purpose, "Explotacion y mantenimiento");
        assert_eq!(d.activity, "Reparación, mantenimiento de buques");
        assert_eq!(d.cnae_activity, "3315 - Reparación y mantenimiento");
        assert_eq!(d.company_status, "Viva");
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
