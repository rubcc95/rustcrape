use anyhow::Result;
use serde::Deserialize;

use crate::browser::Browser;
use crate::context::Context;
use crate::empresite::config::EmpresiteParams;
use crate::scraper::ScrapeResult;
use crate::types::{Coincidence, Config, EmpresiteConfig};
use crate::utils::*;
use crate::verboser::Verboser;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::input::{DispatchKeyEventParams, DispatchKeyEventType};
use std::future::Future;
use std::time::Duration;

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
///
/// Sin filtros el numero de pagina viaja en el `href`; con filtros la
/// paginacion usa enlaces `javascript:void(0)` y la URL (con los filtros)
/// queda en el atributo `onclick`, asi que se revisan ambos.
async fn has_next_page(page: &Page, current: u32) -> Result<bool> {
    let needle = format!("PgNum-{}/", current + 1);
    let js = format!(
        r#"(() => {{
            const needle = "{}";
            for (const a of document.querySelectorAll('a')) {{
                const hay = (a.getAttribute('href') || '') + ' ' + (a.getAttribute('onclick') || '');
                if (hay.includes(needle)) return true;
            }}
            return false;
        }})()"#,
        needle
    );
    Ok(page.evaluate(js).await?.into_value()?)
}

// --- Captcha -----------------------------------------------------------------

/// Indica si la pagina actual es la de bloqueo (429 + reCAPTCHA).
async fn has_captcha(page: &Page) -> Result<bool> {
    let js = r#"(() => {
        if (document.querySelector("iframe[src*='recaptcha/api2/anchor']")) return true;
        const t = document.body ? document.body.innerText : '';
        return t.includes('Demasiadas peticiones');
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Envia una pulsacion de tecla (keyDown + keyUp) por CDP.
async fn press_key(page: &Page, key: &str, code: &str, vk: i64) -> Result<()> {
    let down = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyDown)
        .key(key)
        .code(code)
        .windows_virtual_key_code(vk)
        .native_virtual_key_code(vk)
        .build()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let up = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .key(key)
        .code(code)
        .windows_virtual_key_code(vk)
        .native_virtual_key_code(vk)
        .build()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    page.execute(down)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    page.execute(up).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// Activa la casilla del reCAPTCHA via teclado, sin coordenadas de pixel.
///
/// El iframe del reCAPTCHA es cross-origin (OOPIF), asi que no se puede
/// evaluar su interior ni clicar su contenido desde la pagina principal. En
/// cambio se enfoca el iframe (el foco entra en su documento), se presiona TAB
/// para mover el foco al checkbox interno (`#recaptcha-anchor`) y ESPACIO para
/// activarlo. Los eventos de teclado si se enrutan al frame enfocado, a
/// diferencia de los eventos de raton por coordenadas, cuyo enrutado falla en
/// ventanas pequenas.
async fn click_captcha_checkbox(page: &Page) -> Result<bool> {
    let ok: Option<bool> = page
        .evaluate(
            r#"(() => {
                const f = document.querySelector("iframe[src*='recaptcha/api2/anchor']");
                if (!f) return null;
                f.focus();
                return true;
            })()"#,
        )
        .await?
        .into_value()?;

    if ok.is_none() {
        return Ok(false);
    }

    tokio::time::sleep(Duration::from_millis(150)).await;

    // TAB mueve el foco al checkbox interno del iframe.
    press_key(page, "Tab", "Tab", 9).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;

    // ESPACIO activa la casilla.
    press_key(page, " ", "Space", 32).await?;

    Ok(true)
}

