/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Explicit asynchronous OS-vault jobs; no secret is serialized to Qt properties.
use super::bridge;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::{
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use viewer::{
    ai::{
        credentials::{self, Credential, Index, OsStore, Store},
        Provider, Session,
    },
    Diagnostic,
};

pub(super) fn index_path() -> Result<PathBuf, Diagnostic> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
    } else {
        std::env::var_os("HOME")
    }
    .ok_or_else(|| {
        Diagnostic::new(
            "AiVaultIndex",
            "Could not locate user connection preferences.",
        )
    })?;
    Ok(if cfg!(target_os = "windows") {
        PathBuf::from(base).join("RAVEN")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(base).join("Library/Application Support/RAVEN")
    } else {
        PathBuf::from(base).join(".config/raven")
    }
    .join("ai-connections.json"))
}
fn epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_secs())
        .unwrap_or(u64::MAX)
}
enum Action {
    Save(Credential, String),
    Remove,
    Restore,
    Forget(Option<viewer::ai::chatgpt::Connection>),
}
struct Outcome {
    restored: Option<Session>,
    message: String,
}
fn execute(
    action: Action,
    provider: Provider,
    path: &std::path::Path,
    cancel: &AtomicBool,
) -> Result<Outcome, Diagnostic> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Diagnostic::new(
            "AiCancelled",
            "Credential operation cancelled.",
        ));
    }
    let mut index = Index::read(path)?;
    let message = match action {
        Action::Save(secret, model) => {
            let previous = index.clone();
            index.providers.insert(provider.id().into(), model);
            index.preferred = Some(provider.id().into());
            // Write hints first: a failed preference write must never leave an orphan secret.
            index.write(path)?;
            if let Err(error) = secret.save(&OsStore, provider) {
                if previous.write(path).is_err() {
                    return Err(Diagnostic::new("AiVaultIndex", "Credential storage failed and preferences could not be reset. No plaintext fallback was used. Retry Forget connection after unlocking your vault."));
                }
                return Err(error);
            }
            "Connection remembered in the OS credential vault."
        }
        Action::Remove | Action::Forget(_) => unreachable!(),
        Action::Restore => {
            let restored = credentials::restore(&OsStore, provider, epoch())?.ok_or_else(|| {
                Diagnostic::new("AiSavedMissing", "The saved credential was not found. Forget the saved entry and configure the connection again.")
            })?;
            return Ok(Outcome {
                restored: Some(restored),
                message: "Saved connection restored. No analysis was sent.".into(),
            });
        }
    };
    Ok(Outcome {
        restored: None,
        message: message.into(),
    })
}
fn remove(
    provider: Provider,
    path: &std::path::Path,
    connection: Option<viewer::ai::chatgpt::Connection>,
    cancel: &AtomicBool,
) -> Result<Outcome, Diagnostic> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Diagnostic::new(
            "AiCancelled",
            "Credential operation cancelled.",
        ));
    }
    let mut index = Index::read(path)?;
    if index.providers.contains_key(provider.id()) {
        OsStore.delete(provider)?;
        index.providers.remove(provider.id());
        if index.preferred.as_deref() == Some(provider.id()) {
            index.preferred = index.providers.keys().next().cloned();
        }
        index.write(path)?;
    }
    let message = if let Some(connection) = connection {
        if connection.revoke(cancel).is_ok() {
            "Connection forgotten and ChatGPT signed out."
        } else {
            "Saved and local credentials removed. Remote ChatGPT revocation was not confirmed; check ChatGPT Settings."
        }
    } else {
        "Saved connection removed. No provider key was revoked."
    };
    Ok(Outcome {
        restored: None,
        message: message.into(),
    })
}
impl bridge::AiBackend {
    /// Saves or removes the active provider's credentials only with explicit consent.
    /// # Arguments
    /// * `provider`, `remember` - Fixed selected owner and requested persistence policy.
    /// # Errors
    /// Publishes a sanitized vault failure; never falls back to a plaintext file.
    pub fn remember_connection(self: Pin<&mut Self>, provider: &QString, remember: bool) {
        if self.rust().busy || self.rust().forced_demo || self.rust().demo_mode {
            return;
        }
        let Some(owner) = Provider::from_id(&provider.to_string()) else {
            return;
        };
        let action = if remember {
            match credentials::snapshot(&self.rust().session, owner) {
                Ok(secret) => {
                    let catalog = self.rust().session.catalog();
                    let model = catalog["providers"]
                        .as_array()
                        .and_then(|items| items.iter().find(|p| p["id"] == owner.id()))
                        .and_then(|p| p["model"].as_str())
                        .unwrap_or("")
                        .to_owned();
                    Action::Save(secret, model)
                }
                Err(error) => {
                    self.set_error_message(QString::from(error.message));
                    return;
                }
            }
        } else {
            Action::Remove
        };
        self.vault_job(owner, action);
    }
    /// Restores a saved provider into Rust without copying its secret to the form.
    /// # Arguments
    /// * `provider` - Explicit user-selected fixed provider ID.
    /// # Errors
    /// Notifies about missing, locked, malformed or expired saved credentials.
    pub fn restore_connection(self: Pin<&mut Self>, provider: &QString) {
        if let Some(provider) = Provider::from_id(&provider.to_string()) {
            self.vault_job(provider, Action::Restore);
        }
    }
    /// Clears local credentials and explicitly removes the selected saved connection.
    /// # Arguments
    /// * `provider` - Fixed selected owner; ChatGPT also attempts remote revocation.
    pub fn forget_connection(mut self: Pin<&mut Self>, provider: &QString) {
        if self.rust().busy || self.rust().forced_demo || self.rust().demo_mode {
            return;
        }
        let Some(owner) = Provider::from_id(&provider.to_string()) else {
            return;
        };
        self.as_mut().invalidate();
        let connection = if owner == Provider::ChatGpt {
            self.as_mut().rust_mut().session.disconnect_chatgpt()
        } else {
            let _ = self.as_mut().rust_mut().session.forget(owner.id());
            None
        };
        let catalog = self.rust().session.catalog();
        self.as_mut()
            .set_state_json(QString::from(catalog.to_string()));
        self.vault_job(owner, Action::Forget(connection));
    }
    /// Records opt-in for the next explicit browser sign-in without storing anything.
    /// # Arguments
    /// * `remember` - User checkbox; false keeps browser credentials session-only.
    pub fn remember_next_signin(mut self: Pin<&mut Self>, remember: bool) {
        self.as_mut().rust_mut().remember_signin = remember;
    }
    /// Refreshes known local expiry status without querying a provider.
    pub fn check_credential_status(mut self: Pin<&mut Self>) {
        if self.rust().busy || self.rust().forced_demo || self.rust().demo_mode {
            return;
        }
        let initial = self.rust().session.catalog();
        let expired = initial["selected"] == "chatgpt" && initial["chatgpt"]["expired"] == true;
        let catalog = if expired {
            self.as_mut().invalidate();
            self.as_mut().rust_mut().session.authentication_rejected()
        } else {
            initial
        };
        self.as_mut()
            .set_state_json(QString::from(catalog.to_string()));
        if expired {
            self.set_credential_status(QString::from(
                "ChatGPT sign-in expired. Connect again to sign in; no analysis was sent.",
            ));
        }
    }
    fn vault_job(mut self: Pin<&mut Self>, provider: Provider, action: Action) {
        if self.rust().busy || self.rust().forced_demo || self.rust().demo_mode {
            return;
        }
        let path = match index_path() {
            Ok(path) => path,
            Err(error) => {
                self.set_error_message(QString::from(error.message));
                return;
            }
        };
        self.as_mut().invalidate();
        let cancel = Arc::new(AtomicBool::new(false));
        let generation = self.rust().vault_generation.wrapping_add(1);
        self.as_mut().rust_mut().vault_generation = generation;
        self.as_mut().rust_mut().control = cancel.clone();
        self.as_mut().set_busy(true);
        self.as_mut().set_error_message(QString::default());
        self.as_mut()
            .set_credential_status(QString::from("Accessing the OS credential vault…"));
        let thread = self.qt_thread();
        let spawn = std::thread::Builder::new()
            .name("ai-credentials".into())
            .spawn(move || {
                let result = match action {
                    Action::Remove => remove(provider, &path, None, &cancel),
                    Action::Forget(connection) => remove(provider, &path, connection, &cancel),
                    action => execute(action, provider, &path, &cancel),
                };
                let index = Index::read(&path).unwrap_or_default();
                let _ = thread.queue(move |mut state| {
                    if state.rust().vault_generation != generation {
                        return;
                    }
                    state.as_mut().set_busy(false);
                    let saved = serde_json::to_string(&index).unwrap_or_else(|_| "{}".into());
                    state.as_mut().set_saved_json(QString::from(saved));
                    match result {
                        Ok(outcome) => {
                            if let Some(session) = outcome.restored {
                                if cancel.load(Ordering::Relaxed)
                                    || state.rust().forced_demo
                                    || state.rust().demo_mode
                                {
                                    state.set_credential_status(QString::from(
                                        "Credential restore cancelled; no connection activated.",
                                    ));
                                    return;
                                }
                                let catalog = state.as_mut().rust_mut().session.import_restored(session);
                                state.as_mut().set_state_json(QString::from(catalog.to_string()));
                                state.as_mut().set_reauth_provider(QString::default());
                                state.as_mut().set_credential_status(QString::from(outcome.message));
                                if provider == Provider::OpenAi {
                                    state.test_connection();
                                }
                            } else {
                                state.set_credential_status(QString::from(outcome.message));
                            }
                        }
                        Err(error) => {
                            if error.code == "AiSignInExpired" || error.code == "AiAuthentication" {
                                state.as_mut().set_reauth_provider(QString::from(provider.id()));
                            }
                            state.as_mut().set_credential_status(QString::from(
                                "The credential operation did not complete. Review the message below.",
                            ));
                            state.set_error_message(QString::from(error.message));
                        }
                    }
                });
            });
        if spawn.is_err() {
            self.as_mut().set_busy(false);
            self.set_error_message(QString::from("Could not start the credential worker."));
        }
    }
}
