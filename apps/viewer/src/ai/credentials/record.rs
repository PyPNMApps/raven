/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Closed private vault records; public catalogs never serialize these fields.
use super::super::{chatgpt::Connection, Provider, Session};
use crate::Diagnostic;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

/// Opaque private serialized snapshot. Deliberately not Debug or Serialize.
pub struct Credential {
    owner: Provider,
    bytes: Zeroizing<String>,
}
impl Credential {
    /// Persists this record only through an explicitly supplied secure Store.
    /// # Arguments
    /// * `store`, `provider` - Authorized vault and fixed selected owner.
    /// # Errors
    /// Reports store failures without reflecting secret data.
    pub fn save(&self, store: &dyn super::Store, provider: Provider) -> Result<(), Diagnostic> {
        if provider != self.owner {
            return Err(invalid());
        }
        store.write(provider, &self.bytes)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u8,
    provider: String,
    model: String,
    key: String,
    refresh: String,
    client: String,
    expires: u64,
    models: Vec<(String, String)>,
}
impl Drop for Record {
    fn drop(&mut self) {
        self.key.zeroize();
        self.refresh.zeroize();
    }
}
pub(super) fn model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._/:".contains(&b))
}
fn invalid() -> Diagnostic {
    tracing::warn!(target:"viewer::ai::credentials", code="AiSavedCredential", "Saved credential validation rejected");
    Diagnostic::new("AiSavedCredential","The saved connection is malformed or unsupported. Forget it and connect again; no credentials were exposed.")
}
/// Copies an active connection into an opaque zeroizing vault payload.
/// # Arguments
/// * `session`, `provider` - Current Rust credential owner and explicit selected provider.
/// # Returns
/// A bounded record, never presentation JSON or a filesystem path.
/// # Errors
/// Rejects missing configurations or an oversized credential snapshot.
pub fn snapshot(session: &Session, provider: Provider) -> Result<Credential, Diagnostic> {
    let config = session.configurations.get(&provider).ok_or_else(invalid)?;
    let connection = if provider == Provider::ChatGpt {
        Some(session.chatgpt.as_ref().ok_or_else(invalid)?)
    } else {
        None
    };
    if let Some(connection) = connection {
        connection.valid()?;
    }
    let record = Record {
        version: 1,
        provider: provider.id().into(),
        model: config.model.clone(),
        key: config._api_key.to_string(),
        refresh: connection
            .map(|c| c.refresh.to_string())
            .unwrap_or_default(),
        client: connection.map(|c| c.client.clone()).unwrap_or_default(),
        expires: connection.map(|c| c.expires).unwrap_or(0),
        models: connection.map(|c| c.models.clone()).unwrap_or_default(),
    };
    let bytes = Zeroizing::new(serde_json::to_string(&record).map_err(|_| invalid())?);
    if bytes.len() > 262_144 {
        return Err(invalid());
    }
    Ok(Credential {
        owner: provider,
        bytes,
    })
}
/// Restores one explicit vault record into a new isolated in-memory Session.
/// # Arguments
/// * `store`, `provider`, `now` - Vault, expected fixed owner and current epoch seconds.
/// # Returns
/// A validated Session, or None when no saved credential exists; no network request.
/// # Errors
/// Refuses malformed/wrong-owner records and expired ChatGPT access. API-key
/// expiry is unknown until a provider rejects it; restoring never claims verification.
pub fn restore(
    store: &dyn super::Store,
    provider: Provider,
    now: u64,
) -> Result<Option<Session>, Diagnostic> {
    let Some(bytes) = store.read(provider)? else {
        return Ok(None);
    };
    if bytes.len() > 262_144 {
        return Err(invalid());
    }
    let record: Record = serde_json::from_str(&bytes).map_err(|_| invalid())?;
    if record.version != 1
        || record.provider != provider.id()
        || !model_id(&record.model)
        || record.key.is_empty()
        || record.key.len() > 262_144
        || !record.key.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(invalid());
    }
    let mut session = Session::default();
    if provider == Provider::ChatGpt {
        if record.expires <= now.saturating_add(30) {
            return Err(Diagnostic::new("AiSignInExpired","The saved ChatGPT sign-in has expired. Select ChatGPT and Connect to sign in again. No analysis was sent."));
        }
        if !model_id(&record.client)
            || record.refresh.is_empty()
            || record.refresh.len() > 262_144
            || !record.refresh.bytes().all(|byte| byte.is_ascii_graphic())
            || record.models.is_empty()
            || record.models.len() > 512
            || record
                .models
                .iter()
                .map(|(id, _)| id)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != record.models.len()
            || !record.models.iter().any(|(id, _)| id == &record.model)
            || record.models.iter().any(|(id, label)| {
                !model_id(id)
                    || label.is_empty()
                    || label.len() > 128
                    || label.chars().any(char::is_control)
            })
        {
            return Err(invalid());
        }
        session.connect_chatgpt(Connection {
            access: Zeroizing::new(record.key.clone()),
            refresh: Zeroizing::new(record.refresh.clone()),
            client: record.client.clone(),
            expires: record.expires,
            model: record.model.clone(),
            models: record.models.clone(),
        });
    } else {
        if !record.refresh.is_empty()
            || !record.client.is_empty()
            || record.expires != 0
            || !record.models.is_empty()
        {
            return Err(invalid());
        }
        session.configure(provider.id(), &record.model, record.key.clone())?;
    }
    Ok(Some(session))
}

impl Session {
    /// Merges a validated restored provider without discarding other session keys.
    /// # Arguments
    /// * `restored` - Isolated Session returned by the bounded vault restore owner.
    /// # Returns
    /// Nonsecret catalog; old pending analysis approval is invalidated.
    pub fn import_restored(&mut self, restored: Session) -> serde_json::Value {
        self.pending = None;
        for (provider, configuration) in restored.configurations {
            self.configurations.insert(provider, configuration);
            self.authentication_required.remove(&provider);
        }
        if restored.chatgpt.is_some() {
            self.chatgpt = restored.chatgpt;
        }
        self.selected = restored.selected;
        self.enabled = restored.enabled;
        self.catalog()
    }
}
