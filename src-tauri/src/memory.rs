use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::RuntimeDatabase;
use crate::redaction::Redactor;
use crate::types::RuntimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Session,
    Persistent,
    Workspace,
}

impl MemoryScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryScope::Session => "session",
            MemoryScope::Persistent => "persistent",
            MemoryScope::Workspace => "workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub value: String,
    pub redacted_value: String,
    pub expires_at_epoch: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MemoryService {
    db: RuntimeDatabase,
    redactor: Redactor,
}

impl MemoryService {
    pub fn new_in_memory() -> Result<Self, RuntimeError> {
        Ok(Self {
            db: RuntimeDatabase::new_in_memory()?,
            redactor: Redactor::default(),
        })
    }

    pub fn with_database(db: RuntimeDatabase) -> Self {
        Self {
            db,
            redactor: Redactor::default(),
        }
    }

    pub fn put(
        &self,
        scope: MemoryScope,
        memory_key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), RuntimeError> {
        let now = now_epoch();
        let expires_at = ttl_seconds.map(|ttl| now + ttl as i64);
        let redacted = self.redactor.redact_text(value);
        self.db
            .save_memory_entry(scope.as_str(), memory_key, value, &redacted, expires_at)
    }

    pub fn get(&self, scope: MemoryScope, memory_key: &str) -> Result<Option<MemoryEntry>, RuntimeError> {
        let now = now_epoch();
        let loaded = self.db.load_memory_entry(scope.as_str(), memory_key, now)?;
        Ok(loaded.map(|(value, redacted_value, expires_at_epoch)| MemoryEntry {
            value,
            redacted_value,
            expires_at_epoch,
        }))
    }

    pub fn cleanup_expired(&self) -> Result<u64, RuntimeError> {
        self.db.purge_expired_memory(now_epoch())
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
