use crate::storage::Persistence;
use crate::types::{Coincidence, DbConfig, GoogleMapsConfig};
use crate::verboser::Verboser;
use anyhow::Result;
use sqlx::{MySqlPool, Row};

const NUMERIC_TYPES: &[&str] = &[
    "decimal", "double", "float", "int", "bigint", "smallint",
    "tinyint", "mediumint", "dec", "fixed", "numeric", "real",
];

const NEW_BOUND_COLUMNS: &[(&str, &str)] = &[
    ("in_progress", "TINYINT(1) NOT NULL DEFAULT 0"),
    ("items", "INT DEFAULT NULL"),
    ("duplicated", "INT DEFAULT NULL"),
    ("started_at", "TIMESTAMP NULL DEFAULT NULL"),
];

// Tablas imprescindibles para reconocer la base de datos como de rustcrape.
// `bounds` (Google Maps) y `empresite_pages` (Empresite) se crean bajo demanda
// segun el target que se vaya a usar.
const REQUIRED_TABLES: &[&str] = &[
    "coincidences",
    "configuration_rustcrape",
];

struct ColumnInfo {
    name: String,
    data_type: String,
}

#[derive(Clone)]
pub struct MysqlPersistence {
    pool: MySqlPool,
    db_name: String,
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    port: u16,
    #[allow(dead_code)]
    user: String,
    #[allow(dead_code)]
    password: String,
}

