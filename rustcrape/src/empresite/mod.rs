pub mod config;
mod scrape;

pub use config::EmpresiteParams;

use std::num::NonZeroU32;

use anyhow::Result;

use crate::context::Context;
use crate::scraper::{ScrapeResult, Scraper};
use crate::storage::Persistence;
use crate::types::Config;
use crate::verboser::Verboser;

pub struct EmpresiteScraper {
    //config: EmpresiteConfig,
    //profile_dir: Option<PathBuf>,
    //browser_path: Option<PathBuf>,
}

impl EmpresiteScraper {
    pub fn new(
        //config: EmpresiteConfig,
        // profile_dir: Option<PathBuf>,
        // browser_path: Option<PathBuf>,
    ) -> Self {
        Self {
            //config,
            //profile_dir,
            //browser_path,
        }
    }
}

impl Scraper for EmpresiteScraper {
    // type Config = EmpresiteConfig;
    type Params = EmpresiteParams;

    fn name(&self) -> &'static str {
        "empresite"
    }

    // // fn config(&self) -> &Self::Config {
    // //     &self.config
    // // }

    // fn headless(&self) -> bool {
    //     self.config.headless
    // }

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
        verboser: &dyn Verboser,
    ) -> Result<()> {
        if persist.has_empresite_pages().await? {
            verboser.debug("Empresite seed: pages already present, nothing to do");
        } else {
            verboser.debug("Empresite seed: inserting first listing page (PgNum-1)");
            persist.insert_empresite_page(1).await?;
        }
        Ok(())
    }

    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist.claim_empresite_page().await?.map(|(id, page)| {
            (
                id,
                EmpresiteParams {
                    page,
                    // headless: self.config.headless,
                    // browser_path: self.browser_path.as_deref(),
                    // profile_dir: self.profile_dir.as_deref(),
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
        //browser: &Browser,
        config: &Config,
        params: &Self::Params,
        ctx: &Context,
    ) -> Result<ScrapeResult> {
        scrape::scrape(params, config, ctx).await
    }
}
