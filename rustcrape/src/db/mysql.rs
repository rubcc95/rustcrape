use crate::generator::SPAIN;
use crate::storage::Persistence;
use crate::types::{Coincidence, DbConfig, PersistentConfig, ProjectStats};
use crate::verboser::Verboser;
use anyhow::Result;
use sqlx::{MySqlPool, Row};

const NUMERIC_TYPES: &[&str] = &[
    "decimal",
    "double",
    "float",
    "int",
    "bigint",
    "smallint",
    "tinyint",
    "mediumint",
    "dec",
    "fixed",
    "numeric",
    "real",
];

const NEW_BOUND_COLUMNS: &[(&str, &str)] = &[
    ("in_progress", "TINYINT(1) NOT NULL DEFAULT 0"),
    ("items", "INT DEFAULT NULL"),
    ("duplicated", "INT DEFAULT NULL"),
    ("started_at", "TIMESTAMP NULL DEFAULT NULL"),
];

struct ColumnInfo {
    name: String,
    data_type: String,
}

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
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        let (host, port, user, password, database) = match config {
            DbConfig::Mysql {
                host,
                port,
                user,
                password,
                database,
            } => (
                host.clone(),
                *port,
                user.clone(),
                password.clone(),
                database.clone(),
            ),
            _ => unreachable!(),
        };

        verboser.connecting_db();

        // 1. Connect to engine (no specific database)
        let engine_url = format!("mysql://{}:{}@{}:{}", user, password, host, port);
        let admin_pool = MySqlPool::connect(&engine_url).await?;

        // 2. Check if database exists
        let exists =
            sqlx::query("SELECT 1 FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = ? LIMIT 1")
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
        let db_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            user, password, host, port, database
        );
        let pool = MySqlPool::connect(&db_url).await?;
        drop(admin_pool);

        let this = Self {
            pool,
            db_name: database,
            host,
            port,
            user,
            password,
        };

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

    async fn create_tables(
        &self,
        params: &PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.creating_tables();
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
            "CREATE TABLE IF NOT EXISTS coincidences (
                id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                web VARCHAR(255) DEFAULT '',
                email VARCHAR(255) DEFAULT '',
                tfno VARCHAR(50) DEFAULT '',
                maps TEXT DEFAULT '',
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

        let centers = SPAIN
            .generate_grid(params.zoom, verboser)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        for chunk in centers.chunks(100) {
            let mut builder = sqlx::QueryBuilder::new("INSERT IGNORE INTO bounds (lat, lng) ");
            builder.push_values(chunk, |mut b, (lat, lng)| {
                b.push_bind(lat);
                b.push_bind(lng);
            });
            builder.build().execute(&self.pool).await?;
        }

        verboser.generating_bounds(centers.len(), centers.len(), centers.len());
        Ok(())
    }

    async fn ensure_tables(
        &self,
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.verifying_db();
        let rows =
            sqlx::query("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = ?")
                .bind(&self.db_name)
                .fetch_all(&self.pool)
                .await?;

        if rows.is_empty() {
            return self.create_tables(params, verboser).await;
        }

        let existing: std::collections::HashSet<String> =
            rows.iter().map(|row| row.get(0)).collect();

        let bounds_ok = existing.contains("bounds");
        let coincidences_ok = existing.contains("coincidences");
        let config_ok = existing.contains("configuration_rustcrape");

        if bounds_ok && coincidences_ok && config_ok {
            self.validate_bounds().await?;
            self.validate_coincidences().await?;
            self.validate_coincidences_unique().await?;
            self.ensure_bound_columns().await?;

            let config_row =
                sqlx::query("SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1")
                    .fetch_optional(&self.pool)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("configuration_rustcrape table is empty"))?;
            params.search_query = config_row.get("search_query");
            params.zoom = config_row.get("zoom");
            return Ok(());
        }

        if bounds_ok || coincidences_ok || config_ok {
            let mut present = Vec::new();
            let mut missing = Vec::new();
            for (name, ok) in [
                ("bounds", bounds_ok),
                ("coincidences", coincidences_ok),
                ("configuration_rustcrape", config_ok),
            ] {
                if ok {
                    present.push(name);
                } else {
                    missing.push(name);
                }
            }
            anyhow::bail!(
                "Tablas existentes: {}. Tablas faltantes: {}. Deben existir todas o ninguna.",
                present.join(", "),
                missing.join(", ")
            );
        }

        anyhow::bail!(
            "La base de datos '{}' contiene tablas inesperadas ({}). Se esperaba una base de datos vacía o con las tablas rustcrape.",
            self.db_name,
            existing.iter().cloned().collect::<Vec<_>>().join(", ")
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
        let required = ["name", "email", "web", "tfno", "maps"];
        let names: std::collections::HashSet<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        let missing: Vec<&str> = required
            .iter()
            .filter(|n| !names.contains(*n))
            .copied()
            .collect();
        if !missing.is_empty() {
            anyhow::bail!(
                "Table coincidences: missing columns: {}",
                missing.join(", ")
            );
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
    async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        Ok(
            sqlx::query(
                "SELECT id, lat, lng FROM bounds WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND items IS NULL LIMIT 1"
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .next()
            .map(|row| (row.get("id"), row.get("lat"), row.get("lng"))),
        )
    }

    async fn claim_bound(&self, bound_id: i64) -> Result<bool> {
        let result =
            sqlx::query("UPDATE bounds SET in_progress = 1, started_at = NOW() WHERE id = ?")
                .bind(bound_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn release_bound(&self, bound_id: i64, completed: Option<(i32, i32)>) -> Result<bool> {
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

    async fn write_coincidences(&self, data: Vec<Coincidence>) -> Result<(u64, u64)> {
        let filtered: Vec<Coincidence> = data
            .into_iter()
            .filter(|c| c.web.is_some() || c.email.is_some() || c.tfno.is_some())
            .collect();
        if filtered.is_empty() {
            return Ok((0, 0));
        }
        let mut builder =
            sqlx::QueryBuilder::new("INSERT IGNORE INTO coincidences (name, web, email, tfno) ");
        builder.push_values(filtered, |mut b, c| {
            b.push_bind(c.name);
            b.push_bind(c.web.unwrap_or_default());
            b.push_bind(c.email.unwrap_or_default());
            b.push_bind(c.tfno.unwrap_or_default());
        });
        let phones: u64 = sqlx::QueryBuilder::new(
            "SELECT COUNT(DISTINCT tfno) FROM coincidences WHERE tfno IS NOT NULL",
        )
        .build()
        .fetch_one(&self.pool)
        .await?
        .get(0);
        let result = builder.build().execute(&self.pool).await?;
        Ok((result.rows_affected(), phones))
    }
}

pub async fn fetch_stats(config: &DbConfig) -> Result<ProjectStats> {
    let (host, port, user, password, database) = match config {
        DbConfig::Mysql {
            host,
            port,
            user,
            password,
            database,
        } => (
            host.clone(),
            *port,
            user.clone(),
            password.clone(),
            database.clone(),
        ),
        _ => unreachable!(),
    };

    let db_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        user, password, host, port, database
    );
    let pool = MySqlPool::connect(&db_url).await?;

    let bounds_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bounds")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let bounds_processed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bounds WHERE items IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
    let results_found: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM coincidences")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
    let phones_found: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT tfno) FROM coincidences WHERE tfno IS NOT NULL AND tfno != ''",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    pool.close().await;

    Ok(ProjectStats {
        bounds_total,
        bounds_processed,
        bounds_remaining: bounds_total - bounds_processed,
        results_found,
        phones_found,
    })
}
