/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Optional read-only OFDM AI assistance. Credentials remain session-only.
pub mod chatgpt;
pub mod demo;
pub mod evidence;
mod presentation;
pub mod protocol;
pub mod transport;
use crate::Diagnostic;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use zeroize::Zeroizing;

/// Maximum provider credential bytes retained for the current process only.
const MAX_API_KEY_BYTES: usize = 4096;
/// Bound user-selected model identifiers before any future request construction.
const MAX_MODEL_BYTES: usize = 128;

/// Reads an explicitly selected plain-text credential without retaining its path.
///
/// # Arguments
/// * `path` - Local regular file containing just one API key, with optional LF/CRLF.
/// # Returns
/// A zeroizing secret buffer for the temporary masked form; no session is changed.
/// # Errors
/// Rejects unreadable, nonregular, oversized or malformed files with generic errors.
pub fn read_key_file(path: &Path) -> Result<Zeroizing<String>, Diagnostic> {
    let invalid = || {
        Diagnostic::new(
            "AiKeyFile",
            "Select a plain-text file containing only one API key (up to 4096 bytes).",
        )
    };
    if !std::fs::metadata(path).map_err(|_| invalid())?.is_file() {
        return Err(invalid());
    }
    let file = std::fs::File::open(path).map_err(|_| invalid())?;
    let metadata = file.metadata().map_err(|_| invalid())?;
    let max_file_bytes = MAX_API_KEY_BYTES + 2;
    if !metadata.is_file() || metadata.len() > max_file_bytes as u64 {
        return Err(invalid());
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take((max_file_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > max_file_bytes {
        return Err(invalid());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
    let key = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if key.is_empty()
        || key.len() > MAX_API_KEY_BYTES
        || !key.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(invalid());
    }
    Ok(Zeroizing::new(key.to_owned()))
}

/// Documented shared-chat API candidates, not a claim of live compatibility.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provider {
    OpenAi,
    ChatGpt,
    Gemini,
    Claude,
    Grok,
}
impl Provider {
    /// Reads a fixed provider identifier without accepting arbitrary endpoints.
    /// # Arguments
    /// * `identifier` - Exact application provider ID.
    /// # Returns
    /// Known provider, or None for unsupported identifiers (including Kiro).
    pub fn from_id(identifier: &str) -> Option<Self> {
        match identifier {
            "chatgpt" => Some(Self::ChatGpt),
            "openai" => Some(Self::OpenAi),
            "gemini" => Some(Self::Gemini),
            "claude" => Some(Self::Claude),
            "grok" => Some(Self::Grok),
            _ => None,
        }
    }
    /// Returns the stable nonsecret configuration identifier.
    /// # Returns
    /// A fixed lowercase provider ID.
    pub fn id(self) -> &'static str {
        match self {
            Self::ChatGpt => "chatgpt",
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
            Self::Claude => "claude",
            Self::Grok => "grok",
        }
    }
    /// Returns the provider label used by native controls.
    /// # Returns
    /// A fixed product label, without model/version guesses.
    pub fn label(self) -> &'static str {
        match self {
            Self::ChatGpt => "ChatGPT (subscription)",
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
            Self::Claude => "Claude",
            Self::Grok => "Grok",
        }
    }
    /// Exposes the fixed shared-chat endpoint for review, without making requests.
    /// # Returns
    /// Documented HTTPS endpoint; provider capabilities remain separately tested.
    pub fn endpoint(self) -> &'static str {
        match self {
            Self::ChatGpt => chatgpt::RESPONSES_URL,
            Self::OpenAi => "https://api.openai.com/v1/chat/completions",
            Self::Gemini => {
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
            }
            Self::Claude => "https://api.anthropic.com/v1/chat/completions",
            Self::Grok => "https://api.x.ai/v1/chat/completions",
        }
    }
}

/// Private, deliberately non-Debug/non-Serialize credential. Never exposed to Qt.
struct Configuration {
    model: String,
    _api_key: Zeroizing<String>,
}

/// Independent session-only provider state; no filesystem or network capabilities.
#[derive(Default)]
pub struct Session {
    configurations: BTreeMap<Provider, Configuration>,
    selected: Option<Provider>,
    enabled: bool,
    pending: Option<protocol::Pending>,
    chatgpt: Option<chatgpt::Connection>,
}
impl Session {
    /// Validates and stores a provider's model/key without contacting that service.
    /// # Arguments
    /// * `provider_id` - Known provider ID.
    /// * `model` - Explicit model identifier; never inferred from key contents.
    /// * `api_key` - Owned secret, zeroized on rejection, replacement or destruction.
    ///   Empty retains the same provider's previously stored key for a model change.
    /// # Returns
    /// Nonsecret catalog; success selects this provider and enables local setup.
    /// # Errors
    /// Rejects missing/oversized/unsafe fields transactionally; errors contain no key.
    pub fn configure(
        &mut self,
        provider_id: &str,
        model: &str,
        api_key: String,
    ) -> Result<Value, Diagnostic> {
        self.pending = None;
        let api_key = Zeroizing::new(api_key);
        let provider = Provider::from_id(provider_id)
            .ok_or_else(|| Diagnostic::new("AiProvider", "Select a supported AI provider."))?;
        if provider == Provider::ChatGpt {
            return Err(Diagnostic::new(
                "AiSignIn",
                "Use Continue with ChatGPT instead of an API key.",
            ));
        }
        let model = model.trim();
        if model.is_empty()
            || model.len() > MAX_MODEL_BYTES
            || !model
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._/:".contains(&byte))
        {
            return Err(Diagnostic::new(
                "AiModel",
                "Enter a valid model identifier (up to 128 bytes).",
            ));
        }
        if api_key.is_empty() {
            let configuration = self.configurations.get_mut(&provider).ok_or_else(|| {
                Diagnostic::new("AiCredential", "Enter an API key for this provider.")
            })?;
            configuration.model = model.into();
        } else {
            if api_key.len() > MAX_API_KEY_BYTES
                || !api_key.bytes().all(|byte| byte.is_ascii_graphic())
            {
                return Err(Diagnostic::new(
                    "AiCredential",
                    "Enter an API key without spaces or control characters (up to 4096 bytes).",
                ));
            }
            self.configurations.insert(
                provider,
                Configuration {
                    model: model.into(),
                    _api_key: api_key,
                },
            );
        }
        self.selected = Some(provider);
        self.enabled = true;
        Ok(self.catalog())
    }

