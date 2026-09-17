use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

use crate::storage::{Persistence, WriteOutcome};
use crate::types::{Coincidence, GMapsConfig, ProjectStats};
use crate::verboser::Verboser;
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

pub fn default_path() -> String {
    let base = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join("rustcrape.db").to_string_lossy().to_string()
}

#[derive(Clone)]
pub struct SqlitePersistence {
    pool: SqlitePool,
}

impl SqlitePersistence {
    pub async fn new(
        db_path: &str,
        params: &mut GMapsConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let opts = SqliteConnectOptions::from_str(db_path)?.create_if_missing(true);
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
        params: &mut GMapsConfig,
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
                source_url TEXT DEFAULT '',
                source TEXT NOT NULL DEFAULT '',
                creado TEXT DEFAULT (datetime('now')),
                legal_name TEXT,
                tax_id TEXT,
                legal_form TEXT,
                sector TEXT,
                incorporation_date TEXT,
                last_change_date TEXT,
                corporate_purpose TEXT,
                activity TEXT,
                cnae_activity TEXT,
                company_status TEXT,
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

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS empresite_pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page INTEGER NOT NULL,
                in_progress INTEGER NOT NULL DEFAULT 0,
                items INTEGER DEFAULT NULL,
                duplicated INTEGER DEFAULT NULL,
                started_at TEXT DEFAULT NULL,
                UNIQUE (page)
            )",
        )
        .execute(&self.pool)
        .await?;

        self.ensure_coincidence_columns().await?;

        // Insert config only if not already present
        sqlx::query(
            "INSERT OR IGNORE INTO configuration_rustcrape (id, search_query, zoom, version) VALUES (1, ?, ?, 1)",
        )
        .bind(&params.search_query)
        .bind(params.zoom as i32)
        .execute(&self.pool)
        .await?;

        // Load persisted config (overwrites params if db already existed)
        if let Ok(Some(row)) =
            sqlx::query("SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1")
                .fetch_optional(&self.pool)
                .await
        {
            params.search_query = row.get("search_query");
            params.zoom = row.get("zoom");
        }

        Ok(())
    }

    async fn ensure_coincidence_columns(&self) -> Result<()> {
        let rows = sqlx::query("PRAGMA table_info(coincidences)")
            .fetch_all(&self.pool)
            .await?;
        let existing: HashSet<String> = rows.iter().map(|r| r.get("name")).collect();

        if existing.contains("maps") && !existing.contains("source_url") {
            sqlx::raw_sql("ALTER TABLE coincidences RENAME COLUMN maps TO source_url")
                .execute(&self.pool)
                .await?;
        }

        if !existing.contains("source") {
            sqlx::raw_sql("ALTER TABLE coincidences ADD COLUMN source TEXT NOT NULL DEFAULT ''")
                .execute(&self.pool)
                .await?;
        }

        const NEW_COINCIDENCE_COLUMNS: &[&str] = &[
            "legal_name",
            "tax_id",
            "legal_form",
            "sector",
            "incorporation_date",
            "last_change_date",
            "corporate_purpose",
            "activity",
            "cnae_activity",
            "company_status",
        ];

        for col in NEW_COINCIDENCE_COLUMNS {
            if !existing.contains(*col) {
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                    "ALTER TABLE coincidences ADD COLUMN {} TEXT",
                    col
                )))
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Numero de coincidencias almacenadas que aportan telefono.
    async fn count_phones(&self) -> Result<i64> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS total FROM coincidences WHERE tfno IS NOT NULL AND tfno <> ''",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("total"))
    }
}

impl Persistence for SqlitePersistence {
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<WriteOutcome> {
        let filtered: Vec<Coincidence> = data.into_iter().filter(|c| c.has_any_data()).collect();
        if filtered.is_empty() {
            return Ok(WriteOutcome::default());
        }
        let phones_before = self.count_phones().await?;
        let mut builder = sqlx::QueryBuilder::new(
            "INSERT OR IGNORE INTO coincidences (name, web, email, tfno, source_url, source, \
             legal_name, tax_id, legal_form, sector, incorporation_date, last_change_date, \
             corporate_purpose, activity, cnae_activity, company_status) ",
        );
        builder.push_values(filtered, |mut b, c| {
            b.push_bind(c.name);
            b.push_bind(c.web.unwrap_or_default());
            b.push_bind(c.email.unwrap_or_default());
            b.push_bind(c.tfno.unwrap_or_default());
            b.push_bind(c.source_url);
            b.push_bind(source);
            b.push_bind(c.legal_name);
            b.push_bind(c.tax_id);
            b.push_bind(c.legal_form);
            b.push_bind(c.sector);
            b.push_bind(c.incorporation_date);
            b.push_bind(c.last_change_date);
            b.push_bind(c.corporate_purpose);
            b.push_bind(c.activity);
            b.push_bind(c.cnae_activity);
            b.push_bind(c.company_status);
        });
        let result = builder.build().execute(&self.pool).await?;
        let inserted = result.rows_affected();
        let phones_after = self.count_phones().await?;
        Ok(WriteOutcome {
            inserted,
            inserted_with_phone: (phones_after - phones_before).max(0) as u64,
        })
    }

