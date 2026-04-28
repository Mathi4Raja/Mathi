use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::types::RuntimeError;

#[derive(Debug, Clone)]
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
                CREATE TABLE IF NOT EXISTS secrets_vault (
                    key TEXT PRIMARY KEY,
                    cipher_text TEXT NOT NULL,
                    nonce TEXT NOT NULL,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
                );
                CREATE TABLE IF NOT EXISTS memory_entries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    scope TEXT NOT NULL,
                    memory_key TEXT NOT NULL,
                    value TEXT NOT NULL,
                    redacted_value TEXT NOT NULL,
                    expires_at INTEGER,
                    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                    UNIQUE(scope, memory_key)
                );
                CREATE TABLE IF NOT EXISTS policy_audit (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    action TEXT NOT NULL,
                    agent_id TEXT NOT NULL,
                    outcome TEXT NOT NULL,
                    details TEXT NOT NULL,
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

    pub fn record_policy_audit(
        &self,
        action: &str,
        agent_id: &str,
        outcome: &str,
        details: &str,
    ) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO policy_audit (action, agent_id, outcome, details) VALUES (?1, ?2, ?3, ?4)",
                params![action, agent_id, outcome, details],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn policy_audit_count(&self) -> Result<u64, RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        let count = connection
            .query_row("SELECT COUNT(*) FROM policy_audit", [], |row| row.get::<_, i64>(0))
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(count as u64)
    }

    pub fn save_secret(&self, key: &str, cipher_text: &str, nonce: &str) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO secrets_vault (key, cipher_text, nonce) VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET cipher_text = excluded.cipher_text, nonce = excluded.nonce, updated_at = strftime('%s', 'now')",
                params![key, cipher_text, nonce],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn load_secret(&self, key: &str) -> Result<Option<(String, String)>, RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        let result = connection
            .query_row(
                "SELECT cipher_text, nonce FROM secrets_vault WHERE key = ?1",
                params![key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(result)
    }

    pub fn delete_secret(&self, key: &str) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute("DELETE FROM secrets_vault WHERE key = ?1", params![key])
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn save_memory_entry(
        &self,
        scope: &str,
        memory_key: &str,
        value: &str,
        redacted_value: &str,
        expires_at: Option<i64>,
    ) -> Result<(), RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        connection
            .execute(
                "INSERT INTO memory_entries (scope, memory_key, value, redacted_value, expires_at) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(scope, memory_key) DO UPDATE SET value = excluded.value, redacted_value = excluded.redacted_value, expires_at = excluded.expires_at",
                params![scope, memory_key, value, redacted_value, expires_at],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(())
    }

    pub fn load_memory_entry(
        &self,
        scope: &str,
        memory_key: &str,
        now_epoch_secs: i64,
    ) -> Result<Option<(String, String, Option<i64>)>, RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        let result = connection
            .query_row(
                "SELECT value, redacted_value, expires_at FROM memory_entries WHERE scope = ?1 AND memory_key = ?2 AND (expires_at IS NULL OR expires_at > ?3)",
                params![scope, memory_key, now_epoch_secs],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(result)
    }

    pub fn purge_expired_memory(&self, now_epoch_secs: i64) -> Result<u64, RuntimeError> {
        let connection = Connection::open(&self.path)
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        let deleted = connection
            .execute(
                "DELETE FROM memory_entries WHERE expires_at IS NOT NULL AND expires_at <= ?1",
                params![now_epoch_secs],
            )
            .map_err(|error| RuntimeError::BootstrapFailed(error.to_string()))?;
        Ok(deleted as u64)
    }
}
