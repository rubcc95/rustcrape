use crate::storage::{Persistence, WriteOutcome};
use crate::types::{
    Coincidence, CoincidenceColumn, CoincidencePage, CoincidenceRecord, DbConfig, GMapsConfig,
    ProjectStats, SortOrder,
};
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

// Tablas imprescindibles para reconocer la base de datos como de rustcrape.
// `bounds` (Google Maps) y `empresite_pages` (Empresite) se crean bajo demanda
// segun el target que se vaya a usar.
const REQUIRED_TABLES: &[&str] = &["coincidences", "configuration_rustcrape"];

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
        params: &mut GMapsConfig,
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

        const NEW_COINCIDENCE_COLUMNS: &[(&str, &str)] = &[
            ("legal_name", "VARCHAR(255) DEFAULT NULL"),
            ("tax_id", "VARCHAR(20) DEFAULT NULL"),
            ("legal_form", "VARCHAR(100) DEFAULT NULL"),
            ("sector", "VARCHAR(100) DEFAULT NULL"),
            ("incorporation_date", "VARCHAR(20) DEFAULT NULL"),
            ("last_change_date", "VARCHAR(20) DEFAULT NULL"),
            ("corporate_purpose", "TEXT"),
            ("activity", "VARCHAR(255) DEFAULT NULL"),
            ("cnae_activity", "VARCHAR(255) DEFAULT NULL"),
            ("company_status", "VARCHAR(50) DEFAULT NULL"),
        ];

        for (name, def) in NEW_COINCIDENCE_COLUMNS {
            if !existing.contains(*name) {
                sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                    "ALTER TABLE coincidences ADD COLUMN {} {}",
                    name, def
                )))
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn create_tables(&self, params: &GMapsConfig, verboser: &impl Verboser) -> Result<()> {
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
                legal_name VARCHAR(255) DEFAULT NULL,
                tax_id VARCHAR(20) DEFAULT NULL,
                legal_form VARCHAR(100) DEFAULT NULL,
                sector VARCHAR(100) DEFAULT NULL,
                incorporation_date VARCHAR(20) DEFAULT NULL,
                last_change_date VARCHAR(20) DEFAULT NULL,
                corporate_purpose TEXT,
                activity VARCHAR(255) DEFAULT NULL,
                cnae_activity VARCHAR(255) DEFAULT NULL,
                company_status VARCHAR(50) DEFAULT NULL,
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
        params: &mut GMapsConfig,
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

            let config_row =
                sqlx::query("SELECT search_query, zoom FROM configuration_rustcrape WHERE id = 1")
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

    /// Numero de coincidencias almacenadas que aportan telefono.
    async fn count_phones(&self) -> Result<i64> {
        let row = sqlx::query(
            "SELECT CAST(COUNT(*) AS SIGNED) AS total FROM coincidences \
             WHERE tfno IS NOT NULL AND tfno <> ''",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("total"))
    }
}

impl Persistence for MysqlPersistence {
    async fn write_coincidences(
        &self,
        source: &str,
        data: Vec<Coincidence>,
    ) -> Result<WriteOutcome> {
        let filtered: Vec<Coincidence> = data.into_iter().filter(|c| c.has_any_data()).collect();
        if filtered.is_empty() {
            return Ok(WriteOutcome::default());
        }
        let phones_before = self.count_phones().await?;
        let result = sqlx::QueryBuilder::new(
            "INSERT IGNORE INTO coincidences (name, web, email, tfno, source_url, source, \
             legal_name, tax_id, legal_form, sector, incorporation_date, last_change_date, \
             corporate_purpose, activity, cnae_activity, company_status) ",
        )
        .push_values(filtered, |mut b, c| {
            b.push_bind(c.name)
                .push_bind(c.web.unwrap_or_default())
                .push_bind(c.email.unwrap_or_default())
                .push_bind(c.tfno.unwrap_or_default())
                .push_bind(c.source_url)
                .push_bind(source)
                .push_bind(c.legal_name)
                .push_bind(c.tax_id)
                .push_bind(c.legal_form)
                .push_bind(c.sector)
                .push_bind(c.incorporation_date)
                .push_bind(c.last_change_date)
                .push_bind(c.corporate_purpose)
                .push_bind(c.activity)
                .push_bind(c.cnae_activity)
                .push_bind(c.company_status);
        })
        .build()
        .execute(&self.pool)
        .await?;
        let inserted = result.rows_affected();
        let phones_after = self.count_phones().await?;
        Ok(WriteOutcome {
            inserted,
            inserted_with_phone: (phones_after - phones_before).max(0) as u64,
        })
    }