    /// Enables or disables configured assistance without deleting its credentials.
    /// # Arguments
    /// * `enabled` - Requested local setup state; no network action is implied.
    /// # Returns
    /// Updated nonsecret catalog.
    /// # Errors
    /// Enabling requires a selected configuration; failure preserves existing state.
    pub fn set_enabled(&mut self, enabled: bool) -> Result<Value, Diagnostic> {
        self.pending = None;
        if enabled && self.selected.is_none() {
            return Err(Diagnostic::new(
                "AiCredential",
                "Configure an AI provider first.",
            ));
        }
        self.enabled = enabled;
        Ok(self.catalog())
    }

    /// Forgets only the selected provider's session key/model; no remote revocation.
    /// # Arguments
    /// * `provider_id` - Exact known provider to forget.
    /// # Returns
    /// Updated catalog; removing an inactive provider preserves the active one.
    /// # Errors
    /// Unknown provider IDs are rejected without altering any configuration.
    pub fn forget(&mut self, provider_id: &str) -> Result<Value, Diagnostic> {
        self.pending = None;
        let provider = Provider::from_id(provider_id)
            .ok_or_else(|| Diagnostic::new("AiProvider", "Select a supported AI provider."))?;
        self.configurations.remove(&provider);
        if provider == Provider::ChatGpt {
            self.chatgpt = None;
        }
        if self.selected == Some(provider) {
            self.selected = None;
            self.enabled = false;
        }
        Ok(self.catalog())
    }

    /// Describes provider options and session setup without serializing credentials.
    /// # Returns
    /// Small explicit JSON with configured/untested states; verified is always false.
    pub fn catalog(&self) -> Value {
        let providers: Vec<_> = [
            Provider::OpenAi,
            Provider::ChatGpt,
            Provider::Gemini,
            Provider::Claude,
            Provider::Grok,
        ]
        .into_iter()
        .map(|provider| {
            let configuration = self.configurations.get(&provider);
            json!({"id":provider.id(), "label":provider.label(), "endpoint":provider.endpoint(),
                    "model":configuration.map(|value| value.model.as_str()).unwrap_or(""),
                    "configured":configuration.is_some(), "verified":false})
        })
        .collect();
        json!({"analysis_prompts":presentation::prepared_questions(),"providers":providers,"selected":self.selected.map(Provider::id),
            "enabled":self.enabled,"verified":false,"analysis_available":self.enabled && matches!(self.selected,Some(Provider::OpenAi|Provider::ChatGpt)),"chatgpt":self.chatgpt.as_ref().map(chatgpt::Connection::catalog),"storage":"session"})
    }
}

impl Session {
    /// Activates verified browser credentials without exposing them to the UI.
    /// # Arguments
    /// * `connection` - Verified OAuth result with account-specific models.
    /// # Returns
    /// Updated nonsecret provider catalog, invalidating old approvals.
    pub fn connect_chatgpt(&mut self, connection: chatgpt::Connection) -> Value {
        self.pending = None;
        self.configurations.insert(
            Provider::ChatGpt,
            Configuration {
                model: connection.model.clone(),
                _api_key: connection.access.clone(),
            },
        );
        self.chatgpt = Some(connection);
        self.selected = Some(Provider::ChatGpt);
        self.enabled = true;
        self.catalog()
    }
    /// Changes the signed-in ChatGPT model using the authenticated model catalog.
    /// # Arguments
    /// * `model` - Exact visible model slug.
    /// # Returns
    /// Updated nonsecret catalog.
    /// # Errors
    /// Rejects unknown models or missing/expired sign-in.
    pub fn select_chatgpt_model(&mut self, model: &str) -> Result<Value, Diagnostic> {
        self.pending = None;
        let connection = self.chatgpt.as_mut().ok_or_else(protocol::invalid)?;
        connection.select_model(model)?;
        self.configurations
            .get_mut(&Provider::ChatGpt)
            .ok_or_else(protocol::invalid)?
            .model = model.into();
        self.selected = Some(Provider::ChatGpt);
        self.enabled = true;
        Ok(self.catalog())
    }
    /// Clears local tokens immediately, returning an opaque revocation job.
    /// # Returns
    /// Credential owner for explicit remote sign-out, or None if disconnected.
    pub fn disconnect_chatgpt(&mut self) -> Option<chatgpt::Connection> {
        self.pending = None;
        self.configurations.remove(&Provider::ChatGpt);
        if self.selected == Some(Provider::ChatGpt) {
            self.selected = None;
            self.enabled = false;
        }
        self.chatgpt.take()
    }
}
