use crate::generator::SPAIN;
use crate::types::{Coincidence, DbConfig, PersistentConfig};
use crate::verboser::Verboser;
use anyhow::Result;
use sqlx::mysql::MySqlQueryResult;
use sqlx::pool::PoolConnection;
use sqlx::{AssertSqlSafe, Executor, MySql, MySqlPool, Row, SqlSafeStr, raw_sql};

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

struct ColumnInfo {
    name: String,
    data_type: String,
}

const NEW_BOUND_COLUMNS: &[(&str, &str)] = &[
    ("in_progress", "TINYINT(1) NOT NULL DEFAULT 0"),
    ("items", "INT DEFAULT NULL"),
    ("duplicated", "INT DEFAULT NULL"),
    ("started_at", "TIMESTAMP NULL DEFAULT NULL"),
];

pub struct DbManager(MySqlPool);

impl DbManager {
    fn quote_identifier(name: &str) -> String {
        format!("`{}`", name.replace('`', "``"))
    }

    pub async fn new(
        config: &DbConfig,
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<Self> {
        verboser.connecting_db();
        let engine_url = format!(
            "mysql://{}:{}@{}:{}",
            config.user, config.password, config.host, config.port
        );

        let pool = MySqlPool::connect(&engine_url).await?;

        let exists = sqlx::query(
            r#" SELECT 1
                FROM INFORMATION_SCHEMA.SCHEMATA
                WHERE SCHEMA_NAME = ?
                LIMIT 1
            "#,
        )
        .bind(&config.database)
        .fetch_optional(&pool)
        .await?
        .is_some();

        let connect = || async {
            let engine_url = format!(
                "mysql://{}:{}@{}:{}/{}",
                config.user, config.password, config.host, config.port, config.database
            );
            Result::<Self>::Ok(Self(MySqlPool::connect(&engine_url).await?))
        };

        Ok(if !exists {
            verboser.creating_db();
            raw_sql(AssertSqlSafe(format!(
                "CREATE DATABASE IF NOT EXISTS {}",
                Self::quote_identifier(&config.database)
            )))
            .execute(&pool)
            .await?;
            let mut this = connect().await?;
            this.create_tables(params, verboser).await?;
            this
        } else {
            let mut this: DbManager = connect().await?;
            this.ensure_tables(config, params, verboser).await?;
            this
        })
    }

    pub async fn connect(&self) -> Result<DbConnection> {
        Ok(DbConnection(self.0.acquire().await?))
    }
}

impl DbUtility for DbManager {
    fn as_executor(&mut self) -> impl Executor<'_, Database = MySql> {
        &self.0
    }
}
pub(crate) trait DbUtility {
    fn as_executor(&mut self) -> impl Executor<'_, Database = MySql>;

    async fn read_bound(&mut self) -> Result<Option<(f32, f32)>> {
        Ok(
            sqlx::query("SELECT lat, lng FROM bounds WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND items IS NULL LIMIT 1")
            .fetch_all(self.as_executor())
            .await?
            .into_iter()
            .next()
            .map(|row| (row.get("lat"), row.get("lng"))))
    }

    async fn claim_bound(&mut self, lat: f32, lng: f32) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE bounds SET in_progress = 1, started_at = NOW() WHERE lat = ? AND lng = ? AND (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR)",
        )
        .bind(lat)
        .bind(lng)
        .execute(self.as_executor())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn release_bound(
        &mut self,
        lat: f32,
        lng: f32,
        completed: Option<(i32, i32)>,
    ) -> Result<bool> {
        let result = match completed {
            Some((items, duplicated)) => sqlx::query(
                "UPDATE bounds SET in_progress = 0, started_at = NOW(), items = ?, duplicated = ? WHERE lat = ? AND lng = ?",
            ).bind(items).bind(duplicated),
            None => sqlx::query(
                "UPDATE bounds SET in_progress = 0, started_at = NOW() WHERE lat = ? AND lng = ?",
            ),
        }         .bind(lat)
        .bind(lng)
        .execute(self.as_executor())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn write_coincidences(
        &mut self,
        data: impl IntoIterator<Item = Coincidence>,
    ) -> Result<MySqlQueryResult> {        
        let filtered: Vec<Coincidence> = data
            .into_iter()
            .filter(|c| c.web.is_some() || c.email.is_some() || c.tfno.is_some())
            .collect();
        if filtered.is_empty() {
            return Ok(MySqlQueryResult::default());
        }        
        let mut builder =
            sqlx::QueryBuilder::new("INSERT IGNORE INTO coincidences (name, web, email, tfno) ");
        builder.push_values(filtered, |mut b, c| {
            b.push_bind(c.name);
            b.push_bind(c.web.unwrap_or_default());
            b.push_bind(c.email.unwrap_or_default());
            b.push_bind(c.tfno.unwrap_or_default());
        });
        Ok(builder.build().execute(self.as_executor()).await?)
    }
}

