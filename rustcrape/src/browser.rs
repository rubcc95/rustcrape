use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::emulation::{
    SetUserAgentOverrideParams, UserAgentBrandVersion, UserAgentMetadata,
};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser as COxideBrowser, BrowserConfig, Page};
use futures::StreamExt;
use rand::Rng;

use crate::types::Config;

/// Versiones de Chrome usadas para construir un User-Agent y sus Client Hints
/// (`Sec-CH-UA`) coherentes entre si. Solo Windows, porque la huella real del
/// equipo (WebGL, fuentes, resolucion) es la de una maquina Windows: simular
/// Linux/macOS delataria la incoherencia.
const CHROME_VERSIONS: &[(&str, &str)] = &[
    ("131.0.6778.140", "131"),
    ("130.0.6723.117", "130"),
    ("132.0.6834.84", "132"),
];

fn build_user_agent(major: &str) -> String {
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36",
        major
    )
}

fn build_user_agent_metadata(full: &str, major: &str) -> UserAgentMetadata {
    UserAgentMetadata::builder()
        .brands([
            UserAgentBrandVersion::new("Chromium", major),
            UserAgentBrandVersion::new("Google Chrome", major),
            UserAgentBrandVersion::new("Not_A Brand", "24"),
        ])
        .full_version_lists([
            UserAgentBrandVersion::new("Chromium", full),
            UserAgentBrandVersion::new("Google Chrome", full),
            UserAgentBrandVersion::new("Not_A Brand", "24.0.0.0"),
        ])
        .platform("Windows")
        .platform_version("10.0.0")
        .architecture("x86")
        .model("")
        .mobile(false)
        .bitness("64")
        .build()
        .expect("metadata de User-Agent siempre valido")
}

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

// /// Responde automaticamente a los dialogos JS (`alert`, `confirm`, `prompt`).
// ///
// /// Con el dominio `Page` activo un `alert` bloquea la ejecucion JS del renderer
// /// hasta que se responde; sin handler, las evaluaciones posteriores se quedan
// /// colgadas. Esta tarea los descarta en cuanto aparecen.
// pub async fn dismiss_dialogs(page: &Page) -> Result<()> {
//     //tokio::spawn(async move {
//     let Ok(mut events) = page.event_listener::<EventJavascriptDialogOpening>().await else {
//         return Ok(());
//     };

//     while events.next().await.is_some() {
//         page.execute(HandleJavaScriptDialogParams::new(false)).await?;
//     }

//     Ok(())
// }
pub struct Browser {
    browser: COxideBrowser,
    user_agent: String,
    user_agent_metadata: UserAgentMetadata,
}

impl Browser {
    pub async fn new(
        headless: bool,
        browser_path: Option<&Path>,
        profile_dir: Option<&Path>,
    ) -> Result<Self> {
        let idx = rand_range(0, VIEWPORTS.len() - 1);
        let (width, height) = VIEWPORTS[idx];
        let (full, major) = CHROME_VERSIONS[rand_range(0, CHROME_VERSIONS.len() - 1)];
        let user_agent = build_user_agent(major);
        let user_agent_metadata = build_user_agent_metadata(full, major);

        let mut builder = BrowserConfig::builder()
            .viewport(Viewport {
                width,
                height,
                ..Default::default()
            })
            .window_size(width, height)
            .hide();

        // Perfil persistente: conserva cookies entre lanzamientos. Para rotar de
        // IP hay que usar un perfil efimero (empresite_fresh), porque reutilizar
        // cookies marcadas con una IP distinta delata el cambio.
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

        Ok(Self {
            browser,
            user_agent,
            user_agent_metadata,
        })
    }

    /// Directorio raiz de los perfiles persistentes de Chrome.
    fn profile_root(config: &Config) -> PathBuf {
        match config.browser_profile_dir.as_deref() {
            Some(dir) => Path::new(dir).join("rustcrape-profiles"),
            None => std::env::temp_dir().join("rustcrape-profiles"),
        }
    }

