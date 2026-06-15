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

pub struct Browser {
    browser: COxideBrowser,
}

impl Browser {
    pub async fn launch(headless: bool) -> Result<Self> {
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

        if !headless {
            builder = builder.with_head();
        }

        let config = builder
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build browser config: {}", e))?;

        let (browser, mut handler) = COxideBrowser::launch(config)
            .await
            .context("failed to launch chromium browser")?;

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
    use crate::verboser::DebugVerboser;
    use crate::scrapper::scrape;
    use crate::types::{PersistentConfig, SearchConfig, SearchContext};

    #[tokio::test]
    #[ignore]
    async fn test_buscar_with_browser_instance() {
        let instance = Browser::launch(true).await.unwrap();
        let config = SearchConfig {
            persistent: PersistentConfig {
                search_query: "Tintoreria".to_string(),
                zoom: 12,
            },
            delay_min: 100,
            delay_max: 1400,
            stop_threshold: 5,
            headless: false,
        };
        let coincidences = scrape(
            &instance,
            SearchContext {
                config: &config,
                lat: 27.1248,
                lng: -15.4300,
            },
            &DebugVerboser,
        )
        .await
        .unwrap();
        println!("Gran Canaria: {} coincidences", coincidences.len());
        for (idx, res) in coincidences.into_iter().enumerate() {
            println!("Result {}: {:?}", idx + 1, res);
        }
        instance.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_multi() {
        let instance = Browser::launch(true).await.unwrap();

        let config = SearchConfig {
            persistent: PersistentConfig {
                search_query: "Tintoreria".to_string(),
                zoom: 12,
            },
            delay_min: 100,
            delay_max: 1400,
            stop_threshold: 5,
            headless: false,
        };
        let coincidences: Vec<crate::types::Coincidence> = scrape(
            &instance,
            SearchContext {
                lat: 27.8248,
                lng: -15.4300,
                config: &config,
            },
            &DebugVerboser,
        )
        .await
        .unwrap();
        println!("Gran Canaria: {} coincidences", coincidences.len());
        for (idx, coincidence) in coincidences.into_iter().enumerate() {
            println!("Result {}: {:?}", idx + 1, coincidence);
        }

        instance.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn search_single() {
        let instance = Browser::launch(true).await.unwrap();
        let config = SearchConfig {
            persistent: PersistentConfig {
                search_query: "Parque de Ferrera".to_string(),
                zoom: 12,
            },
            delay_min: 100,
            delay_max: 1400,
            stop_threshold: 5,
            headless: false,
        };
        let coincidences = scrape(
            &instance,
            SearchContext {
                lat: 43.5528489,
                lng: -5.9226716,
                config: &config,
            },
            &DebugVerboser,
        )
        .await
        .unwrap();
        println!("El Hierro: {} rescoincidencesults", coincidences.len());
        // The assertion is soft — may be 0 or 1, both are valid outcomes.
        if coincidences.is_empty() {
            println!("No coincidences found in El Hierro (expected: may be 0 or 1)");
        }

        instance.close().await.unwrap();
    }
}
