use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::types::RuntimeError;

#[derive(Debug)]
pub struct RuntimeDatabase {
    path: PathBuf,
}

impl RuntimeDatabase {
    pub fn new_in_memory() -> Result<Self, RuntimeError> {
        let path = std::env::temp_dir().join(format!("mathi-runtime-{}.sqlite3", Uuid::new_v4()));
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        }
        let database = Self { path };
        database.initialize()?;
        Ok(database)
    }

    fn initialize(&self) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                CREATE TABLE IF NOT EXISTS telemetry_samples (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    duration_ms INTEGER NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
                );
                CREATE TABLE IF NOT EXISTS session_state (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    key TEXT NOT NULL UNIQUE,
                    value TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
                );
                "#,
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn record_sample(&self, name: &str, duration_ms: u64) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO telemetry_samples (name, duration_ms) VALUES (?1, ?2)",
                params![name, duration_ms as i64],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn save_session_state(&self, key: &str, value: &str) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO session_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn telemetry_count(&self) -> Result<u64, RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        let count = connection
            .query_row("SELECT COUNT(*) FROM telemetry_samples", [], |row| row.get::<_, i64>(0))
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(count as u64)
    }
}
