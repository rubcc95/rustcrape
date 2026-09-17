use std::num::NonZeroU32;

use anyhow::Result;

use crate::context::Context;
use crate::storage::Persistence;
use crate::types::{Coincidence, Config};
use crate::verboser::Verboser;

/// Resultado de scrapear una tarea. `has_more` indica si existe una tarea
/// siguiente que deba encolarse (Empresite: pagina siguiente).
pub struct ScrapeResult {
    pub coincidences: Vec<Coincidence>,
    pub has_more: bool,
}

impl ScrapeResult {
    pub fn new(coincidences: Vec<Coincidence>, has_more: bool) -> Self {
        Self {
            coincidences,
            has_more,
        }
    }

    pub fn empty(has_more: bool) -> Self {
        Self {
            coincidences: Vec::new(),
            has_more,
        }
    }
}

pub struct ScrapingParams<T> {
    pub specialized: T,
    pub headless: bool,
}
/// Un scraper concreto (Google Maps, Empresite, ...).
///
/// Cada target define sus propios parametros (`Params`) y su propia cola de
/// trabajo (`seed`/`claim`/`release`/`advance`), de modo que el motor puede
/// iterar sobre trabajos sin conocer como se navega la web.
#[allow(async_fn_in_trait)]
pub trait Scraper: Send + Sync {
    // /// Configuracion especifica del target.
    // type Config: Clone + Send + Sync;
    /// Parametros de una tarea concreta.
    type Params: Send;

    /// Identificador del target; se usa como columna `source` en la base de datos.    
    fn name(&self) -> &'static str;
    // //fn config(&self) -> &Self::Config;
    // fn headless(&self) -> bool;
    fn rate_limit(&self, config: &Config) -> Option<NonZeroU32>;
    fn iterations(&self, config: &Config) -> Option<NonZeroU32>;

    /// Descripcion legible de una tarea, para los mensajes de progreso.
    fn describe(&self, params: &Self::Params) -> String;

    /// Siembra la cola de trabajo del target (idempotente).
    async fn seed<P: Persistence>(
        &self,
        config: &Config,
        persist: &P,
        verboser: &dyn Verboser,
    ) -> Result<()>;

    /// Obtiene y reclama la siguiente tarea pendiente, o `None` si no quedan.
    async fn claim<P: Persistence>(&self, persist: &P) -> Result<Option<(i64, Self::Params)>>;

    /// Marca una tarea como completada (`Some`) o la deja pendiente (`None`).
    async fn release<P: Persistence>(
        &self,
        persist: &P,
        id: i64,
        done: Option<(i32, i32)>,
    ) -> Result<bool>;

    /// Encola el siguiente trabajo a partir del actual, si corresponde.
    async fn advance<P: Persistence>(
        &self,
        persist: &P,
        params: &Self::Params,
        has_more: bool,
    ) -> Result<()>;

    /// Ejecuta el scraping de una tarea.
    async fn scrape(
        &self,
        //browser: &Browser,
        config: &Config,
        params: &Self::Params,
        ctx: &Context,
    ) -> Result<ScrapeResult>;
}
