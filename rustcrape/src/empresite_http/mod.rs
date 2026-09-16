pub mod scrape;

use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Result;

use crate::empresite::config::EmpresiteParams;
use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::Config;
use crate::verboser::Verboser;
use crate::vpn::VpnRotator;

/// Variante de Empresite que scrapea por HTTP puro (`reqwest` + parser HTML),
/// sin navegador. Comparte la misma cola de páginas y la misma columna `source`
/// de la base de datos que el scraper original basado en chromiumoxide.
pub struct EmpresiteHttpScraper {
    vpn: Arc<VpnRotator>,
}

impl EmpresiteHttpScraper {
    pub fn new(vpn: Arc<VpnRotator>) -> Self {
        Self { vpn }
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

    async fn seed<P: Persistence>(
        &self,
        _: &Config,
        persist: &P,
        _verboser: &dyn Verboser,
    ) -> Result<()> {
        if !persist.has_empresite_pages().await? {
            persist.insert_empresite_page(1).await?;
        }
        Ok(())
    }

    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist.claim_empresite_page().await?.map(|(id, page)| {
            (
                id,
                EmpresiteParams { page },
            )
        }))
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
        config: &Config,
        params: &Self::Params,
        verboser: &dyn Verboser,
    ) -> Result<ScrapeResult> {
        scrape::scrape(params, config, &self.vpn, verboser).await
    }
}
