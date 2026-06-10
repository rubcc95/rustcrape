use crate::config::{AppConfig, DbType};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resultado {
    pub id: Option<i64>,
    pub nombre: String,
    pub email: String,
    pub web: String,
    pub tfno: String, 
    pub maps: String,
}

pub enum DbConn {
    Sqlite(Connection),
    Mysql(MySqlPool),
}

pub struct Database {
    pub conn: Arc<Mutex<DbConn>>,
}

impl Database {
    pub async fn new(config: &AppConfig) -> Result<Self, String> {
        match config.db_type {
            DbType::Sqlite => {
                let path = &config.sqlite_path;
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let conn = Connection::open(path).map_err(|e| e.to_string())?;
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS cuadrantes (
                        lat DECIMAL(10,6) NOT NULL,
                        lng DECIMAL(10,6) NOT NULL,
                        PRIMARY KEY (lat, lng)
                    );
                    CREATE TABLE IF NOT EXISTS resultados (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        nombre TEXT NOT NULL,
                        email TEXT DEFAULT '',
                        web TEXT DEFAULT '',
                        tfno TEXT DEFAULT '',
                        maps TEXT DEFAULT '',
                        creado TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    );",
                )
                .map_err(|e| e.to_string())?;
                Ok(Self {
                    conn: Arc::new(Mutex::new(DbConn::Sqlite(conn))),
                })
            }
            DbType::Mysql => {
                let url = format!(
                    "mysql://{}:{}@{}:{}/{}",
                    config.mysql_user,
                    config.mysql_password,
                    config.mysql_host,
                    config.mysql_port,
                    config.mysql_database
                );
                let pool = MySqlPoolOptions::new()
                    .max_connections(5)
                    .connect(&url)
                    .await
                    .map_err(|e| e.to_string())?;
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS cuadrantes (
                        lat DECIMAL(10,6) NOT NULL,
                        lng DECIMAL(10,6) NOT NULL,
                        PRIMARY KEY (lat, lng)
                    )",
                )
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS resultados (
                        id INT AUTO_INCREMENT PRIMARY KEY,
                        nombre TEXT NOT NULL,
                        email TEXT DEFAULT '',
                        web TEXT DEFAULT '',
                        tfno TEXT DEFAULT '',
                        maps TEXT DEFAULT '',
                        creado TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )",
                )
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
                Ok(Self {
                    conn: Arc::new(Mutex::new(DbConn::Mysql(pool))),
                })
            }
        }
    }

    pub async fn get_cuadrante_count(&self) -> Result<u64, String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                let count: i64 = c
                    .query_row("SELECT COUNT(*) FROM cuadrantes", [], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                Ok(count as u64)
            }
            DbConn::Mysql(p) => {
                let row: (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM cuadrantes")
                        .fetch_one(p)
                        .await
                        .map_err(|e| e.to_string())?;
                Ok(row.0 as u64)
            }
        }
    }

    pub async fn get_resultado_count(&self) -> Result<u64, String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                let count: i64 = c
                    .query_row("SELECT COUNT(*) FROM resultados", [], |r| r.get(0))
                    .map_err(|e| e.to_string())?;
                Ok(count as u64)
            }
            DbConn::Mysql(p) => {
                let row: (i64,) =
                    sqlx::query_as("SELECT COUNT(*) FROM resultados")
                        .fetch_one(p)
                        .await
                        .map_err(|e| e.to_string())?;
                Ok(row.0 as u64)
            }
        }
    }

    pub async fn get_cuadrante(&self) -> Result<Option<(f64, f64)>, String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                let result = c
                    .query_row(
                        "SELECT lat, lng FROM cuadrantes LIMIT 1",
                        [],
                        |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?)),
                    )
                    .ok();
                Ok(result)
            }
            DbConn::Mysql(p) => {
                let result = sqlx::query_as::<_, (f64, f64)>(
                    "SELECT lat, lng FROM cuadrantes LIMIT 1",
                )
                .fetch_optional(p)
                .await
                .map_err(|e| e.to_string())?;
                Ok(result)
            }
        }
    }

    pub async fn delete_cuadrante(&self, lat: f64, lng: f64) -> Result<(), String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                c.execute("DELETE FROM cuadrantes WHERE lat = ?1 AND lng = ?2", rusqlite::params![lat, lng])
                    .map_err(|e| e.to_string())?;
            }
            DbConn::Mysql(p) => {
                sqlx::query("DELETE FROM cuadrantes WHERE lat = ? AND lng = ?")
                    .bind(lat)
                    .bind(lng)
                    .execute(p)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub async fn insert_resultado(&self, r: &Resultado) -> Result<(), String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                c.execute(
                    "INSERT INTO resultados (nombre, email, web, tfno, maps) VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![r.nombre, r.email, r.web, r.tfno, r.maps],
                )
                .map_err(|e| e.to_string())?;
            }
            DbConn::Mysql(p) => {
                sqlx::query(
                    "INSERT INTO resultados (nombre, email, web, tfno, maps) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&r.nombre)
                .bind(&r.email)
                .bind(&r.web)
                .bind(&r.tfno)
                .bind(&r.maps)
                .execute(p)
                .await
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub async fn reinsert_cuadrante(&self, lat: f64, lng: f64) -> Result<(), String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                c.execute(
                    "INSERT OR IGNORE INTO cuadrantes (lat, lng) VALUES (?1, ?2)",
                    rusqlite::params![lat, lng],
                )
                .map_err(|e| e.to_string())?;
            }
            DbConn::Mysql(p) => {
                sqlx::query(
                    "INSERT IGNORE INTO cuadrantes (lat, lng) VALUES (?, ?)",
                )
                .bind(lat)
                .bind(lng)
                .execute(p)
                .await
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub async fn insert_cuadrantes(&self, points: &[(f64, f64)]) -> Result<(), String> {
        let conn = self.conn.lock().await;
        match &*conn {
            DbConn::Sqlite(c) => {
                let tx = c.unchecked_transaction().map_err(|e| e.to_string())?;
                for (lat, lng) in points {
                    tx.execute(
                        "INSERT OR IGNORE INTO cuadrantes (lat, lng) VALUES (?1, ?2)",
                        rusqlite::params![lat, lng],
                    )
                    .map_err(|e| e.to_string())?;
                }
                tx.commit().map_err(|e| e.to_string())?;
            }
            DbConn::Mysql(p) => {
                let mut tx = p.begin().await.map_err(|e| e.to_string())?;
                for (lat, lng) in points {
                    sqlx::query("INSERT IGNORE INTO cuadrantes (lat, lng) VALUES (?, ?)")
                        .bind(lat)
                        .bind(lng)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                tx.commit().await.map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub async fn get_resultados(&self, page: u64, page_size: u64) -> Result<Vec<Resultado>, String> {
        let conn = self.conn.lock().await;
        let offset = page * page_size;
        match &*conn {
            DbConn::Sqlite(c) => {
                let mut stmt = c
                    .prepare("SELECT id, nombre, email, web, tfno, maps FROM resultados ORDER BY id DESC LIMIT ?1 OFFSET ?2")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(rusqlite::params![page_size as i64, offset as i64], |r| {
                        Ok(Resultado {
                            id: Some(r.get(0)?),
                            nombre: r.get(1)?,
                            email: r.get(2)?,
                            web: r.get(3)?,
                            tfno: r.get(4)?,
                            maps: r.get(5)?,
                        })
                    })
                    .map_err(|e| e.to_string())?;
                let mut resultados = Vec::new();
                for row in rows {
                    resultados.push(row.map_err(|e| e.to_string())?);
                }
                Ok(resultados)
            }
            DbConn::Mysql(p) => {
                let rows = sqlx::query_as::<_, (i64, String, String, String, String, String)>(
                    "SELECT id, nombre, email, web, tfno, maps FROM resultados ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(page_size as i64)
                .bind(offset as i64)
                .fetch_all(p)
                .await
                .map_err(|e| e.to_string())?;
                Ok(rows
                    .into_iter()
                    .map(|(id, nombre, email, web, tfno, maps)| Resultado {
                        id: Some(id),
                        nombre,
                        email,
                        web,
                        tfno,
                        maps,
                    })
                    .collect())
            }
        }
    }
}
