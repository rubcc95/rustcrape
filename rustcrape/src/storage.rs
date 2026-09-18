use crate::types::{Coincidence, CoincidenceColumn, CoincidencePage, ProjectStats, SortOrder};
use anyhow::Result;

/// Resultado de persistir un lote de coincidencias: cuantas se insertaron
/// realmente y cuantas de ellas aportan telefono.
#[derive(Debug, Clone, Copy, Default)]
pub struct WriteOutcome {
    pub inserted: u64,
    pub inserted_with_phone: u64,
}

#[allow(async_fn_in_trait)]
pub trait Persistence: Send + Sync {
    /// Escribe coincidencias etiquetadas con el target que las produjo.
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<WriteOutcome>;

    /// Estado agregado de la base de datos (tareas, resultados y telefonos).
    async fn stats(&self) -> Result<ProjectStats>;

    /// Pagina ordenada de filas de la tabla `coincidences`.
    async fn list_coincidences(
        &self,
        column: CoincidenceColumn,
        order: SortOrder,
        limit: u32,
        offset: u32,
    ) -> Result<CoincidencePage>;

    // --- Google Maps: rejilla de bounds ---
    async fn has_bounds(&self) -> Result<bool>;
    async fn seed_bounds(&self, centers: &[(f64, f64)]) -> Result<()>;
    //async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>>;
    async fn claim_bound(&self) -> Result<Option<(i64, f32, f32)>>;
    async fn release_bound(
        &self,
        bound_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool>;

    // --- Empresite: cola dinamica de paginas ---
    async fn has_empresite_pages(&self) -> Result<bool>;
    async fn insert_empresite_page(&self, page: u32) -> Result<()>;
    //async fn read_empresite_page(&self) -> Result<Option<(i64, u32)>>;
    async fn claim_empresite_page(&self) -> Result<Option<(i64, u32)>>;
    async fn release_empresite_page(
        &self,
        page_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool>;
}
