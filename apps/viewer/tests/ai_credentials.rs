/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};
use viewer::{
    ai::{
        credentials::{self, Index, Store},
        Provider, Session,
    },
    Diagnostic,
};
use zeroize::Zeroizing;
#[derive(Default)]
struct Vault {
    entries: RefCell<BTreeMap<Provider, Zeroizing<String>>>,
    blocked: Cell<bool>,
}
impl Store for Vault {
    fn read(&self, p: Provider) -> Result<Option<Zeroizing<String>>, Diagnostic> {
        if self.blocked.get() {
            return Err(Diagnostic::new("AiVault", "Vault unavailable"));
        }
        Ok(self.entries.borrow().get(&p).cloned())
    }
    fn write(&self, p: Provider, value: &str) -> Result<(), Diagnostic> {
        if self.blocked.get() {
            return Err(Diagnostic::new("AiVault", "Vault unavailable"));
        }
        self.entries
            .borrow_mut()
            .insert(p, Zeroizing::new(value.into()));
        Ok(())
    }
    fn delete(&self, p: Provider) -> Result<(), Diagnostic> {
        if self.blocked.get() {
            return Err(Diagnostic::new("AiVault", "Vault unavailable"));
        }
        self.entries.borrow_mut().remove(&p);
        Ok(())
    }
}
fn payload() -> String {
    r#"{"version":1,"provider":"openai","model":"test-model","key":"synthetic-api-key","refresh":"","client":"","expires":0,"models":[]}"#.into()
}
#[test]
fn remembered_keys_roundtrip_across_sessions_without_catalog_secret_readback() {
    let vault = Vault::default();
    let mut first = Session::default();
    first
        .configure("openai", "test-model", "synthetic-api-key".into())
        .unwrap();
    credentials::snapshot(&first, Provider::OpenAi)
        .unwrap()
        .save(&vault, Provider::OpenAi)
        .unwrap();
    drop(first);
    let restored = credentials::restore(&vault, Provider::OpenAi, 0)
        .unwrap()
        .unwrap();
    let mut second = Session::default();
    second
        .configure("gemini", "other-model", "synthetic-other-key".into())
        .unwrap();
    let catalog = second.import_restored(restored);
    assert_eq!(catalog["selected"], "openai");
    assert_eq!(catalog["verified"], false);
    assert!(!catalog.to_string().contains("synthetic"));
    assert!(catalog["providers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == "gemini" && p["configured"] == true));
    vault.delete(Provider::OpenAi).unwrap();
    assert!(credentials::restore(&vault, Provider::OpenAi, 0)
        .unwrap()
        .is_none());
}
#[test]
fn closed_records_reject_wrong_owners_duplicates_versions_and_oversize() {
    let vault = Vault::default();
    for data in [
        payload().replace("\"version\":1", "\"version\":2"),
        payload().replace("\"provider\":\"openai\"", "\"provider\":\"grok\""),
        payload().replace("\"model\":", "\"extra\":true,\"model\":"),
        payload().replace("\"version\":1", "\"version\":1,\"version\":1"),
        " ".repeat(262145),
    ] {
        vault.write(Provider::OpenAi, &data).unwrap();
        let error = credentials::restore(&vault, Provider::OpenAi, 0)
            .err()
            .unwrap();
        assert!(!error.message.contains("synthetic-api-key"));
    }
}
#[test]
fn unavailable_vault_has_no_plaintext_fallback_and_preserves_existing_entries() {
    let vault = Vault::default();
    vault.write(Provider::OpenAi, &payload()).unwrap();
    let mut session = Session::default();
    session
        .configure("openai", "replacement-model", "synthetic-new-key".into())
        .unwrap();
    vault.blocked.set(true);
    assert!(credentials::snapshot(&session, Provider::OpenAi)
        .unwrap()
        .save(&vault, Provider::OpenAi)
        .is_err());
    assert!(credentials::restore(&vault, Provider::OpenAi, 0).is_err());
    assert!(vault.delete(Provider::OpenAi).is_err());
    vault.blocked.set(false);
    let original = credentials::restore(&vault, Provider::OpenAi, 0)
        .unwrap()
        .unwrap()
        .catalog();
    assert_eq!(original["providers"][0]["model"], "test-model");
}
#[test]
fn known_oauth_expiry_is_rejected_before_activation_without_api_key_fallback() {
    let vault = Vault::default();
    let record = serde_json::json!({"version":1,"provider":"chatgpt","model":"account-model","key":"synthetic-access","refresh":"synthetic-refresh","client":"test-client","expires":100,"models":[["account-model","Account model"]]});
    vault.write(Provider::ChatGpt, &record.to_string()).unwrap();
    let error = credentials::restore(&vault, Provider::ChatGpt, 100)
        .err()
        .unwrap();
    assert_eq!(error.code, "AiSignInExpired");
    assert!(!error.message.contains("synthetic"));
    assert!(credentials::restore(&vault, Provider::OpenAi, 100)
        .unwrap()
        .is_none());
}
#[test]
fn nonsecret_index_is_bounded_and_does_not_unlock_or_contain_a_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connections.json");
    let mut index = Index::default();
    index.providers.insert("openai".into(), "test-model".into());
    index.preferred = Some("openai".into());
    index.write(&path).unwrap();
    let bytes = std::fs::read_to_string(&path).unwrap();
    assert!(!bytes.contains("key"));
    assert_eq!(
        Index::read(&path).unwrap().providers["openai"],
        "test-model"
    );
    std::fs::write(&path, " ".repeat(8193)).unwrap();
    assert!(Index::read(&path).is_err());
    std::fs::write(
        &path,
        r#"{"providers":{"unknown":"model"},"preferred":null}"#,
    )
    .unwrap();
    assert!(Index::read(&path).is_err());
}
#[test]
fn authentication_rejection_requires_reconnect_without_deleting_the_saved_key() {
    let vault = Vault::default();
    let mut session = Session::default();
    session
        .configure("openai", "test-model", "synthetic-api-key".into())
        .unwrap();
    credentials::snapshot(&session, Provider::OpenAi)
        .unwrap()
        .save(&vault, Provider::OpenAi)
        .unwrap();
    assert_eq!(
        viewer::ai::transport::status_error(401).code,
        "AiAuthentication"
    );
    for status in [403, 404, 429, 500] {
        assert_ne!(
            viewer::ai::transport::status_error(status).code,
            "AiAuthentication"
        );
    }
    let catalog = session.authentication_rejected();
    assert_eq!(catalog["authentication_required"], true);
    assert_eq!(catalog["enabled"], false);
    assert!(credentials::restore(&vault, Provider::OpenAi, 0)
        .unwrap()
        .is_some());
    session
        .configure("openai", "test-model", "synthetic-replacement".into())
        .unwrap();
    assert_eq!(session.catalog()["authentication_required"], false);
}

#[test]
fn saved_chatgpt_roundtrip_and_wrong_owner_are_checked() {
    let vault = Vault::default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let record = serde_json::json!({"version":1,"provider":"chatgpt","model":"account-model","key":"synthetic-access","refresh":"synthetic-refresh","client":"test-client","expires":now+3600,"models":[["account-model","Account model"]]});
    vault.write(Provider::ChatGpt, &record.to_string()).unwrap();
    let restored = credentials::restore(&vault, Provider::ChatGpt, now)
        .unwrap()
        .unwrap();
    let credential = credentials::snapshot(&restored, Provider::ChatGpt).unwrap();
    assert!(credential.save(&vault, Provider::OpenAi).is_err());
    credential.save(&vault, Provider::ChatGpt).unwrap();
    let catalog = credentials::restore(&vault, Provider::ChatGpt, now)
        .unwrap()
        .unwrap()
        .catalog();
    assert!(!catalog.to_string().contains("synthetic"));
    assert_eq!(catalog["selected"], "chatgpt");
    let mut duplicate = record.clone();
    duplicate["models"] = serde_json::json!([["account-model", "One"], ["account-model", "Two"]]);
    vault
        .write(Provider::ChatGpt, &duplicate.to_string())
        .unwrap();
    assert!(credentials::restore(&vault, Provider::ChatGpt, now).is_err());
}
#[test]
fn preferences_reject_directories_symlinks_and_invalid_writes() {
    let directory = tempfile::tempdir().unwrap();
    assert!(Index::read(directory.path()).is_err());
    let path = directory.path().join("index.json");
    let mut invalid = Index::default();
    invalid.providers.insert("unknown".into(), "model".into());
    assert!(invalid.write(&path).is_err());
    assert!(!path.exists());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("missing-target", &path).unwrap();
        assert!(Index::read(&path).is_err());
        assert!(Index::default().write(&path).is_err());
    }
}
