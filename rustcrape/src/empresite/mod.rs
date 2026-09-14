pub mod config;
mod scrape;

pub use config::EmpresiteParams;

use std::num::NonZeroU32;

use anyhow::Result;

use crate::browser::Browser;
use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::EmpresiteConfig;
use crate::verboser::Verboser;

pub struct EmpresiteScraper {
    config: EmpresiteConfig,
}

impl EmpresiteScraper {
    pub fn new(config: EmpresiteConfig) -> Self {
        Self { config }
    }
}

impl Scraper for EmpresiteScraper {
    type Config = EmpresiteConfig;
    type Params = EmpresiteParams;

    fn name(&self) -> &'static str {
        "empresite"
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
        format!("página {}", params.page)
    }

    async fn seed<P: Persistence>(&self, persist: &P, _verboser: &dyn Verboser) -> Result<()> {
        if !persist.has_empresite_pages().await? {
            persist.insert_empresite_page(1).await?;
        }
        Ok(())
    }

    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>> {
        let Some((id, page)) = persist.read_empresite_page().await? else {
            return Ok(None);
        };
        persist.claim_empresite_page(id).await?;
        Ok(Some((id, EmpresiteParams { page })))
    }

    async fn release<P: Persistence>(
        &self,
        persist: &P,
        id: i64,
        done: Option<(i32, i32)>,
    ) -> Result<bool> {
        persist.release_empresite_page(id, done).await
    }

    async fn advance<P: Persistence>(
        &self,
        persist: &P,
        params: &Self::Params,
        has_more: bool,
    ) -> Result<()> {
        if has_more {
            persist
                .insert_empresite_page(params.page.saturating_add(1))
                .await?;
        }
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
