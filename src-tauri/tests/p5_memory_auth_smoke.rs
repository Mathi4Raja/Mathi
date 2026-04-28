use mathi_runtime::auth::LocalVault;
use mathi_runtime::memory::{MemoryScope, MemoryService};
use mathi_runtime::redaction::Redactor;

#[test]
fn vault_encrypts_roundtrip_and_revoke_removes_secret() {
    let vault = LocalVault::new_in_memory("test-passphrase").expect("vault");
    vault
        .store_secret("provider/openrouter", "sk-live-123456789")
        .expect("store secret");

    let loaded = vault
        .load_secret("provider/openrouter")
        .expect("load secret");
    assert_eq!(loaded, "sk-live-123456789");

    vault
        .revoke_secret("provider/openrouter")
        .expect("revoke secret");
    assert!(vault.load_secret("provider/openrouter").is_err());
}

#[test]
fn redactor_masks_common_sensitive_patterns() {
    let redactor = Redactor::default();
    let input = "Authorization: Bearer abc123token user=test@example.com api_key=supersecret123";
    let redacted = redactor.redact_text(input);

    assert!(!redacted.contains("abc123token"));
    assert!(!redacted.contains("test@example.com"));
    assert!(!redacted.contains("supersecret123"));
    assert!(redacted.contains("[REDACTED]"));
}

#[test]
fn memory_service_persists_by_scope_with_redaction() {
    let memory = MemoryService::new_in_memory().expect("memory");
    memory
        .put(
            MemoryScope::Session,
            "active-context",
            "Bearer abcdef secret=hidden",
            None,
        )
        .expect("put");

    let entry = memory
        .get(MemoryScope::Session, "active-context")
        .expect("get")
        .expect("entry");

    assert!(entry.value.contains("Bearer abcdef"));
    assert!(entry.redacted_value.contains("[REDACTED]"));
    assert!(memory
        .get(MemoryScope::Workspace, "active-context")
        .expect("get workspace")
        .is_none());
}

#[test]
fn memory_ttl_cleanup_removes_expired_entries() {
    let memory = MemoryService::new_in_memory().expect("memory");
    memory
        .put(MemoryScope::Persistent, "short-lived", "value", Some(0))
        .expect("put ttl");

    let purged = memory.cleanup_expired().expect("cleanup");
    assert!(purged >= 1);
    assert!(memory
        .get(MemoryScope::Persistent, "short-lived")
        .expect("get after cleanup")
        .is_none());
}
