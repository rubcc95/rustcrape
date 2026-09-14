use crate::types::Coincidence;
use anyhow::Result;

#[allow(async_fn_in_trait)]
pub trait Persistence: Send + Sync {
    /// Escribe coincidencias etiquetadas con el target que las produjo.
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<u64>;

    // --- Google Maps: rejilla de bounds ---
    async fn has_bounds(&self) -> Result<bool>;
    async fn seed_bounds(&self, centers: &[(f64, f64)]) -> Result<()>;
    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>>;
    async fn claim_bound(&self, bound_id: i64) -> Result<bool>;
    async fn release_bound(
        &self,
        bound_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool>;

    // --- Empresite: cola dinamica de paginas ---
    async fn has_empresite_pages(&self) -> Result<bool>;
    async fn insert_empresite_page(&self, page: u32) -> Result<()>;
    async fn read_empresite_page(&self) -> Result<Option<(i64, u32)>>;
    async fn claim_empresite_page(&self, page_id: i64) -> Result<bool>;
    async fn release_empresite_page(
        &self,
        page_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool>;
}
