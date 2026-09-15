pub mod mysql;
pub mod sqlite;

use crate::storage::Persistence;
use crate::types::{Coincidence, DbConfig, GMapsConfig};
use crate::verboser::Verboser;
use anyhow::Result;

#[derive(Clone)]
pub enum PersistenceKind {
    Sqlite(sqlite::SqlitePersistence),
    Mysql(mysql::MysqlPersistence),
}

impl PersistenceKind {
    pub async fn create(
        config: &DbConfig,
        params: &mut GMapsConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        match config {
            DbConfig::Sqlite { path } => {
                let db_path = path
                    .clone()
                    .unwrap_or_else(sqlite::default_path);
                Ok(PersistenceKind::Sqlite(
                    sqlite::SqlitePersistence::new(&db_path, params, verboser).await?,
                ))
            }
            DbConfig::Mysql { .. } => Ok(PersistenceKind::Mysql(
                mysql::MysqlPersistence::new(config, params, verboser).await?,
            )),
        }
    }
}

impl Persistence for PersistenceKind {
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<u64> {
        match self {
            PersistenceKind::Sqlite(p) => p.write_coincidences(source, data).await,
            PersistenceKind::Mysql(p) => p.write_coincidences(source, data).await,
        }
    }

    async fn has_bounds(&self) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.has_bounds().await,
            PersistenceKind::Mysql(p) => p.has_bounds().await,
        }
    }

    async fn seed_bounds(&self, centers: &[(f64, f64)]) -> Result<()> {
        match self {
            PersistenceKind::Sqlite(p) => p.seed_bounds(centers).await,
            PersistenceKind::Mysql(p) => p.seed_bounds(centers).await,
        }
    }


    async fn claim_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        match self {
            PersistenceKind::Sqlite(p) => p.claim_bound().await,
            PersistenceKind::Mysql(p) => p.claim_bound().await,
        }
    }

    async fn release_bound(
        &self,
        bound_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.release_bound(bound_id, completed).await,
            PersistenceKind::Mysql(p) => p.release_bound(bound_id, completed).await,
        }
    }

    async fn has_empresite_pages(&self) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.has_empresite_pages().await,
            PersistenceKind::Mysql(p) => p.has_empresite_pages().await,
        }
    }

    async fn insert_empresite_page(&self, page: u32) -> Result<()> {
        match self {
            PersistenceKind::Sqlite(p) => p.insert_empresite_page(page).await,
            PersistenceKind::Mysql(p) => p.insert_empresite_page(page).await,
        }
    }

    async fn claim_empresite_page(&self) -> Result<Option<(i64, u32)>> {
        match self {
            PersistenceKind::Sqlite(p) => p.claim_empresite_page().await,
            PersistenceKind::Mysql(p) => p.claim_empresite_page().await,
        }
    }

    async fn release_empresite_page(
        &self,
        page_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.release_empresite_page(page_id, completed).await,
            PersistenceKind::Mysql(p) => p.release_empresite_page(page_id, completed).await,
        }
    }
}
