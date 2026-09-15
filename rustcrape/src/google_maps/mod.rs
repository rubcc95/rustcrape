pub mod config;
mod grid;
mod scrape;

pub use config::GoogleMapsParams;
pub use grid::{Border, SPAIN};

use std::num::NonZeroU32;

use anyhow::Result;
 
use crate::browser::Browser;
use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::GoogleMapsConfig;
use crate::verboser::Verboser;

pub struct GoogleMapsScraper {
    config: GoogleMapsConfig,
}

impl GoogleMapsScraper {
    pub fn new(config: GoogleMapsConfig) -> Self {
        Self { config }
    }
}

impl Scraper for GoogleMapsScraper {
    type Config = GoogleMapsConfig;
    type Params = GoogleMapsParams;

    fn name(&self) -> &'static str {
        "google_maps"
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn headless(&self) -> bool {
        self.config.headless
    }

    fn rate_limit(&self) -> Option<NonZeroU32> {
        self.config.rate_limit
    }

    fn iterations(&self) -> Option<NonZeroU32> {
        self.config.iterations
    }

    fn describe(&self, params: &Self::Params) -> String {
        format!("({}, {})", params.lat, params.lng)
    }

    async fn seed<P: Persistence>(&self, persist: &P, verboser: &dyn Verboser) -> Result<()> {
        if persist.has_bounds().await? {
            return Ok(());
        }
        let centers = SPAIN
            .generate_grid(self.config.zoom, verboser)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        persist.seed_bounds(&centers).await?;
        Ok(())
    }

    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist.claim_bound().await?.map(|(id, lat, lng)| (id, GoogleMapsParams { lat, lng })))
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
        browser: &Browser,
        params: &Self::Params,
        verboser: &dyn Verboser,
    ) -> Result<ScrapeResult> {
        scrape::scrape(browser, params, &self.config, verboser).await
    }
}