trait DbUtilityExt: DbUtility{

    async fn get_columns(&mut self, db_name: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let rows = sqlx::query(
            "SELECT COLUMN_NAME, DATA_TYPE, COLUMN_KEY \
         FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
         ORDER BY ORDINAL_POSITION",
        )
        .bind(db_name)
        .bind(table)
        .fetch_all(self.as_executor())
        .await?;

        let cols = rows
            .iter()
            .map(|row| ColumnInfo {
                name: row.get(0),
                data_type: row.get(1),
                //column_key: row.get(2),
            })
            .collect();
        Ok(cols)
    }

    async fn ensure_bound_columns(&mut self, db_name: &str) -> Result<()> {
        let cols = self.get_columns(db_name, "bounds").await?;
        let existing: std::collections::HashSet<String> =
            cols.into_iter().map(|c| c.name).collect();

        for (name, def) in NEW_BOUND_COLUMNS {
            if !existing.contains(*name) {
                self.exec(AssertSqlSafe(format!(
                    "ALTER TABLE bounds ADD COLUMN {} {}",
                    name, def
                )))
                .await?;
            }
        }
        Ok(())
    }

    async fn ensure_tables(
        &mut self,
        config: &DbConfig,
        params: &mut PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.verifying_db();
        let rows = sqlx::query(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES \
         WHERE TABLE_SCHEMA = ?",
        )
        .bind(&config.database)
        .fetch_all(self.as_executor())
        .await?;

        if rows.is_empty() {
            self.create_tables(params, verboser).await?;
            return Ok(());
        }

        let existing: std::collections::HashSet<String> =
            rows.iter().map(|row| row.get(0)).collect();

        let bounds_ok = existing.contains("bounds");
        let coincidences_ok = existing.contains("coincidences");
        let config_ok = existing.contains("configuration_rustcrape");

        if bounds_ok && coincidences_ok && config_ok {
            self.validate_bounds(&config.database).await?;
            self.validate_coincidences(&config.database).await?;
            self.validate_coincidences_unique(&config.database).await?;
            self.ensure_bound_columns(&config.database).await?;

            let config_row = sqlx::query(
                "SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1",
            )
            .fetch_optional(self.as_executor())
            .await?
            .ok_or_else(|| anyhow::anyhow!("configuration_rustcrape table is empty"))?;
            params.search_query = config_row.get("search_query");
            params.zoom = config_row.get("zoom");
            eprintln!(
                "Configuracion cargada: search_query='{}', zoom={}",
                params.search_query, params.zoom
            );
            eprintln!("Esquemas de base de datos validos.");
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
            config.database,
            existing.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    async fn create_tables(
        &mut self,
        params: &PersistentConfig,
        verboser: &impl Verboser,
    ) -> Result<()> {
        verboser.creating_tables();
        self.exec(
            "CREATE TABLE IF NOT EXISTS bounds (
            lat FLOAT NOT NULL,
            lng FLOAT NOT NULL,
            in_progress TINYINT(1) NOT NULL DEFAULT 0,
            items INT DEFAULT NULL,
            duplicated INT DEFAULT NULL,
            started_at TIMESTAMP NULL DEFAULT NULL,
            PRIMARY KEY (lat, lng)
        )",
        )
        .await?;

        self.exec(
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
        .await?;

        self.exec(
            "CREATE TABLE IF NOT EXISTS configuration_rustcrape (
            id INT PRIMARY KEY DEFAULT 1,
            search_query VARCHAR(255) NOT NULL,
            zoom INT UNSIGNED NOT NULL,
            version INT NOT NULL DEFAULT 1,
            CHECK (id = 1)
        )",
        )
        .await?;

        sqlx::query("INSERT INTO configuration_rustcrape (id, search_query, zoom, version) VALUES (1, ?, ?, 1)")
            .bind(&params.search_query)
            .bind(params.zoom as i32)
            .execute(self.as_executor())
            .await?;

        let centers = SPAIN
            .generate_grid(params.zoom, verboser)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        for chunk in centers.chunks(100) {
            let mut builder = sqlx::QueryBuilder::new("INSERT INTO bounds (lat, lng) ");
            builder.push_values(chunk, |mut bounds, (lat, lng)| {
                bounds.push_bind(lat);
                bounds.push_bind(lng);
            });
            builder.build().execute(self.as_executor()).await?;
        }

        eprintln!("Grid generado correctamente.");
        Ok(())
    }

    async fn validate_bounds(&mut self, db_name: &str) -> Result<()> {
        let cols = self.get_columns(db_name, "bounds").await?;
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
    async fn validate_coincidences(&mut self, db_name: &str) -> Result<()> {
        let cols = self.get_columns(db_name, "coincidences").await?;
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

    async fn validate_coincidences_unique(&mut self, db_name: &str) -> Result<()> {
        let rows = sqlx::query(
            "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = 'coincidences' \
             AND CONSTRAINT_NAME = 'uq_datos' ORDER BY ORDINAL_POSITION",
        )
        .bind(db_name)
        .fetch_all(self.as_executor())
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

    async fn exec(&mut self, sql: impl SqlSafeStr) -> Result<MySqlQueryResult> {
        Ok(raw_sql(sql).execute(self.as_executor()).await?)
    }
}

impl<T: DbUtility> DbUtilityExt for T {}

pub struct DbConnection(PoolConnection<MySql>);

impl DbUtility for DbConnection {
    #[inline]
    fn as_executor(&mut self) -> impl Executor<'_, Database = MySql> {
        &mut *self.0
    }
}


#[cfg(test)]
mod tests {
    use crate::verboser::DebugVerboser;

    use super::*;
    use sqlx::Row;

    fn test_config() -> DbConfig {
        DbConfig {
            host: "localhost".to_string(),
            port: 3306,
            user: "biz_user".to_string(),
            password: "biz_pass".to_string(),
            database: "rustcrape".to_string(),
        }
    }

    #[tokio::test]
    async fn test_f32_roundtrip_match() {
        let config_db = test_config();
        let mut config_persistence = PersistentConfig {
            search_query: "test".to_string(),
            zoom: 12,
        };
        let db = DbManager::new(
            &config_db,
            &mut config_persistence,
            &DebugVerboser::default(),
        )
        .await
        .unwrap();
        let mut conn = db.connect().await.unwrap();

        // 1. Read one lat/lng from the database
        let row = sqlx::query("SELECT lat, lng FROM bounds LIMIT 1")
            .fetch_one(&mut *conn.0)
            .await
            .unwrap();
        let lat: f32 = row.get("lat");
        let lng: f32 = row.get("lng");
        println!(
            "Read lat={:?} (bits: {:032b}), lng={:?} (bits: {:032b})",
            lat,
            lat.to_bits(),
            lng,
            lng.to_bits()
        );

        // 2. Match back with f32 bind
        let count_f32: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM bounds WHERE lat = ? AND lng = ?")
                .bind(lat)
                .bind(lng)
                .fetch_one(&mut *conn.0)
                .await
                .unwrap();
        println!("f32 bind COUNT: {}", count_f32);

        // 3. Match back with f64 bind
        let count_f64: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM bounds WHERE lat = ? AND lng = ?")
                .bind(lat as f64)
                .bind(lng as f64)
                .fetch_one(&mut *conn.0)
                .await
                .unwrap();
        println!("f64 bind COUNT: {}", count_f64);

        // 4. CAST columns to DOUBLE + f64 bind
        let count_cast: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM bounds WHERE CAST(lat AS DOUBLE) = ? AND CAST(lng AS DOUBLE) = ?",
        )
        .bind(lat as f64)
        .bind(lng as f64)
        .fetch_one(&mut *conn.0)
        .await
        .unwrap();
        println!("CAST+ f64 COUNT: {}", count_cast);

        // 5. claim_bound and release_bound to verify full cycle
        let claimed = conn.claim_bound(lat, lng).await.unwrap();
        println!("claim_bound: {}", claimed);
        if claimed {
            let released = conn.release_bound(lat, lng, Some((5, 2))).await.unwrap();
            println!("release_bound: {}", released);
        }

        eprintln!(
            "f32={} f64={} cast+={} claim={}",
            count_f32, count_f64, count_cast, claimed
        );
    }
}
