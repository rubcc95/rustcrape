pub mod mysql;
pub mod sqlite;

use crate::storage::Persistence;
use crate::types::{Coincidence, DbConfig, PersistentConfig, ProjectStats};
use crate::verboser::Verboser;
use anyhow::Result;

pub enum PersistenceKind {
    Sqlite(sqlite::SqlitePersistence),
    Mysql(mysql::MysqlPersistence),
}

impl PersistenceKind {
    pub async fn create(
        config: &DbConfig,
        params: &mut PersistentConfig,
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
    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        match self {
            PersistenceKind::Sqlite(p) => p.read_bound().await,
            PersistenceKind::Mysql(p) => p.read_bound().await,
        }
    }

    async fn claim_bound(&self, bound_id: i64) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.claim_bound(bound_id).await,
            PersistenceKind::Mysql(p) => p.claim_bound(bound_id).await,
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

    async fn write_coincidences(&self, data: Vec<Coincidence>) -> Result<(u64, u64)> {
        match self {
            PersistenceKind::Sqlite(p) => p.write_coincidences(data).await,
            PersistenceKind::Mysql(p) => p.write_coincidences(data).await,
        }
    }
}

pub async fn fetch_project_stats(config: &DbConfig) -> Result<ProjectStats> {
    match config {
        DbConfig::Sqlite { path } => {
            let db_path = path
                .clone()
                .unwrap_or_else(sqlite::default_path);
            sqlite::fetch_stats(&db_path).await
        }
        DbConfig::Mysql { .. } => mysql::fetch_stats(config).await,
    }
}
