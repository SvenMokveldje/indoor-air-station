use std::{io::Read, path::Path, sync::{Arc, Mutex}};
use rusqlite::{Connection, params, OptionalExtension};
use shared::Reading;
use chrono::Utc;

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS readings (
        seq INTEGER PRIMARY KEY,
        temperature_c REAL,
        humidity_pct REAL,
        pressure_hpa REAL,
        co2_ppm INTEGER,
        recorded_at TEXT NOT NULL
    );
    ";

#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
       Self::from_connection(Connection::open(path)?) 
    }

    pub fn open_from_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub async fn insert_reading(&self, r: &Reading) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now: String = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let reading = r.clone();
        
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("mutex poisoned");
            conn.execute(
               "INSERT INTO readings (seq, temperature_c, humidity_pct, pressure_hpa, co2_ppm, recorded_at)
               VALUES (?, ?, ?, ?, ?, ?)",
               params![reading.seq, reading.temperature_c, reading.humidity_pct, reading.pressure_hpa, reading.co2_ppm, now],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await??;
        Ok(())
    }

    pub async fn get_reading(&self, seq: u32) -> Result<Option<Reading>, Box<dyn std::error::Error + Send + Sync>> {
        let conn = Arc::clone(&self.conn);
        let result: Option<Reading> = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().expect("mutex poisoned");
            conn.query_row("SELECT * FROM readings WHERE seq = ?", 
                [seq],
                |row| {
                    Ok(Reading {
                        seq: row.get(0)?,
                        temperature_c: row.get(1)?,
                        humidity_pct: row.get(2)?,
                        pressure_hpa: row.get(3)?,
                        co2_ppm: row.get(4)?,
                    })
                }
            )
            .optional()
        })
        .await??;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_database() -> Database {
        Database::open_from_memory().expect("in memory db")
    } 
    
    #[tokio::test]
    async fn insert_items_can_be_read_back() {
        let db = new_database();

        let reading = Reading {
            seq: 1,
            temperature_c: 25.0,
            humidity_pct: 50.0,
            pressure_hpa: 1013.0,
            co2_ppm: Some(400),
        };
        db.insert_reading(&reading).await.unwrap();
        let result = db.get_reading(reading.seq).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), reading);
    }

    #[tokio::test]
    async fn insert_item_with_same_seq_errors() {
        let db = new_database();

        let reading = Reading {
            seq: 1,
            temperature_c: 25.0,
            humidity_pct: 50.0,
            pressure_hpa: 1013.0,
            co2_ppm: Some(400),
        };
        db.insert_reading(&reading).await.unwrap();

        let reading2 = Reading {
            seq: 1,
            temperature_c: 26.0,
            humidity_pct: 51.0,
            pressure_hpa: 1014.0,
            co2_ppm: Some(401),
        };
        assert!(db.insert_reading(&reading2).await.is_err());
        let result = db.get_reading(1).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), reading);
    }

    #[tokio::test]
    async fn get_item_which_does_not_exist_returns_none() {
        let db = new_database();

        let result = db.get_reading(1).await.unwrap();
        assert!(result.is_none());
    }
}