impl MysqlPersistence {
    pub async fn new(
        config: &DbConfig,
        params: &mut GoogleMapsConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        let (host, port, user, password, database) = match config {
            DbConfig::Mysql { host, port, user, password, database } => {
                (host.clone(), *port, user.clone(), password.clone(), database.clone())
            }
            _ => unreachable!(),
        };

        verboser.connecting_db();

        // 1. Connect to engine (no specific database)
        let engine_url = format!("mysql://{}:{}@{}:{}", user, password, host, port);
        let admin_pool = MySqlPool::connect(&engine_url).await?;

        // 2. Check if database exists
        let exists = sqlx::query(
            "SELECT 1 FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = ? LIMIT 1",
        )
        .bind(&database)
        .fetch_optional(&admin_pool)
        .await?
        .is_some();

        if !exists {
            verboser.creating_db();
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE IF NOT EXISTS `{}`",
                database.replace('`', "``"),
            )))
            .execute(&admin_pool)
            .await?;
        }

        // 3. Connect with the database
        let db_url = format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database);
        let pool = MySqlPool::connect(&db_url).await?;
        drop(admin_pool);

        let this = Self { pool, db_name: database, host, port, user, password };

        // 4. Ensure tables
        if !exists {
            this.create_tables(params, verboser).await?;
        } else {
            this.ensure_tables(params, verboser).await?;
        }

        Ok(this)
    }

    async fn get_columns(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let rows = sqlx::query(
            "SELECT COLUMN_NAME, DATA_TYPE \
             FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
             ORDER BY ORDINAL_POSITION",
        )
        .bind(&self.db_name)
        .bind(table)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| ColumnInfo {
                name: row.get(0),
                data_type: row.get(1),
            })
            .collect())
    }

    async fn ensure_bound_columns(&self) -> Result<()> {
        let cols = self.get_columns("bounds").await?;
        let existing: std::collections::HashSet<String> =
            cols.into_iter().map(|c| c.name).collect();

        for (name, def) in NEW_BOUND_COLUMNS {
            if !existing.contains(*name) {
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                    "ALTER TABLE bounds ADD COLUMN {} {}",
                    name, def
                )))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn ensure_coincidence_columns(&self) -> Result<()> {
        let cols = self.get_columns("coincidences").await?;
        let existing: std::collections::HashSet<String> =
            cols.iter().map(|c| c.name.clone()).collect();

        // Migra la antigua columna `maps` a `source_url`.
        if existing.contains("maps") && !existing.contains("source_url") {
            sqlx::raw_sql("ALTER TABLE coincidences CHANGE COLUMN maps source_url TEXT")
                .execute(&self.pool)
                .await?;
        }

        if !existing.contains("source") {
            sqlx::raw_sql(
                "ALTER TABLE coincidences ADD COLUMN source VARCHAR(50) NOT NULL DEFAULT ''",
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn create_tables(
        &self,
        params: &GoogleMapsConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.creating_tables();
        self.ensure_optional_tables().await?;

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS coincidences (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                web VARCHAR(255) DEFAULT '',
                email VARCHAR(255) DEFAULT '',
                tfno VARCHAR(50) DEFAULT '',
                source_url TEXT DEFAULT '',
                source VARCHAR(50) NOT NULL DEFAULT '',
                creado TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE KEY uq_datos (name, email, web, tfno)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS configuration_rustcrape (
                id INT PRIMARY KEY DEFAULT 1,
                search_query VARCHAR(255) NOT NULL,
                zoom INT UNSIGNED NOT NULL,
                version INT NOT NULL DEFAULT 1,
                CHECK (id = 1)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO configuration_rustcrape (id, search_query, zoom, version) VALUES (1, ?, ?, 1)",
        )
        .bind(&params.search_query)
        .bind(params.zoom as i32)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Crea (si faltan) las tablas opcionales de cada target: `bounds` para
    /// Google Maps y `empresite_pages` para Empresite. No se exigen en
    /// `REQUIRED_TABLES` para permitir bases de datos de una sola web.
    async fn ensure_optional_tables(&self) -> Result<()> {
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS bounds (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                lat FLOAT NOT NULL,
                lng FLOAT NOT NULL,
                in_progress TINYINT(1) NOT NULL DEFAULT 0,
                items INT DEFAULT NULL,
                duplicated INT DEFAULT NULL,
                started_at TIMESTAMP NULL DEFAULT NULL,
                UNIQUE KEY uq_bound (lat, lng)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS empresite_pages (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                page INT NOT NULL,
                in_progress TINYINT(1) NOT NULL DEFAULT 0,
                items INT DEFAULT NULL,
                duplicated INT DEFAULT NULL,
                started_at TIMESTAMP NULL DEFAULT NULL,
                UNIQUE KEY uq_empresite_page (page)
            )",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_tables(
        &self,
        params: &mut GoogleMapsConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.verifying_db();
        let rows = sqlx::query("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ?")
            .bind(&self.db_name)
            .fetch_all(&self.pool)
            .await?;

        if rows.is_empty() {
            return self.create_tables(params, verboser).await;
        }

        let existing: std::collections::HashSet<String> =
            rows.iter().map(|row| row.get(0)).collect();

        let missing: Vec<&str> = REQUIRED_TABLES
            .iter()
            .filter(|t| !existing.contains(**t))
            .copied()
            .collect();

        if missing.is_empty() {
            self.ensure_optional_tables().await?;
            self.ensure_bound_columns().await?;
            self.ensure_coincidence_columns().await?;

            self.validate_bounds().await?;
            self.validate_coincidences().await?;
            self.validate_coincidences_unique().await?;

            let config_row = sqlx::query(
                "SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1",
            )
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("configuration_rustcrape table is empty"))?;
            params.search_query = config_row.get("search_query");
            params.zoom = config_row.get("zoom");
            return Ok(());
        }

        anyhow::bail!(
            "Tablas faltantes: {}. Deben existir todas: {}.",
            missing.join(", "),
            REQUIRED_TABLES.join(", ")
        );
    }

    async fn validate_bounds(&self) -> Result<()> {
        let cols = self.get_columns("bounds").await?;
        let names: std::collections::HashSet<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        if !names.contains("lat") || !names.contains("lng") {
            anyhow::bail!("Bounds table: missing columns lat and/or lng");
        }
        for col in cols {
            if (col.name == "lat" || col.name == "lng")
                && !NUMERIC_TYPES.contains(&col.data_type.as_str())
            {
                anyhow::bail!(
                    "Bounds table: column '{}' must be numeric (current type: {})",
                    col.name,
                    col.data_type
                );
            }
        }
        Ok(())
    }

    async fn validate_coincidences(&self) -> Result<()> {
        let cols = self.get_columns("coincidences").await?;
        let required = ["name", "email", "web", "tfno", "source_url"];
        let names: std::collections::HashSet<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        let missing: Vec<&str> = required.iter().filter(|n| !names.contains(*n)).copied().collect();
        if !missing.is_empty() {
            anyhow::bail!("Table coincidences: missing columns: {}", missing.join(", "));
        }
        Ok(())
    }

    async fn validate_coincidences_unique(&self) -> Result<()> {
        let rows = sqlx::query(
            "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = 'coincidences' \
             AND CONSTRAINT_NAME = 'uq_datos' ORDER BY ORDINAL_POSITION",
        )
        .bind(&self.db_name)
        .fetch_all(&self.pool)
        .await?;

        let cols: Vec<String> = rows.iter().map(|r| r.get(0)).collect();
        let expected = ["name", "email", "web", "tfno"];
        if cols != expected {
            anyhow::bail!(
                "Table coincidences: expected UNIQUE KEY uq_datos on (name, email, web, tfno), found on ({})",
                cols.join(", ")
            );
        }
        Ok(())
    }
}

impl Persistence for MysqlPersistence {
    async fn write_coincidences(&self, source: &str, data: Vec<Coincidence>) -> Result<u64> {
        let filtered: Vec<Coincidence> = data
            .into_iter()
            .filter(|c| c.web.is_some() || c.email.is_some() || c.tfno.is_some())
            .collect();
        if filtered.is_empty() {
            return Ok(0);
        }
        let mut builder = sqlx::QueryBuilder::new(
            "INSERT IGNORE INTO coincidences (name, web, email, tfno, source_url, source) ",
        );
        builder.push_values(filtered, |mut b, c| {
            b.push_bind(c.name);
            b.push_bind(c.web.unwrap_or_default());
            b.push_bind(c.email.unwrap_or_default());
            b.push_bind(c.tfno.unwrap_or_default());
            b.push_bind(c.source_url);
            b.push_bind(source);
        });
        let result = builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn has_bounds(&self) -> Result<bool> {
        Ok(sqlx::query("SELECT 1 FROM bounds LIMIT 1")
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    async fn seed_bounds(&self, centers: &[(f64, f64)]) -> Result<()> {
        for chunk in centers.chunks(100) {
            let mut builder = sqlx::QueryBuilder::new("INSERT IGNORE INTO bounds (lat, lng) ");
            builder.push_values(chunk, |mut b, (lat, lng)| {
                b.push_bind(lat);
                b.push_bind(lng);
            });
            builder.build().execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        Ok(
            sqlx::query(
                "SELECT id, lat, lng FROM bounds WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND items IS NULL ORDER BY id LIMIT 1"
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
            "UPDATE bounds SET in_progress = 1, started_at = NOW() WHERE id = ?",
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
                "UPDATE bounds SET in_progress = 0, started_at = NOW(), items = ?, duplicated = ? WHERE id = ?",
            )
            .bind(items)
            .bind(duplicated),
            None => sqlx::query(
                "UPDATE bounds SET in_progress = 0, started_at = NOW() WHERE id = ?",
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
        sqlx::query("INSERT IGNORE INTO empresite_pages (page) VALUES (?)")
            .bind(page)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn read_empresite_page(&self) -> Result<Option<(i64, u32)>> {
        Ok(
            sqlx::query(
                "SELECT id, page FROM empresite_pages WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND items IS NULL ORDER BY page ASC LIMIT 1",
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .next()
            .map(|row| (row.get("id"), row.get("page"))),
        )
    }

    async fn claim_empresite_page(&self, page_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE empresite_pages SET in_progress = 1, started_at = NOW() WHERE id = ?",
        )
        .bind(page_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn release_empresite_page(
        &self,
        page_id: i64,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        let result = match completed {
            Some((items, duplicated)) => sqlx::query(
                "UPDATE empresite_pages SET in_progress = 0, started_at = NOW(), items = ?, duplicated = ? WHERE id = ?",
            )
            .bind(items)
            .bind(duplicated),
            None => sqlx::query(
                "UPDATE empresite_pages SET in_progress = 0, started_at = NOW() WHERE id = ?",
            ),
        }
        .bind(page_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
