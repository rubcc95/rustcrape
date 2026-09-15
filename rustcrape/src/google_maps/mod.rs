pub mod config;
mod grid;
mod scrape;

pub use config::GMapsParams;
pub use grid::{Border, SPAIN};

use std::num::NonZeroU32;

use anyhow::Result;

use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::Config;
use crate::verboser::Verboser;

pub struct GoogleMapsScraper {
    // config: GoogleMapsConfig,
    // browser_path: Option<PathBuf>,
    // profile_dir: Option<PathBuf>,
}

impl GoogleMapsScraper {
    pub fn new(// config: GoogleMapsConfig,
        // profile_dir: Option<PathBuf>,
        // browser_path: Option<PathBuf>,
    ) -> Self {
        Self {
            // config,
            // profile_dir,
            // browser_path,
        }
    }
}

impl Scraper for GoogleMapsScraper {
    type Params = GMapsParams;

    fn name(&self) -> &'static str {
        "google_maps"
    }

    // fn config(&self) -> &Self::Config {
    //     &self.config
    // }

    // fn headless(&self) -> bool {
    //     self.config.headless
    // }

    fn rate_limit(&self, config: &Config) -> Option<NonZeroU32> {
        config.gmaps.rate_limit
    }

    fn iterations(&self, config: &Config) -> Option<NonZeroU32> {
        config.gmaps.iterations
    }

    fn describe(&self, params: &Self::Params) -> String {
        format!("({}, {})", params.lat, params.lng)
    }

    async fn seed<P: Persistence>(
        &self,
        config: &Config,
        persist: &P,
        verboser: &dyn Verboser,
    ) -> Result<()> {
        if persist.has_bounds().await? {
            return Ok(());
        }
        let centers = SPAIN
            .generate_grid(config.gmaps.zoom, verboser)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        persist.seed_bounds(&centers).await?;
        Ok(())
    }

    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist.claim_bound().await?.map(|(id, lat, lng)| {
            (
                id,
                GMapsParams {
                    lat,
                    lng,
                    // headless: config.google_maps.headless,
                    // browser_path: browser_path.as_deref(),
                    // profile_dir: profile_dir.as_deref(),
                },
            )
        }))
    }

    async fn release<P: Persistence>(
        &self,
        persist: &P,
        id: i64,
        done: Option<(i32, i32)>,
    ) -> Result<bool> {
        persist.release_bound(id, done).await
    }

    async fn advance<P: Persistence>(
        &self,
        _persist: &P,
        _params: &Self::Params,
        _has_more: bool,
    ) -> Result<()> {
        Ok(())
    }

    async fn scrape(
        &self,
        //browser: &Browser,
        config: &Config,
        params: &Self::Params,
        verboser: &dyn Verboser,
    ) -> Result<ScrapeResult> {
        scrape::scrape(params, config, verboser).await
    }
}
