use anyhow::Result;
use serde::{Deserialize, de::DeserializeOwned};

use crate::empresite::config::EmpresiteParams;
use crate::scraper::ScrapeResult;
use crate::types::{Coincidence, EmpresiteConfig};
use crate::verboser::Verboser;
use crate::vpn::VpnRotator;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
};
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::cdp::js_protocol::runtime::{EvaluateParams, ExecutionContextId};
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

/// Evalua una expresion JS dentro del contexto de ejecucion de un frame.
async fn eval_in_context<T: DeserializeOwned>(
    page: &Page,
    ctx: &ExecutionContextId,
    js: &str,
) -> Result<T> {
    let params = EvaluateParams::builder()
        .expression(js)
        .context_id(ctx.clone())
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(page.evaluate(params).await?.into_value()?)
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

// --- Captcha -----------------------------------------------------------------

/// Indica si la pagina actual es la de bloqueo (429 + reCAPTCHA).
async fn is_blocked(page: &Page) -> Result<bool> {
    let js = r#"(() => {
        if (document.querySelector("iframe[src*='recaptcha/api2/anchor']")) return true;
        const t = document.body ? document.body.innerText : '';
        return t.includes('Demasiadas peticiones');
    })()"#;    
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Localiza el frame del reCAPTCHA y su contexto de ejecucion.
async fn anchor_context(page: &Page) -> Result<Option<(FrameId, ExecutionContextId)>> {
    for frame_id in page.frames().await? {
        let Some(url) = page.frame_url(frame_id.clone()).await? else {
            continue;
        };
        if !url.contains("recaptcha/api2/anchor") {
            continue;
        }
        if let Some(ctx) = page.frame_execution_context(frame_id.clone()).await? {
            return Ok(Some((frame_id, ctx)));
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Clica la casilla del reCAPTCHA con eventos de raton reales (trusted).
///
/// El `.click()` sintetico no vale: reCAPTCHA ignora eventos `isTrusted=false`.
/// Calculamos las coordenadas (bounding box del iframe + rect interno) y
/// despachamos mouseMoved/mousePressed/mouseReleased via CDP.
async fn click_captcha_checkbox(page: &Page, ctx: &ExecutionContextId) -> Result<bool> {
    // Posicion del iframe en el viewport de la pagina principal. Se usa
    // getBoundingClientRect (viewport-relative) para que case con las
    // coordenadas que espera Input.dispatchMouseEvent.
    let iframe_rect: Option<Rect> = page
        .evaluate(
            r#"(() => {
                const f = document.querySelector("iframe[src*='recaptcha/api2/anchor']");
                if (!f) return null;
                f.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
                const r = f.getBoundingClientRect();
                return { x: r.x, y: r.y, w: r.width, h: r.height };
            })()"#,
        )
        .await?
        .into_value()?;

    let Some(iframe_rect) = iframe_rect else {
        return Ok(false);
    };

    // Posicion de la casilla dentro del iframe (viewport del propio iframe).
    let rect: Option<Rect> = eval_in_context(
        page,
        ctx,
        r#"(() => {
            const el = document.querySelector('#recaptcha-anchor');
            if (!el) return null;
            const r = el.getBoundingClientRect();
            return { x: r.x, y: r.y, w: r.width, h: r.height };
        })()"#,
    )
    .await?;

    let Some(rect) = rect else {
        return Ok(false);
    };

    let cx = iframe_rect.x + rect.x + rect.w / 2.0;
    let cy = iframe_rect.y + rect.y + rect.h / 2.0;

    let dispatch = |event_type: DispatchMouseEventType, button: Option<(MouseButton, i64)>| {
        let mut builder = DispatchMouseEventParams::builder()
            .r#type(event_type)
            .x(cx)
            .y(cy);
        if let Some((button, buttons)) = button {
            builder = builder.button(button).buttons(buttons).click_count(1);
        }
        builder.build()
    };

    let _ = page
        .execute(
            dispatch(DispatchMouseEventType::MouseMoved, None).map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = page
        .execute(
            dispatch(
                DispatchMouseEventType::MousePressed,
                Some((MouseButton::Left, 1)),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let _ = page
        .execute(
            dispatch(
                DispatchMouseEventType::MouseReleased,
                Some((MouseButton::Left, 0)),
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .await;

    Ok(true)
}

/// Detecta si esta visible el reto de imagenes (bframe con tamano).
async fn image_challenge_visible(page: &Page) -> Result<bool> {
    let js = r#"(() => {
        for (const f of document.querySelectorAll('iframe')) {
            const s = f.getAttribute('src') || '';
            if (s.includes('recaptcha/api2/bframe')) {
                const r = f.getBoundingClientRect();
                if (r.width > 0 && r.height > 0) return true;
            }
        }
        return false;
    })()"#;
    Ok(page.evaluate(js).await?.into_value()?)
}

/// Sondea hasta que la casilla quede marcada o aparezca el reto de imagenes.
async fn wait_checkbox_checked(page: &Page, ctx: &ExecutionContextId, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    loop {
        let checked: bool = eval_in_context(
            page,
            ctx,
            "document.querySelector('#recaptcha-anchor')?.getAttribute('aria-checked') === 'true'",
        )
        .await
        .unwrap_or(false);

        if checked {
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
    let Some((_frame_id, ctx)) = anchor_context(page).await? else {
        verboser.warn("anchor_context falló!");
        return Ok(false);
    };

    if !click_captcha_checkbox(page, &ctx).await? {
        
        verboser.warn("click_captcha_checkbox falló!");
        return Ok(false);
    }

    if !wait_checkbox_checked(page, &ctx, Duration::from_secs(10)).await {
        verboser.warn("wait_checkbox_checked falló!");
        //verboser.warn("Captcha: la casilla no se marco (posible reto de imagenes)");
        return Ok(false);
    }

    submit_captcha(page).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(!is_blocked(page).await?)
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

        let blocked = match is_blocked(page).await {
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
async fn ensure_unblocked(
    page: &Page,
    config: &EmpresiteConfig,
    vpn: &VpnRotator,
    verboser: &dyn Verboser,
) -> Result<()> {
    if !is_blocked(page).await? {
        return Ok(());
    }

    verboser.warn("Captcha/429 detectado en Empresite; intentando resolverlo");

    // Plan A: click automatico.
    if try_solve_captcha(page, verboser).await? {
        verboser.warn("Captcha resuelto automaticamente");
        return Ok(());
    }

    // Plan B: rotar VPN y reintentar.
    if vpn.force_rotate(verboser).await? {
        verboser.warn("VPN rotada; recargando pagina");

        let current = page.url().await?.unwrap_or_default();
        if !current.is_empty() {
            let _ = page.goto(current).await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;

        if !is_blocked(page).await? {
            return Ok(());
        }

        if try_solve_captcha(page, verboser).await? {
            return Ok(());
        }
    }

    // Plan C: resolucion manual (solo si hay navegador visible).
    if !config.headless {
        wait_manual(page, verboser).await?;
        return Ok(());
    }

    // Sin VPN y sin modo manual: pequeno backoff para no martillear.
    tokio::time::sleep(Duration::from_secs(30)).await;
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

/// Desenlace del scraping de una ficha.
enum DetailOutcome {
    Found(Coincidence),
    Skipped,
    Blocked,
}

/// Scrapea una ficha de empresa abriendo una pestaña nueva.
async fn scrape_detail(
    browser: &Browser,
    link: &str,
    config: &EmpresiteConfig,
    vpn: &VpnRotator,
    verboser: &dyn Verboser,
    count: usize,
) -> Result<DetailOutcome> {
    let detail_page = browser.new_page(link).await?;
    let _ = detail_page.wait_for_navigation().await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    if is_blocked(&detail_page).await? {
        if ensure_unblocked(&detail_page, config, vpn, verboser)
            .await
            .is_err()
        {
            let _ = detail_page.close().await;
            return Ok(DetailOutcome::Blocked);
        }
    }

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
        return Ok(DetailOutcome::Skipped);
    }

    let email = clean_email(&raw.email);
    let web = clean_web(&raw.web);
    let tfno = clean_tfno(&raw.tfno);

    verboser.processed_coincidence(&raw.name, count);

    Ok(DetailOutcome::Found(Coincidence {
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
    vpn: &VpnRotator,
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

    // El listado puede venir bloqueado por captcha.
    ensure_unblocked(&page, config, vpn, verboser).await?;

    let links = extract_company_links(&page).await?;
    verboser.found_multiple_coincidences();

    let has_more = has_next_page(&page, params.page).await?;

    let mut coincidences = Vec::new();
    let mut blocked = false;
    for (idx, link) in links.into_iter().enumerate() {
        if verboser.is_cancelled() {
            break;
        }

        tokio::time::sleep(random_delay(config.delay_min, config.delay_max)).await;

        match scrape_detail(browser, &link, config, vpn, verboser, idx + 1).await {
            Ok(DetailOutcome::Found(coincidence)) => coincidences.push(coincidence),
            Ok(DetailOutcome::Skipped) => {}
            Ok(DetailOutcome::Blocked) => {
                blocked = true;
                break;
            }
            Err(err) => verboser.warn(&format!("Error extrayendo ficha {}: {}", link, err)),
        }
    }

    // Si una ficha quedo bloqueada, reintentar la pagina entera mas tarde.
    if blocked {
        return Err(anyhow::anyhow!(
            "captcha irresoluble durante el scraping de fichas"
        ));
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