    async fn stats(&self) -> Result<ProjectStats> {
        let bounds = sqlx::query(
            "SELECT COUNT(*) AS total, COALESCE(SUM(items IS NOT NULL), 0) AS done FROM bounds",
        )
        .fetch_one(&self.pool)
        .await?;
        let pages = sqlx::query(
            "SELECT COUNT(*) AS total, COALESCE(SUM(items IS NOT NULL), 0) AS done FROM empresite_pages",
        )
        .fetch_one(&self.pool)
        .await?;
        let coincidences = sqlx::query(
            "SELECT COUNT(*) AS total, \
             COALESCE(SUM(CASE WHEN tfno IS NOT NULL AND tfno <> '' THEN 1 ELSE 0 END), 0) AS phones \
             FROM coincidences",
        )
        .fetch_one(&self.pool)
        .await?;

        let total = bounds.get::<i64, _>("total") + pages.get::<i64, _>("total");
        let processed = bounds.get::<i64, _>("done") + pages.get::<i64, _>("done");

        Ok(ProjectStats {
            bounds_total: total as u64,
            bounds_processed: processed as u64,
            bounds_remaining: (total - processed).max(0) as u64,
            results_found: coincidences.get::<i64, _>("total") as u64,
            phones_found: coincidences.get::<i64, _>("phones") as u64,
        })
    }

    async fn has_bounds(&self) -> Result<bool> {
        Ok(sqlx::query("SELECT 1 FROM bounds LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    async fn seed_bounds(&self, centers: &[(f64, f64)]) -> Result<()> {
        for chunk in centers.chunks(100) {
            let mut builder = sqlx::QueryBuilder::new("INSERT OR IGNORE INTO bounds (lat, lng) ");
            builder.push_values(chunk, |mut b, (lat, lng)| {
                b.push_bind(lat);
                b.push_bind(lng);
            });
            builder.build().execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn claim_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        let row = sqlx::query(
            r#"
        SELECT id, lat, lng
        FROM bounds
        WHERE (in_progress = 0 OR started_at < datetime('now', '-2 hours'))
          AND items IS NULL
        ORDER BY id
        LIMIT 1
        "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => {
                let id: i64 = row.get("id");
                let lat: f32 = row.get("lat");
                let lng: f32 = row.get("lng");

                sqlx::query(
                    "UPDATE bounds
                 SET in_progress = 1, started_at = datetime('now')
                 WHERE id = ?",
                )
                .bind(id)
                .execute(&self.pool)
                .await?;

                Some((id, lat, lng))
            }
            None => None,
        })
    }

    async fn release_bound(&self, bound_id: i64, completed: Option<(i32, i32)>) -> Result<bool> {
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

    async fn has_empresite_pages(&self) -> Result<bool> {
        Ok(sqlx::query("SELECT 1 FROM empresite_pages LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    async fn insert_empresite_page(&self, page: u32) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO empresite_pages (page) VALUES (?)")
            .bind(page as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn claim_empresite_page(&self) -> Result<Option<(i64, u32)>> {
        let row = sqlx::query(
            r#"
                SELECT id, PAGE 
                FROM empresite_pages
                WHERE (in_progress = 0 OR started_at < datetime('now', '-2 hours'))
                    AND items IS NULL          
                LIMIT 1
        "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            Some(row) => {
                let id: i64 = row.get("id");
                let page: u32 = row.get("page");
                sqlx::query(
                    r#"
                        UPDATE empresite_pages 
                        SET in_progress = 1, started_at = datetime('now')
                        WHERE id = ?                        
                    "#,
                )
                .bind(id)
                .execute(&self.pool)
                .await?;

                Some((id, page))
            }
            None => None,
        })
    }

    async fn release_empresite_page(
        &self,
        page_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        let result = match completed {
            Some((items, duplicated)) => sqlx::query(
                "UPDATE empresite_pages SET in_progress = 0, started_at = datetime('now'), items = ?, duplicated = ? WHERE id = ?",
            )
            .bind(items)
            .bind(duplicated),
            None => sqlx::query(
                "UPDATE empresite_pages SET in_progress = 0, started_at = datetime('now') WHERE id = ?",
            ),
        }
        .bind(page_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
