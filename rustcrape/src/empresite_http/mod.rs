pub mod scrape;

use std::num::NonZeroU32;

use anyhow::Result;

use crate::context::Context;
use crate::empresite::config::EmpresiteParams;
use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::Config;
use crate::verboser::Verboser;

/// Variante de Empresite que scrapea por HTTP puro (`reqwest` + parser HTML),
/// sin navegador. Comparte la misma cola de páginas y la misma columna `source`
/// de la base de datos que el scraper original basado en chromiumoxide.
pub struct EmpresiteHttpScraper;

impl EmpresiteHttpScraper {
    pub fn new() -> Self {
        Self
    }
}

impl Scraper for EmpresiteHttpScraper {
    type Params = EmpresiteParams;

    fn name(&self) -> &'static str {
        "empresite"
    }

    fn rate_limit(&self, config: &Config) -> Option<NonZeroU32> {
        config.empresite.rate_limit
    }

    fn iterations(&self, config: &Config) -> Option<NonZeroU32> {
        config.empresite.iterations
    }

    fn describe(&self, params: &Self::Params) -> String {
        format!("página {}", params.page)
    }

    async fn seed(
        &self,
        _: &Config,
        persist: &impl Persistence,
        verboser: &dyn Verboser,
    ) -> Result<()> {
        if persist.has_empresite_pages().await? {
            verboser.debug("Empresite HTTP seed: pages already present, nothing to do");
        } else {
            verboser.debug("Empresite HTTP seed: inserting first listing page (PgNum-1)");
            persist.insert_empresite_page(1).await?;
        }
        Ok(())
    }

    async fn claim(&self, persist: &impl Persistence) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist.claim_empresite_page().await?.map(|(id, page)| {
            (
                id,
                EmpresiteParams { page },
            )
        }))
    }

    async fn release(
        &self,
        persist: &impl Persistence,
        id: i64,
        done: Option<(i32, i32)>,
    ) -> Result<bool> {
        persist.release_empresite_page(id, done).await
    }

    async fn advance(
        &self,
        persist: &impl Persistence,
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
        config: &Config,
        params: &Self::Params,
        ctx: &Context,
        persist: &impl Persistence,
    ) -> Result<ScrapeResult> {
        scrape::scrape(params, config, ctx, persist).await
    }
}
