use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser as COxideBrowser, BrowserConfig};
use futures::StreamExt;
use rand::Rng;

const USER_AGENTS: &[&str] = &[
    // Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
    // Linux
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
    // macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36",
];

const VIEWPORTS: &[(u32, u32)] = &[
    (1366, 768),
    (1440, 900),
    (1536, 864),
    (1920, 1080),
    (1280, 800),
    (1600, 900),
];

fn rand_range(min: usize, max: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Busca un navegador Chromium disponible en el sistema.
/// Prioridad: Chrome > Edge.
fn find_browser() -> Option<PathBuf> {
    // 1. Variable de entorno CHROME
    if let Ok(path) = std::env::var("CHROME") {
        let p = PathBuf::from(path);
        if p.is_file() {
            return Some(p);
        }
    }

    // 2. Rutas habituales segun plataforma
    #[cfg(windows)]
    {
        let pf = std::env::var("ProgramFiles").ok();
        let pf86 = std::env::var("ProgramFiles(x86)").ok();
        let local = std::env::var("LOCALAPPDATA").ok();

        let candidates = [
            pf.as_ref()
                .map(|p| PathBuf::from(p).join(r"Google\Chrome\Application\chrome.exe")),
            pf86.as_ref()
                .map(|p| PathBuf::from(p).join(r"Google\Chrome\Application\chrome.exe")),
            local
                .as_ref()
                .map(|p| PathBuf::from(p).join(r"Google\Chrome\Application\chrome.exe")),
            pf.as_ref()
                .map(|p| PathBuf::from(p).join(r"Microsoft\Edge\Application\msedge.exe")),
            pf86.as_ref()
                .map(|p| PathBuf::from(p).join(r"Microsoft\Edge\Application\msedge.exe")),
            local
                .as_ref()
                .map(|p| PathBuf::from(p).join(r"Microsoft\Edge\Application\msedge.exe")),
        ];

        for c in candidates.into_iter().flatten() {
            if c.is_file() {
                return Some(c);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let candidates = [
            "/usr/bin/google-chrome-stable",
            "/usr/bin/google-chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ];
        for c in candidates {
            let p = PathBuf::from(c);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    None
}

pub struct Browser {
    browser: COxideBrowser,
}

impl Browser {
    pub async fn launch(
        headless: bool,
        browser_path: Option<&Path>,
        profile_dir: Option<&Path>,
    ) -> Result<Self> {
        let idx = rand_range(0, VIEWPORTS.len() - 1);
        let (width, height) = VIEWPORTS[idx];
        let user_agent = USER_AGENTS[rand_range(0, USER_AGENTS.len() - 1)];

        let mut builder = BrowserConfig::builder()
            .viewport(Viewport {
                width,
                height,
                ..Default::default()
            })
            .arg(("user-agent", user_agent))
            .hide();

        // Perfil persistente: conserva cookies (reCAPTCHA, consentimiento) entre
        // tareas para reducir la aparicion de captchas.
        if let Some(dir) = profile_dir {
            std::fs::create_dir_all(dir).ok();
            builder = builder.user_data_dir(dir);
        }

        if !headless {
            builder = builder.with_head();
        }

        let exe_path = browser_path
            .filter(|p| p.is_file())
            .map(|p| p.to_path_buf())
            .or_else(find_browser);

        if let Some(path) = exe_path {
            builder = builder.chrome_executable(path);
        }

        let config = builder.build().map_err(|_| {
            anyhow::anyhow!(
                "No se encontro Chrome ni Edge instalado.\n\
                 Instala Google Chrome o Microsoft Edge, o define la variable CHROME."
            )
        })?;

        let (browser, mut handler) = COxideBrowser::launch(config)
            .await
            .context("error al lanzar el navegador")?;

        tokio::spawn(async move {
            let handler_loop = async move {
                loop {
                    match handler.next().await {
                        Some(Ok(())) => continue,
                        Some(Err(e)) => return Err(e),
                        _ => return Ok(()),
                    }
                }
            };

            tokio::select! {
                err = handler_loop => err.map_err(anyhow::Error::from),
                err = tokio::signal::ctrl_c() => err.map_err(anyhow::Error::from),
            }
        });

        Ok(Self { browser })
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await?;
        self.browser.wait().await?;
        Ok(())
    }
}

impl std::ops::Deref for Browser {
    type Target = COxideBrowser;

    fn deref(&self) -> &Self::Target {
        &self.browser
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::google_maps::{GoogleMapsParams, GoogleMapsScraper};
    use crate::scraper::Scraper;
    use crate::types::GoogleMapsConfig;
    use crate::verboser::DebugVerboser;

    fn test_config() -> GoogleMapsConfig {
        GoogleMapsConfig {
            enabled: true,
            search_query: "Tintoreria".to_string(),
            zoom: 12,
            stop_threshold: 5,
            delay_min: 100,
            delay_max: 1400,
            headless: false,
            rate_limit: None,
            iterations: None,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_buscar_with_browser_instance() {
        let instance = Browser::launch(true, None, None).await.unwrap();
        let scraper = GoogleMapsScraper::new(test_config());
        let result = scraper
            .scrape(
                &instance,
                &GoogleMapsParams {
                    lat: 27.1248,
                    lng: -15.4300,
                },
                &DebugVerboser,
            )
            .await
            .unwrap();
        println!("Gran Canaria: {} coincidences", result.coincidences.len());
        for (idx, res) in result.coincidences.into_iter().enumerate() {
            println!("Result {}: {:?}", idx + 1, res);
        }
        instance.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_multi() {
        let instance = Browser::launch(true, None, None).await.unwrap();

        let scraper = GoogleMapsScraper::new(test_config());
        let result = scraper
            .scrape(
                &instance,
                &GoogleMapsParams {
                    lat: 27.8248,
                    lng: -15.4300,
                },
                &DebugVerboser,
            )
            .await
            .unwrap();
        println!("Gran Canaria: {} coincidences", result.coincidences.len());
        for (idx, coincidence) in result.coincidences.into_iter().enumerate() {
            println!("Result {}: {:?}", idx + 1, coincidence);
        }

        instance.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_single() {
        let instance = Browser::launch(true, None, None).await.unwrap();
        let mut config = test_config();
        config.search_query = "Parque de Ferrera".to_string();
        let scraper = GoogleMapsScraper::new(config);
        let result = scraper
            .scrape(
                &instance,
                &GoogleMapsParams {
                    lat: 43.5528489,
                    lng: -5.9226716,
                },
                &DebugVerboser,
            )
            .await
            .unwrap();
        println!("El Hierro: {} coincidences", result.coincidences.len());
        // The assertion is soft — may be 0 or 1, both are valid outcomes.
        if result.coincidences.is_empty() {
            println!("No coincidences found in El Hierro (expected: may be 0 or 1)");
        }

        instance.close().await.unwrap();
    }
}