    async fn stats(&self) -> Result<ProjectStats> {
        let bounds = sqlx::query(
            "SELECT CAST(COUNT(*) AS SIGNED) AS total, \
             CAST(COALESCE(SUM(items IS NOT NULL), 0) AS SIGNED) AS done FROM bounds",
        )
        .fetch_one(&self.pool)
        .await?;
        let pages = sqlx::query(
            "SELECT CAST(COUNT(*) AS SIGNED) AS total, \
             CAST(COALESCE(SUM(items IS NOT NULL), 0) AS SIGNED) AS done FROM empresite_pages",
        )
        .fetch_one(&self.pool)
        .await?;
        let coincidences = sqlx::query(
            "SELECT CAST(COUNT(*) AS SIGNED) AS total, \
             CAST(COALESCE(SUM(CASE WHEN tfno IS NOT NULL AND tfno <> '' THEN 1 ELSE 0 END), 0) AS SIGNED) AS phones \
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

    async fn list_coincidences(
        &self,
        column: CoincidenceColumn,
        order: SortOrder,
        limit: u32,
        offset: u32,
    ) -> Result<CoincidencePage> {
        let total: i64 = sqlx::query("SELECT CAST(COUNT(*) AS SIGNED) AS total FROM coincidences")
            .fetch_one(&self.pool)
            .await?
            .get("total");

        // `column_name()` y `sql()` provienen de enums, nunca de entrada del
        // usuario: el fragmento ORDER BY es seguro frente a inyeccion SQL.

        let rows = sqlx::QueryBuilder::new(
            "SELECT id, name, web, email, tfno, source_url, source, \
             CAST(creado AS CHAR) AS creado, legal_name, tax_id, legal_form, sector, \
             incorporation_date, last_change_date, corporate_purpose, activity, \
             cnae_activity, company_status \
             FROM coincidences ORDER BY ",
        )
        .push(column.column_name())
        .push(" ")
        .push(order.sql())
        .push(" LIMIT ")
        .push_bind(limit as i64)
        .push(" OFFSET ")
        .push_bind(offset as i64)
        .build()
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| CoincidenceRecord {
            id: row.get("id"),
            name: row.get::<Option<String>, _>("name").unwrap_or_default(),
            web: row.get::<Option<String>, _>("web").unwrap_or_default(),
            email: row.get::<Option<String>, _>("email").unwrap_or_default(),
            tfno: row.get::<Option<String>, _>("tfno").unwrap_or_default(),
            source_url: row
                .get::<Option<String>, _>("source_url")
                .unwrap_or_default(),
            source: row.get::<Option<String>, _>("source").unwrap_or_default(),
            creado: row.get::<Option<String>, _>("creado"),
            legal_name: row.get::<Option<String>, _>("legal_name"),
            tax_id: row.get::<Option<String>, _>("tax_id"),
            legal_form: row.get::<Option<String>, _>("legal_form"),
            sector: row.get::<Option<String>, _>("sector"),
            incorporation_date: row.get::<Option<String>, _>("incorporation_date"),
            last_change_date: row.get::<Option<String>, _>("last_change_date"),
            corporate_purpose: row.get::<Option<String>, _>("corporate_purpose"),
            activity: row.get::<Option<String>, _>("activity"),
            cnae_activity: row.get::<Option<String>, _>("cnae_activity"),
            company_status: row.get::<Option<String>, _>("company_status"),
        })
        .collect();

        Ok(CoincidencePage {
            rows,
            total: total as u64,
        })
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

    // async fn read_bound(&self) -> Result<Option<(i64, f32, f32)>> {
    //     Ok(
    //         sqlx::query(
    //             "SELECT id, lat, lng FROM bounds WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) AND items IS NULL ORDER BY id LIMIT 1"
    //         )
    //         .fetch_all(&self.pool)
    //         .await?
    //         .into_iter()
    //         .next()
    //         .map(|row| (row.get("id"), row.get("lat"), row.get("lng"))),
    //     )
    // }

    async fn claim_bound(&self) -> Result<Option<(i64, f32, f32)>> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
                SELECT id, lat, lng 
                FROM bounds 
                WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) 
                    AND items IS NULL 
                ORDER BY id 
                LIMIT 1
                FOR UPDATE
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        Ok(match row {
            Some(row) => {
                let id: i64 = row.get("id");
                let lat: f32 = row.get("lat");
                let lng: f32 = row.get("lng");
                sqlx::query(
                    r#"
                        UPDATE bounds 
                        SET in_progress = 1, started_at = NOW() 
                        WHERE id = ?
                    "#,
                )
                .bind(id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Some((id, lat, lng))
            }
            None => {
                tx.rollback().await?;
                None
            }
        })
    }

    async fn release_bound(&self, bound_id: i64, completed: Option<(i32, i32)>) -> Result<bool> {
        let result = match completed {
            Some((items, duplicated)) => sqlx::query(
                r#"
                    UPDATE bounds 
                    SET in_progress = 0, started_at = NOW(), items = ?, duplicated = ? 
                    WHERE id = ?
                "#,
            )
            .bind(items)
            .bind(duplicated),
            None => sqlx::query(
                r#"
                    UPDATE bounds 
                    SET in_progress = 0, started_at = NOW() 
                    WHERE id = ?
                "#,
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

    async fn claim_empresite_page(&self) -> Result<Option<(i64, u32)>> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query(
            r#"
                SELECT id, PAGE 
                FROM empresite_pages 
                WHERE (in_progress = 0 OR started_at < NOW() - INTERVAL 2 HOUR) 
                    AND items IS NULL 
                LIMIT 1
                FOR UPDATE
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?;

        Ok(match row {
            Some(row) => {
                let id: i64 = row.get("id");
                let page: u32 = row.get("page");
                sqlx::query(
                    r#"
                        UPDATE empresite_pages 
                        SET in_progress = 1, started_at = NOW() 
                        WHERE id = ?                        
                    "#,
                )
                .bind(id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                Some((id, page))
            }
            None => {
                tx.rollback().await?;
                None
            }
        })
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
