pub mod mysql;
pub mod sqlite;

use crate::storage::{Persistence, WriteOutcome};
use crate::types::{
    Coincidence, CoincidenceColumn, CoincidencePage, DbConfig, EmpresiteLocation, GMapsConfig,
    ProjectStats, SortOrder,
};
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
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<WriteOutcome> {
        match self {
            PersistenceKind::Sqlite(p) => p.write_coincidences(source, data).await,
            PersistenceKind::Mysql(p) => p.write_coincidences(source, data).await,
        }
    }

    async fn stats(&self) -> Result<ProjectStats> {
        match self {
            PersistenceKind::Sqlite(p) => p.stats().await,
            PersistenceKind::Mysql(p) => p.stats().await,
        }
    }

    async fn list_coincidences(
        &self,
        column: CoincidenceColumn,
        order: SortOrder,
        limit: u32,
        offset: u32,
    ) -> Result<CoincidencePage> {
        match self {
            PersistenceKind::Sqlite(p) => p.list_coincidences(column, order, limit, offset).await,
            PersistenceKind::Mysql(p) => p.list_coincidences(column, order, limit, offset).await,
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

    async fn empresite_activity(&self) -> Result<Option<(String, String)>> {
        match self {
            PersistenceKind::Sqlite(p) => p.empresite_activity().await,
            PersistenceKind::Mysql(p) => p.empresite_activity().await,
        }
    }

    async fn set_empresite_activity(&self, source: &str, canonical: &str) -> Result<()> {
        match self {
            PersistenceKind::Sqlite(p) => p.set_empresite_activity(source, canonical).await,
            PersistenceKind::Mysql(p) => p.set_empresite_activity(source, canonical).await,
        }
    }

    async fn has_empresite_tasks(&self) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.has_empresite_tasks().await,
            PersistenceKind::Mysql(p) => p.has_empresite_tasks().await,
        }
    }

    async fn insert_empresite_task(&self, location: &EmpresiteLocation, page: u32) -> Result<()> {
        match self {
            PersistenceKind::Sqlite(p) => p.insert_empresite_task(location, page).await,
            PersistenceKind::Mysql(p) => p.insert_empresite_task(location, page).await,
        }
    }

    async fn claim_empresite_task(&self) -> Result<Option<(i64, EmpresiteLocation, u32)>> {
        match self {
            PersistenceKind::Sqlite(p) => p.claim_empresite_task().await,
            PersistenceKind::Mysql(p) => p.claim_empresite_task().await,
        }
    }

    async fn release_empresite_task(
        &self,
        task_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        match self {
            PersistenceKind::Sqlite(p) => p.release_empresite_task(task_id, completed).await,
            PersistenceKind::Mysql(p) => p.release_empresite_task(task_id, completed).await,
        }
    }
}