/// Detecta si esta visible el reto de imagenes (bframe visible).
///
/// El bframe existe siempre en el DOM (incluso sin reto), pero oculto con
/// `visibility: hidden` y desplazado fuera de la pantalla, por eso hay que
/// comprobar la visibilidad y no solo el tamano.
async fn image_challenge_visible(page: &Page) -> Result<bool> {
    let js = r#"(() => {
        for (const f of document.querySelectorAll('iframe')) {
            const s = f.getAttribute('src') || '';
            if (!s.includes('recaptcha/api2/bframe')) continue;
            const cs = getComputedStyle(f);
            if (cs.visibility === 'hidden' || cs.display === 'none') continue;
            const r = f.getBoundingClientRect();
            if (r.width > 0 && r.height > 0 && r.top > -1000) return true;
        }
        return false;
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Lee el token de reCAPTCHA del textarea oculto del frame principal.
///
/// Se usa como senal de exito: cuando la casilla se supera, el widget rellena
/// `g-recaptcha-response` en el documento principal.
async fn captcha_token(page: &Page) -> Result<String> {
    let js = r#"document.querySelector('textarea[name="g-recaptcha-response"]')?.value ?? ''"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Sondea hasta que aparezca el token (casilla superada) o salte el reto.
async fn wait_captcha_token(page: &Page, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        let token = captcha_token(page).await.unwrap_or_default();
        if token.len() > 20 {
            return true;
        }

        if image_challenge_visible(page).await.unwrap_or(false) {
            return false;
        }

        if start.elapsed() >= timeout {
            return false;
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Envia el formulario del captcha pulsando "Verificar".
async fn submit_captcha(page: &Page) -> Result<()> {
    // El banner de cookies puede tapar el boton.
    let _ = accept_cookies(page).await;

    if let Ok(btn) = page
        .find_element("#form_capados_recaptcha input[type='submit']")
        .await
    {
        let _ = btn.scroll_into_view().await;
        let _ = btn.click().await;
    }

    let _ = tokio::time::timeout(Duration::from_secs(25), page.wait_for_navigation()).await;
    Ok(())
}

/// Intenta resolver el captcha automaticamente (Plan A). Devuelve si lo logro.
async fn try_solve_captcha(page: &Page, verboser: &dyn Verboser) -> Result<bool> {
    // El banner de consentimiento puede tapar la casilla o el boton.
    let _ = accept_cookies(page).await;

    if !click_captcha_checkbox(page).await? {
        verboser.warn("Captcha: no se encontro el iframe del anchor");
        return Ok(false);
    }

    if !wait_captcha_token(page, Duration::from_secs(10)).await {
        verboser.warn("Captcha: la casilla no se supero (posible reto de imagenes)");
        return Ok(false);
    }

    submit_captcha(page).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(!has_captcha(page).await?)
}

/// Espera (sin timeout) a que el usuario resuelva el captcha manualmente.
/// Termina cuando se desbloquea, cuando el usuario cancela o cuando cierra el
/// navegador.
async fn wait_manual(page: &Page, verboser: &dyn Verboser) -> Result<()> {
    verboser.warn(
        "Captcha detectado: resuelvelo manualmente en la ventana del navegador \
         (cierra el navegador o cancela para abortar)",
    );

    loop {
        if verboser.is_cancelled() {
            return Err(anyhow::anyhow!(
                "cancelado por el usuario durante el captcha"
            ));
        }

        let blocked = match has_captcha(page).await {
            Ok(blocked) => blocked,
            Err(_) => return Err(anyhow::anyhow!("el navegador se cerro durante el captcha")),
        };

        if !blocked {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Garantiza que la pagina no este bloqueada, aplicando los planes A, B y C.
async fn unblock<F>(
    browser: &mut Browser,
    page: &mut Page,
    config: &Config,
    ctx: &Context,
    is_correctly_loaded: impl Fn(Page) -> F,
) -> Result<()>
where
    
    F: Future<Output = Result<bool>>,
{
    let verboser = ctx.verboser();

    verboser.warn("Captcha/429 detectado en Empresite; intentando resolverlo");

    // Plan A: click automatico.
    if try_solve_captcha(&page, verboser).await? {
        verboser.warn("Captcha resuelto automaticamente");
        return Ok(());
    }

    // Plan B: rotar VPN y reintentar. Si la rotacion no se puede completar
    // (VPN desactivada, sin ruta o fallo irrecuperable) se pasa al plan C. Si
    // se completa pero el captcha persiste, el bucle prueba con otra IP.
    if ctx.vpn_rotate().await? {
        let url = page.url().await?.unwrap();

        browser.close().await?;
        *browser = Browser::empresite_fresh(config).await?;
        *page = browser.new_page(url).await?;

        loop {
            page.wait_for_navigation_response().await?;
            let blocked = wait_until(
                || async {
                    if has_captcha(page).await? {
                        return Ok(Some(true));
                    }
                    if is_correctly_loaded(page.clone()).await? {
                        return Ok(Some(false));
                    }
                    Ok(None)
                },
                Duration::from_millis(300),
                Duration::from_secs(10),
            )
            .await;

            match blocked {
                Ok(blocked) => {
                    if !blocked {
                        return Ok(());
                    }

                    if try_solve_captcha(&page, verboser).await? {
                        verboser.warn("Captcha resuelto automaticamente");
                        return Ok(());
                    }
                }
                Err(err) => {
                    if err.is::<WaitUntilTimeoutError>() {
                        page.reload().await?;
                        continue;
                    }

                    return Err(err);
                }
            }
        }
    }

    // Plan C: resolucion manual (solo si hay navegador visible).
    if !config.empresite.headless {
        wait_manual(&page, verboser).await?;
        return Ok(());
    }

    Err(anyhow::anyhow!(
        "no se pudo resolver el captcha de Empresite"
    ))
}

// --- Fichas ------------------------------------------------------------------

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
    #[serde(default)]
    legal_name: String,
    #[serde(default)]
    tax_id: String,
    #[serde(default)]
    legal_form: String,
    #[serde(default)]
    sector: String,
    #[serde(default)]
    incorporation_date: String,
    #[serde(default)]
    last_change_date: String,
    #[serde(default)]
    corporate_purpose: String,
    #[serde(default)]
    activity: String,
    #[serde(default)]
    cnae_activity: String,
    #[serde(default)]
    company_status: String,
}

/// Extrae los datos crudos de una ficha via evaluacion JS.
async fn extract_detail(page: &Page) -> Result<DetailRaw> {
    let js = r#"(() => {
        const name = (document.querySelector('h1')?.textContent || '').trim();
        const emailEl = document.querySelector('a.email[href^="mailto:"]');
        const webEl = document.querySelector('a.url[href]');
        const telEl = document.querySelector('span.tel span.value');
        const field = (label) => {
            for (const h of document.querySelectorAll('h3')) {
                if (h.textContent.trim() === label) {
                    const el = h.nextElementSibling;
                    return el ? el.textContent.trim() : '';
                }
            }
            return '';
        };
        return {
            name: name,
            email: emailEl ? emailEl.getAttribute('href') : '',
            web: webEl ? webEl.getAttribute('href') : '',
            tfno: telEl ? telEl.textContent.trim() : '',
            legal_name: field('Razón social'),
            tax_id: field('CIF'),
            legal_form: field('Forma jurídica'),
            sector: field('Sector'),
            incorporation_date: field('Fecha de constitución'),
            last_change_date: field('Fecha último cambio'),
            corporate_purpose: field('Objeto social'),
            activity: field('Actividad'),
            cnae_activity: field('Actividad CNAE'),
            company_status: field('Estado de la empresa')
        };
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Comprueba si la pagina es un detalle real de Empresite con sus datos.
///
/// No basta con que el encabezado tenga texto: una pagina de error (servidor,
/// cambio de IP a mitad de conexion, red caida) puede incluir un titulo que se
/// confundiria con el nombre de la empresa. Se exige ademas que exista la
/// estructura exclusiva de una ficha: secciones con id fijo (`#infogeneral`,
/// `#dircont`, etc.), cabeceras tipicas de la plantilla o datos de contacto.
/// Si hay captcha devuelve `false`; el captcha se detecta aparte.
async fn is_detail_loaded(page: &Page) -> Result<bool> {
    let js = r#"(() => {
        if (location.href.startsWith('chrome-error://')) return false;
        const name = (document.querySelector('h1')?.textContent || '').trim();
        if (!name) return false;
        const seccion = document.querySelector(
            '#infogeneral, #dircont, #datoscomerciales, #rankings'
        );
        const cabecera = [...document.querySelectorAll('h2')].some((e) => {
            const t = e.textContent.trim();
            return t.includes('Información general') || t.includes('Dirección y contacto');
        });
        const contacto = document.querySelector(
            'a.email[href^="mailto:"], span.tel span.value, a.url[href]'
        );
        return !!(seccion || cabecera || contacto);
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

/// Limpia un texto generico (datos mercantiles): recorta espacios.
fn clean_text(raw: &str) -> Option<String> {
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
    base.trim()
        .strip_suffix('/')
        .unwrap_or(base.trim())
        .to_string()
}

/// Convierte el texto de la actividad en un slug apto para la URL de Empresite:
/// mayusculas, sin acentos, espacios por guiones y solo caracteres aceptados en
/// una URL (alfanumericos, `-`, `_`, `.` y `~`).
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

/// Desenlace del scraping de una ficha.
enum DetailOutcome {
    Found(Coincidence),
    Skipped,
}

/// Maximo de veces que se intenta abrir una misma ficha antes de darla por
/// inaccesible. No se pasa a la siguiente hasta agotar los intentos.
const MAX_DETAIL_ATTEMPTS: usize = 3;

/// Scrapea una ficha de empresa abriendo una pestaña nueva.
///
/// Primero comprueba el captcha (tiene su propio workflow) y despues si la
/// pagina es un detalle real de Empresite con sus datos. Si la pagina no carga
/// correctamente (error del servidor, cambio de IP, timeout), se cierra la
/// pestana y se vuelve a abrir la misma URL sin pasar a la siguiente ficha,
/// hasta un maximo de intentos.
async fn scrape_detail(
    browser: &mut Browser,
    link: &str,
    config: &Config,
    ctx: &Context,
    count: usize,
) -> Result<DetailOutcome> {
    let verboser = ctx.verboser();
    let mut page = browser.new_page(link).await?;
    for attempt in 1..=MAX_DETAIL_ATTEMPTS {
        //dismiss_dialogs(&detail_page).await;

        // Sondea hasta que la ficha este lista (detalle real con datos) o
        // aparezca un captcha, en vez de esperar a que termine la navegacion
        // completa.
        let is_blocked = wait_until(
            || async {
                if has_captcha(&page).await? {
                    return Ok(Some(true));
                }
                if is_detail_loaded(&page).await? {
                    return Ok(Some(false));
                }
                Ok(None)
            },
            Duration::from_millis(200),
            Duration::from_secs(7),
        ) 
        .await;

        match is_blocked {
            Ok(true) => {
                unblock(browser, &mut page, config, ctx, |page| async move {
                    is_detail_loaded(&page).await
                })
                .await?;
            }
            Ok(false) => {}
            Err(err) => {
                if !err.is::<WaitUntilTimeoutError>() {
                    return Err(err);
                }
                page.reload().await?;
                verboser.warn(&format!(
                    "Ficha {} no cargo correctamente (intento {}); reabriendo",
                    link, attempt
                ));
                tokio::time::sleep(random_delay(
                    config.empresite.delay_min,
                    config.empresite.delay_max,
                ))
                .await;
                continue;
            }
        }

        let raw = extract_detail(&page).await?;

        if raw.name.is_empty() {
            page.close().await?;
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

        verboser.processed_coincidence(&raw.name, count);

        page.close().await?;
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

    page.close().await?;
    Err(anyhow::anyhow!(
        "no se pudo cargar el detalle {} tras {} intentos",
        link,
        MAX_DETAIL_ATTEMPTS
    ))
}

/// Genera la URL del listado para una pagina concreta anexando los filtros
/// configurados. Si hay al menos un filtro se anade el flag `testfiltros=1`,
/// que activa el modo filtrado del endpoint.
fn listing_url(activity: &str, page: u32, cfg: &EmpresiteConfig) -> String {
    let base = if page <= 1 {
        format!("https://empresite.eleconomista.es/Actividad/{activity}/")
    } else {
        format!("https://empresite.eleconomista.es/Actividad/{activity}/PgNum-{page}/")
    };

    let filters = cfg.filter_query();
    if filters.is_empty() {
        base
    } else {
        format!("{base}?testfiltros=1&{filters}")
    }
}

/// Scrapea una pagina del listado de Empresite: extrae los enlaces a las fichas
/// y, para cada una, abre su detalle y registra los datos de contacto.
pub async fn scrape_internal(
    browser: &mut Browser,
    params: &EmpresiteParams,
    config: &Config,
    ctx: &Context,
) -> Result<ScrapeResult> {
    let verboser = ctx.verboser();

    verboser.searching_coincidences();

    let activity = activity_slug(&config.empresite.search_query);
    let url = listing_url(&activity, params.page, &config.empresite);

    let mut page = browser.new_page(&url).await?;

    page.wait_for_navigation().await?;
    verboser.accepting_cookies();
    let _ = accept_cookies(&page).await;

    if verboser.is_cancelled() {
        return Ok(ScrapeResult::empty(false));
    }

    // El listado puede venir bloqueado por captcha.
    if has_captcha(&page).await? {
        unblock(browser, &mut page, &config, ctx, |_page| async move {
            Ok(true)
        })
        .await?;
    }

    let links = extract_company_links(&page).await?;
    verboser.found_multiple_coincidences();

    let has_more = has_next_page(&page, params.page).await?;

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

        if let DetailOutcome::Found(coincidence) =
            scrape_detail(browser, &link, &config, ctx, idx + 1).await?
        {
            coincidences.push(coincidence);
        }
    }

    Ok(ScrapeResult::new(coincidences, has_more))
}

pub async fn scrape(
    params: &EmpresiteParams,
    config: &Config,
    ctx: &Context,
) -> Result<ScrapeResult> {
    let mut browser = Browser::empresite(config).await?;
    let out = scrape_internal(&mut browser, params, config, ctx).await;
    browser.close().await?;
    out
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
    fn test_listing_url_no_filters() {
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
    fn test_listing_url_with_filters() {
        use crate::types::{CompanySize, EmployeeRange, IncorporationDate, LegalForm};

        let cfg = EmpresiteConfig {
            web: true,
            phone: true,
            email: true,
            location: true,
            branch: true,
            company_size: Some(CompanySize::Corporate),
            employees: Some(EmployeeRange { min: 10, max: 50 }),
            incorporation_date: Some(IncorporationDate::LastYear),
            legal_form: Some(LegalForm::LimitedLiabilityCompany),
            ..Default::default()
        };

        assert_eq!(
            listing_url("BARCOS-DE-VELA", 2, &cfg),
            "https://empresite.eleconomista.es/Actividad/BARCOS-DE-VELA/PgNum-2/\
             ?testfiltros=1&emp_web=true&emp_telefono=true&emp_email=true&municipio=true&\
             numSucursales=true&emp_ventas_number=corporativas&emp_empleados_number=10-50&\
             fecha_constitucion=1a&emp_formajuridica=B"
        );
    }

    #[test]
    fn test_employee_range_single_value() {
        let range = crate::types::EmployeeRange { min: 25, max: 25 };
        assert_eq!(range.query_value(), "25");
    }
}