    /// Abre una pagina aplicando antes el User-Agent (y sus Client Hints)
    /// elegidos para esta sesion. Se crea en `about:blank` para poder inyectar
    /// la anulacion de UA antes de emitir la primera peticion de red.
    pub async fn new_page(&self, url: impl Into<String>) -> Result<Page> {
        let page = self.browser.new_page("about:blank").await?;

        let params = SetUserAgentOverrideParams::builder()
            .user_agent(self.user_agent.clone())
            .user_agent_metadata(self.user_agent_metadata.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        page.execute(params).await?;

        page.goto(url).await?;
        Ok(page)
    }

    pub async fn empresite(config: &Config) -> Result<Self> {
        Ok(Self::new(
            config.empresite.headless,
            config.browser_path.as_deref().map(Path::new),
            Some(&Self::profile_root(config).join("empresite")),
        )
        .await?)
    }

    /// Variante con perfil efimero, usada tras rotar la IP: no arrastra cookies
    /// de reCAPTCHA (`rc::c`/`_GRECAPTCHA`) que correlacionarian la sesion
    /// anterior con la nueva direccion.
    pub async fn empresite_fresh(config: &Config) -> Result<Self> {
        let fresh = Self::profile_root(config).join(format!(
            "empresite-fresh-{}-{}",
            std::process::id(),
            rand_range(0, 1_000_000)
        ));
        Ok(Self::new(
            config.empresite.headless,
            config.browser_path.as_deref().map(Path::new),
            Some(&fresh),
        )
        .await?)
    }

    pub async fn gmaps(config: &Config) -> Result<Self> {
        Ok(Self::new(
            config.gmaps.headless,
            config.browser_path.as_deref().map(Path::new),
            Some(&Self::profile_root(config).join("gmaps")),
        )
        .await?)
    }

    pub async fn close(&mut self) -> Result<()> {
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

    //use crate::types::GMapsConfig;

    // fn test_config() -> GMapsConfig {
    //     GMapsConfig {
    //         enabled: true,
    //         search_query: "Tintoreria".to_string(),
    //         zoom: 12,
    //         stop_threshold: 5,
    //         delay_min: 100,
    //         delay_max: 1400,
    //         headless: false,
    //         rate_limit: None,
    //         iterations: None,
    //     }
    // }

    // #[tokio::test]
    // #[ignore]
    // async fn test_buscar_with_browser_instance() {
    //     //let instance = Browser::launch(true, None, None).await.unwrap();
    //     let scraper = GoogleMapsScraper::new(test_config());
    //     let result = scraper
    //         .scrape(
    //             //&instance,
    //             GoogleMapsParams {
    //                 lat: 27.1248,
    //                 lng: -15.4300,
    //             },
    //             &DebugVerboser,
    //         )
    //         .await
    //         .unwrap();
    //     println!("Gran Canaria: {} coincidences", result.coincidences.len());
    //     for (idx, res) in result.coincidences.into_iter().enumerate() {
    //         println!("Result {}: {:?}", idx + 1, res);
    //     }
    //     //instance.close().await.unwrap();
    // }

    // #[tokio::test]
    // #[ignore]
    // async fn search_multi() {
    //     //let instance = Browser::launch(true, None, None).await.unwrap();

    //     let scraper = GoogleMapsScraper::new(test_config());
    //     let result = scraper
    //         .scrape(
    //             //&instance,
    //             &GoogleMapsParams {
    //                 lat: 27.8248,
    //                 lng: -15.4300,
    //             },
    //             &DebugVerboser,
    //         )
    //         .await
    //         .unwrap();
    //     println!("Gran Canaria: {} coincidences", result.coincidences.len());
    //     for (idx, coincidence) in result.coincidences.into_iter().enumerate() {
    //         println!("Result {}: {:?}", idx + 1, coincidence);
    //     }

    //     //instance.close().await.unwrap();
    // }

    // #[tokio::test]
    // #[ignore]
    // async fn search_single() {
    //     //let instance = Browser::launch(true, None, None).await.unwrap();
    //     let mut config = test_config();
    //     config.search_query = "Parque de Ferrera".to_string();
    //     let scraper = GoogleMapsScraper::new(config);
    //     let result = scraper
    //         .scrape(
    //             //&instance,
    //             &GoogleMapsParams {
    //                 lat: 43.5528489,
    //                 lng: -5.9226716,
    //             },
    //             &DebugVerboser,
    //         )
    //         .await
    //         .unwrap();
    //     println!("El Hierro: {} coincidences", result.coincidences.len());
    //     // The assertion is soft — may be 0 or 1, both are valid outcomes.
    //     if result.coincidences.is_empty() {
    //         println!("No coincidences found in El Hierro (expected: may be 0 or 1)");
    //     }

    //     //instance.close().await.unwrap();
    // }
}
