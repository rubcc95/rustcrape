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

    async fn seed(
        &self,
        config: &Config,
        persist: &impl Persistence,
        verboser: &dyn Verboser,
    ) -> Result<()> {
        if persist.has_empresite_tasks().await? {
            verboser.debug("Empresite seed: tasks already present, nothing to do");
            return Ok(());
        }
        let locations = &config.empresite.location_filters;
        if locations.is_empty() {
            verboser.warn("Empresite seed: no locations configured, nothing to queue");
            return Ok(());
        }
        verboser.debug(&format!(
            "Empresite seed: queueing first page for {} location(s)",
            locations.len()
        ));
        for location in locations {
            persist.insert_empresite_task(location, 1).await?;
        }
        Ok(())
    }

    async fn claim(&self, persist: &impl Persistence) -> Result<Option<(i64, Self::Params)>> {
        Ok(persist
            .claim_empresite_task()
            .await?
            .map(|(id, location, page)| (id, EmpresiteParams { location, page })))
    }

    async fn release(
        &self,
        persist: &impl Persistence,
        id: i64,
        done: Option<(i32, i32)>,
    ) -> Result<bool> {
        persist.release_empresite_task(id, done).await
    }

    async fn advance(
        &self,
        persist: &impl Persistence,
        params: &Self::Params,
        has_more: bool,
    ) -> Result<()> {
        if has_more {
            persist
                .insert_empresite_task(&params.location, params.page.saturating_add(1))
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
        persist: &impl Persistence,
    ) -> Result<ScrapeResult> {
        scrape::scrape(params, config, ctx, persist).await
    }
}
