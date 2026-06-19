use crate::types::Coincidence;
use anyhow::Result;

#[allow(async_fn_in_trait)]
pub trait Persistence: Send + Sync {
    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>>;
    async fn claim_bound(&self, bound_id: i64) -> Result<bool>;
    async fn release_bound(
        &self,
        bound_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool>;    
    async fn write_coincidences(&self, data: Vec<Coincidence>) -> Result<(u64, u64)>;
}
