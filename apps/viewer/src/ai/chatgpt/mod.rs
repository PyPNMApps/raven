/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! ChatGPT plan access through explicit browser consent, separate from API-key billing.
mod login;
pub(super) mod responses;
use super::protocol::{invalid, parse_json};
use crate::Diagnostic;
pub use login::SignIn;
use serde_json::{json, Value};
use std::{io::Read, sync::atomic::AtomicBool, time::Duration};
use zeroize::Zeroizing;

pub(super) const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
const REVOKE_URL: &str = "https://auth.openai.com/api/accounts/oauth/revoke";
const JWKS_URL: &str = "https://auth.openai.com/.well-known/jwks.json";
const MODELS_URL: &str = "https://api.openai.com/v1/models";
/// Small authentication replies remain separate from rich model capability metadata.
const MAX_REPLY_BYTES: usize = 262_144;
/// Application budget for the explicitly requested catalog, not a protocol limit.
const MAX_MODEL_CATALOG_BYTES: usize = 8 * 1024 * 1024;

/// In-memory credential set. Never serializable or printable; copies zeroize on drop.
#[derive(Clone)]
pub struct Connection {
    pub(super) access: Zeroizing<String>,
    refresh: Zeroizing<String>,
    client: String,
    expires: u64,
    pub(super) model: String,
    models: Vec<(String, String)>,
}
impl Connection {
    /// Returns safe account-specific model choices without credentials or identity claims.
    /// # Returns
    /// Catalog values suitable for the model picker.
    pub fn catalog(&self) -> Value {
        json!({"signed_in":true,"model":self.model,"models":self.models.iter().map(|(id,label)|json!({"id":id,"label":label})).collect::<Vec<_>>(),"expires_epoch":self.expires.to_string()})
    }
    /// Selects an exact model from the authenticated visible catalog.
    /// # Arguments
    /// * `model` - Catalog slug, never an arbitrary endpoint or inferred entitlement.
    /// # Errors
    /// Rejects unknown models and expired authentication.
    pub fn select_model(&mut self, model: &str) -> Result<(), Diagnostic> {
        self.valid()?;
        if !self.models.iter().any(|(id, _)| id == model) {
            return Err(invalid());
        }
        self.model = model.into();
        Ok(())
    }
    pub(super) fn valid(&self) -> Result<(), Diagnostic> {
        if now()? + 30 >= self.expires {
            return Err(Diagnostic::new("AiSignInExpired", "ChatGPT sign-in expired. Select ChatGPT and press Connect again; no API-key fallback was used."));
        }
        Ok(())
    }
    pub(super) fn contains_secret(&self, text: &str) -> bool {
        text.contains(self.access.as_str()) || text.contains(self.refresh.as_str())
    }
    /// Ends this renewable remote session after local sign-out has already cleared it.
    /// # Arguments
    /// * `cancel` - Operation cancellation flag.
    /// # Returns
    /// Success only after the revocation endpoint acknowledges the request.
    /// # Errors
    /// Sanitized failure; local sign-out is still effective and must not be undone.
    pub fn revoke(self, cancel: &AtomicBool) -> Result<(), Diagnostic> {
        super::protocol::check_cancel(cancel)?;
        let response = http().post(REVOKE_URL).send_form(&[
            ("token", self.refresh.as_str()),
            ("token_type_hint", "refresh_token"),
            ("client_id", &self.client),
        ]);
        read_response(response, cancel).map(|_| ())
    }
}
fn now() -> Result<u64, Diagnostic> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| invalid())
}
fn http() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(60))
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(8))
        .build()
}
fn read_response(
    result: Result<ureq::Response, ureq::Error>,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Diagnostic> {
    read_response_with_limit(result, cancel, MAX_REPLY_BYTES)
}
fn read_response_with_limit(
    result: Result<ureq::Response, ureq::Error>,
    cancel: &AtomicBool,
    max_bytes: usize,
) -> Result<Vec<u8>, Diagnostic> {
    super::protocol::check_cancel(cancel)?;
    let response = match result {
        Ok(response) => response,
        Err(ureq::Error::Status(status, _)) => {
            tracing::warn!(target: "viewer::ai", status, "ChatGPT HTTP request rejected");
            return Err(Diagnostic::new("AiChatGptHttp", format!("ChatGPT returned HTTP {status}. Check consent, access and plan limits. No request was retried.")));
        }
        Err(_) => return Err(Diagnostic::new("AiChatGptNetwork", "Could not reach ChatGPT or the request timed out. Check the network and retry explicitly.")),
    };
    if response.status() != 200 {
        return Err(Diagnostic::new(
            "AiChatGptHttp",
            format!(
                "ChatGPT returned unexpected HTTP {}. Redirects are not followed.",
                response.status()
            ),
        ));
    }
    let mut bytes = Vec::new();
    let mut reader = response.into_reader();
    let mut chunk = [0; 4096];
    loop {
        super::protocol::check_cancel(cancel)?;
        let count = reader.read(&mut chunk).map_err(|_| {
            Diagnostic::new(
                "AiChatGptNetwork",
                "ChatGPT response interrupted or timed out.",
            )
        })?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > max_bytes {
            tracing::warn!(target:"viewer::ai",limit_bytes=max_bytes,"ChatGPT response exceeded its byte budget");
            return Err(Diagnostic::new(
                "AiChatGptLimit",
                format!("ChatGPT response exceeded its {max_bytes}-byte limit."),
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    super::protocol::check_cancel(cancel)?;
    tracing::info!(target:"viewer::ai",response_bytes=bytes.len(),"ChatGPT bounded response received");
    Ok(bytes)
}
fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._/:".contains(&byte))
}

#[cfg(test)]
mod tests;

/// Adds an authentication stage without exposing bodies, tokens or account identifiers.
fn sign_in_stage(stage: &'static str, error: Diagnostic) -> Diagnostic {
    if error.code == "AiCancelled" {
        return error;
    }
    tracing::warn!(target:"viewer::ai",stage,code=%error.code,"ChatGPT sign-in failed");
    let detail = if error.code == "AiContract" {
        "The response format was not recognized.".to_owned()
    } else if error.code.starts_with("AiChatGpt") {
        error.message
    } else {
        "The response failed validation.".to_owned()
    };
    Diagnostic::new(
        "AiChatGptSignIn",
        format!("ChatGPT sign-in failed during {stage}. {detail}"),
    )
}
