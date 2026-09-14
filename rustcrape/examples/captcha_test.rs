use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType,
};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use std::time::Duration;

async fn token(page: &chromiumoxide::Page) -> String {
    page.evaluate(r#"document.querySelector('textarea[name="g-recaptcha-response"]')?.value ?? ''"#)
        .await
        .map(|v| v.into_value().unwrap_or_default())
        .unwrap_or_default()
}

async fn bframe_state(page: &chromiumoxide::Page) -> String {
    page.evaluate(
        r#"(() => {
            for (const f of document.querySelectorAll('iframe')) {
                const s = f.getAttribute('src') || '';
                if (!s.includes('bframe')) continue;
                const cs = getComputedStyle(f);
                const r = f.getBoundingClientRect();
                return `${cs.visibility}|${cs.display}|top=${r.top}|w=${r.width}`;
            }
            return 'none';
        })()"#,
    )
    .await
    .map(|v| v.into_value().unwrap_or_default())
    .unwrap_or_default()
}

/// Envia una pulsacion de tecla (keyDown + keyUp) por CDP.
async fn press_key(page: &chromiumoxide::Page, key: &str, code: &str, vk: i64) -> Result<()> {
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
    page.execute(down).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    page.execute(up).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

/// Activa la casilla del reCAPTCHA via teclado (independiente del tamano y de
/// coordenadas): enfoca el iframe anchor, TAB para mover el foco al checkbox
/// interno (#recaptcha-anchor) y ESPACIO para activarlo.
async fn click_captcha_checkbox(page: &chromiumoxide::Page) -> Result<bool> {
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
    press_key(page, "Tab", "Tab", 9).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    press_key(page, " ", "Space", 32).await?;

    Ok(true)
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://empresite.eleconomista.es/Actividad/TINTORERIA/".to_string());

    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

    let config = BrowserConfig::builder()
        .viewport(None)
        .window_size(1366, 768)
        .arg(("user-agent", user_agent))
        .hide()
        .with_head()
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;

    let (mut browser, mut handler) = Browser::launch(config).await?;
    tokio::spawn(async move { while handler.next().await.is_some() {} });

    let page = browser.new_page(url.as_str()).await?;
    let _ = page.wait_for_navigation().await;
    tokio::time::sleep(Duration::from_secs(5)).await;

    let blocked: bool = page
        .evaluate(r#"(() => !!document.querySelector("iframe[src*='recaptcha/api2/anchor']"))()"#)
        .await?
        .into_value()?;
    println!("blocked={blocked}");

    if !blocked {
        println!("Sin captcha: la pagina cargo normalmente (IP limpia).");
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = page.close().await;
        let _ = browser.close().await;
        return Ok(());
    }

    let clicked = click_captcha_checkbox(&page).await?;
    println!("clicked (teclado)={clicked}");

    for i in 0..20 {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let t = token(&page).await;
        let bf = bframe_state(&page).await;
        println!("t={}ms tokenLen={} bframe={bf}", (i + 1) * 300, t.len());
        if t.len() > 20 {
            println!("SOLVED");
            break;
        }
        if bf.starts_with("visible") {
            println!("PULSADO: reCAPTCHA mostro el reto de imagenes (el checkbox se activo)");
            break;
        }
        if i == 19 {
            println!("NOT SOLVED");
        }
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = page.close().await;
    let _ = browser.close().await;
    Ok(())
}
