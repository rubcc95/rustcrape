use std::path::Path;
use std::str::FromStr;

use crate::generator::SPAIN;
use crate::storage::Persistence;
use crate::types::{Coincidence, PersistentConfig};
use crate::verboser::Verboser;
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

pub fn default_path() -> String {
    let base = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("rustcrape.db")
        .to_string_lossy()
        .to_string()
}

pub struct SqlitePersistence {
    pool: SqlitePool,
}

impl SqlitePersistence {
    pub async fn new(
        db_path: &str,
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let opts = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;

        // Enable WAL mode for better concurrent reads
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout=5000")
            .execute(&pool)
            .await?;

        let this = Self { pool };
        this.create_tables(params, verboser).await?;
        Ok(this)
    }

    async fn create_tables(
        &self,
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.creating_tables();

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS bounds (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                lat REAL NOT NULL,
                lng REAL NOT NULL,
                in_progress INTEGER NOT NULL DEFAULT 0,
                items INTEGER DEFAULT NULL,
                duplicated INTEGER DEFAULT NULL,
                started_at TEXT DEFAULT NULL,
                UNIQUE (lat, lng)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS coincidences (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                web TEXT DEFAULT '',
                email TEXT DEFAULT '',
                tfno TEXT DEFAULT '',
                maps TEXT DEFAULT '',
                creado TEXT DEFAULT (datetime('now')),
                UNIQUE (name, email, web, tfno)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS configuration_rustcrape (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                search_query TEXT NOT NULL,
                zoom INTEGER NOT NULL,
                version INTEGER NOT NULL DEFAULT 1
            )",
        )
        .execute(&self.pool)
        .await?;

        // Insert config only if not already present
        sqlx::query(
            "INSERT OR IGNORE INTO configuration_rustcrape (id, search_query, zoom, version) VALUES (1, ?, ?, 1)",
        )
        .bind(&params.search_query)
        .bind(params.zoom as i32)
        .execute(&self.pool)
        .await?;

        // Load persisted config (overwrites params if db already existed)
        if let Ok(Some(row)) = sqlx::query(
            "SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        {
            params.search_query = row.get("search_query");
            params.zoom = row.get("zoom");
        } 

        // Check if bounds exist; if not, seed the grid
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bounds")
            .fetch_one(&self.pool)
            .await?;

        if count.0 == 0 {
            let centers = SPAIN
                .generate_grid(params.zoom, verboser)
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            verboser.generating_bounds(centers.len(), centers.len(), centers.len());

            for chunk in centers.chunks(100) {
                let mut builder =
                    sqlx::QueryBuilder::new("INSERT OR IGNORE INTO bounds (lat, lng) ");
                builder.push_values(chunk, |mut b, (lat, lng)| {
                    b.push_bind(lat);
                    b.push_bind(lng);
                });
                builder.build().execute(&self.pool).await?;
            }
        }

        Ok(())
    }
}

impl Persistence for SqlitePersistence {
    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        Ok(
            sqlx::query(
                "SELECT id, lat, lng FROM bounds WHERE (in_progress = 0 OR started_at < datetime('now', '-2 hours')) AND items IS NULL LIMIT 1"
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .next()
            .map(|row| (row.get("id"), row.get("lat"), row.get("lng"))),
        )
    }

    async fn claim_bound(&self, bound_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE bounds SET in_progress = 1, started_at = datetime('now') WHERE id = ?",
        )
        .bind(bound_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn release_bound(
        &self,
        bound_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        let result = match completed {
            Some((items, duplicated)) => sqlx::query(
                "UPDATE bounds SET in_progress = 0, started_at = datetime('now'), items = ?, duplicated = ? WHERE id = ?",
            )
            .bind(items)
            .bind(duplicated),
            None => sqlx::query(
                "UPDATE bounds SET in_progress = 0, started_at = datetime('now') WHERE id = ?",
            ),
        }
        .bind(bound_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn write_coincidences(&self, data: Vec<Coincidence>) -> Result<u64> {
        let filtered: Vec<Coincidence> = data
            .into_iter()
            .filter(|c| c.web.is_some() || c.email.is_some() || c.tfno.is_some())
            .collect();
        if filtered.is_empty() {
            return Ok(0);
        }
        let mut builder =
            sqlx::QueryBuilder::new("INSERT OR IGNORE INTO coincidences (name, web, email, tfno) ");
        builder.push_values(filtered, |mut b, c| {
            b.push_bind(c.name);
            b.push_bind(c.web.unwrap_or_default());
            b.push_bind(c.email.unwrap_or_default());
            b.push_bind(c.tfno.unwrap_or_default());
        });
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as u64)
    }
}
