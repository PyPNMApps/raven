/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Loopback OAuth/PKCE with verified ID tokens; only nonsecret registration metadata persists.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::Instant,
};
use url::Url;

const AUTHORIZE_URL: &str = "https://auth.openai.com/api/accounts/authorize";
const ISSUER: &str = "https://auth.openai.com";
const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
const CALLBACK_PATH: &str = "/auth/callback";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_CALLBACK_BYTES: usize = 8192;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    host: String,
    client: Option<String>,
    subject: Option<String>,
}

/// One pending authorization. The listener exists before its public URL is exposed.
pub struct SignIn {
    listener: TcpListener,
    state: String,
    nonce: String,
    verifier: Zeroizing<String>,
    redirect: String,
    registration: Registration,
    metadata: PathBuf,
}
impl SignIn {
    /// Starts a loopback-only authorization attempt after an explicit button action.
    /// # Arguments
    /// * `metadata` - App-owned nonsecret host/client registration JSON location.
    /// # Returns
    /// Opaque listener/PKCE state; no remote request or browser launch occurs here.
    /// # Errors
    /// Rejects unsafe metadata paths, unavailable loopback or secure-random failure.
    pub fn begin(metadata: &Path) -> Result<Self, Diagnostic> {
        let registration = load_registration(metadata)?;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|_| {
            Diagnostic::new(
                "AiCallback",
                "Could not listen on a local sign-in callback port.",
            )
        })?;
        listener.set_nonblocking(true).map_err(|_| invalid())?;
        let redirect = format!(
            "http://127.0.0.1:{}{CALLBACK_PATH}",
            listener.local_addr().map_err(|_| invalid())?.port()
        );
        Ok(Self {
            listener,
            state: random()?,
            nonce: random()?,
            verifier: Zeroizing::new(random()?),
            redirect,
            registration,
            metadata: metadata.into(),
        })
    }
    /// Returns the fixed HTTPS authorization URL, containing no access/refresh token.
    /// # Returns
    /// Encoded browser URL bound to this attempt's state, nonce, host and callback.
    pub fn authorization_url(&self) -> String {
        let mut url = Url::parse(AUTHORIZE_URL).expect("fixed HTTPS authorization URL");
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()));
        let mut query = url.query_pairs_mut();
        query.extend_pairs([
            (
                "client_id",
                self.registration
                    .client
                    .as_deref()
                    .unwrap_or(DYNAMIC_CLIENT),
            ),
            ("ext_agent_host_id", &self.registration.host),
            ("response_type", "code"),
            ("redirect_uri", &self.redirect),
            (
                "scope",
                "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
            ),
            ("resource", "https://api.openai.com/v1"),
            ("state", &self.state),
            ("nonce", &self.nonce),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
        ]);
        if self.registration.client.is_none() {
            query.append_pair("agent_name_hint", "RAVEN");
        }
        drop(query);
        url.into()
    }
    /// Waits for consent, validates tokens and loads visible models on the worker.
    /// # Arguments
    /// * `cancel` - Cancels callback polling and prevents subsequent token/model requests.
    /// # Returns
    /// Verified private session; metadata retains only host/client/subject, never tokens.
    /// # Errors
    /// Rejects invalid state, denial, duplicate parameters, identity, scopes or timeouts.
    pub fn complete(mut self, cancel: &AtomicBool) -> Result<Connection, Diagnostic> {
        tracing::info!(target:"viewer::ai", "ChatGPT sign-in waiting for browser callback");
        let start = Instant::now();
        let (code, client) = loop {
            super::super::protocol::check_cancel(cancel)?;
            if start.elapsed() > LOGIN_TIMEOUT {
                return Err(Diagnostic::new(
                    "AiSignInTimeout",
                    "ChatGPT sign-in timed out. Press Continue with ChatGPT to try again.",
                ));
            }
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    let target = match read_target(&mut stream, cancel) {
                        Ok(target) => target,
                        Err(_) => {
                            respond(&mut stream, false);
                            continue;
                        }
                    };
                    match self.callback(&target) {
                        Ok(answer) => {
                            respond(&mut stream, true);
                            break answer;
                        }
                        Err(error) => {
                            respond(&mut stream, false);
                            if error.code == "AiConsentDenied" {
                                return Err(error);
                            }
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(40))
                }
                Err(_) => return Err(invalid()),
            }
        };
        super::super::protocol::check_cancel(cancel)?;
        let raw = Zeroizing::new(
            read_response(
                http().post(TOKEN_URL).send_form(&[
                    ("grant_type", "authorization_code"),
                    ("client_id", &client),
                    ("code", &code),
                    ("code_verifier", self.verifier.as_str()),
                    ("redirect_uri", &self.redirect),
                    ("resource", "https://api.openai.com/v1"),
                ]),
                cancel,
            )
            .map_err(|error| sign_in_stage("authorization-code exchange", error))?,
        );
        let token = parse_json(&raw, MAX_REPLY_BYTES)
            .map_err(|error| sign_in_stage("token response decoding", error))?;
        let scopes = token["scope"]
            .as_str()
            .ok_or_else(invalid)
            .map_err(|error| sign_in_stage("plan-consent validation", error))?;
        if !["openid", "resource.invoke", "chatgpt.tokens.use.direct"]
            .iter()
            .all(|scope| scopes.split_whitespace().any(|actual| actual == *scope))
            || token["token_type"]
                .as_str()
                .map(|value| value.eq_ignore_ascii_case("Bearer"))
                != Some(true)
        {
            return Err(Diagnostic::new(
                "AiPlanConsent",
                "ChatGPT plan permission was not granted. Sign in again and allow plan usage.",
            ));
        }
        tracing::info!(target:"viewer::ai", "ChatGPT code exchange and plan consent accepted");
        let jwks = parse_json(
            &read_response(http().get(JWKS_URL).call(), cancel)
                .map_err(|error| sign_in_stage("signing-key retrieval", error))?,
            MAX_REPLY_BYTES,
        )
        .map_err(|error| sign_in_stage("signing-key decoding", error))?;
        let subject = verify_identity(
            token["id_token"]
                .as_str()
                .ok_or_else(invalid)
                .map_err(|error| sign_in_stage("identity-token validation", error))?,
            &jwks,
            &client,
            &self.nonce,
            now()?,
        )
        .map_err(|error| sign_in_stage("identity verification", error))?;
        if self
            .registration
            .subject
            .as_ref()
            .is_some_and(|old| old != &subject)
        {
            return Err(Diagnostic::new(
                "AiAccountMismatch",
                "The signed-in account does not match this RAVEN registration.",
            ));
        }
        let access = secret(&token, "access_token")
            .map_err(|error| sign_in_stage("access-token validation", error))?;
        let refresh = secret(&token, "refresh_token")
            .map_err(|error| sign_in_stage("refresh-token validation", error))?;
        let seconds = token["expires_in"]
            .as_u64()
            .filter(|seconds| *seconds > 60 && *seconds <= 86400)
            .ok_or_else(invalid)?;
        self.registration.client = Some(client.clone());
        self.registration.subject = Some(subject);
        save_registration(&self.metadata, &self.registration)
            .map_err(|error| sign_in_stage("registration storage", error))?;
        tracing::info!(target:"viewer::ai", "ChatGPT identity verified; requesting model catalog");
        let authorization = Zeroizing::new(format!("Bearer {}", access.as_str()));
        let model_data = parse_json(
            &read_response_with_limit(
                http()
                    .get(MODELS_URL)
                    .set("Authorization", &authorization)
                    .call(),
                cancel,
                MAX_MODEL_CATALOG_BYTES,
            )
            .map_err(|error| sign_in_stage("model-catalog retrieval", error))?,
            MAX_MODEL_CATALOG_BYTES,
        )
        .map_err(|error| sign_in_stage("model-catalog decoding", error))?;
        let models = visible_models(&model_data)
            .map_err(|error| sign_in_stage("model-catalog validation", error))?;
        tracing::info!(target:"viewer::ai",model_count=models.len(),"ChatGPT sign-in complete");
        Ok(Connection {
            access,
            refresh,
            client,
            expires: now()?.checked_add(seconds).ok_or_else(invalid)?,
            model: models[0].0.clone(),
            models,
        })
    }
    pub(super) fn callback(&self, target: &str) -> Result<(Zeroizing<String>, String), Diagnostic> {
        if !target.starts_with('/') || target.starts_with("//") {
            return Err(invalid());
        }
        let url = Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| invalid())?;
        if url.path() != CALLBACK_PATH || url.fragment().is_some() {
            return Err(invalid());
        }
        let mut fields = std::collections::BTreeMap::new();
        for (name, value) in url.query_pairs() {
            if fields
                .insert(name.into_owned(), value.into_owned())
                .is_some()
            {
                return Err(invalid());
            }
        }
        let actual = fields.get("state").ok_or_else(invalid)?;
        if actual != &self.state {
            return Err(invalid());
        }
        if fields.contains_key("error") {
            return Err(Diagnostic::new(
                "AiConsentDenied",
                "ChatGPT sign-in was declined. No analysis was sent.",
            ));
        }
        let code = fields
            .get("code")
            .filter(|code| !code.is_empty() && code.len() <= 4096)
            .ok_or_else(invalid)?;
        let client = fields
            .get("client_id")
            .or(self.registration.client.as_ref())
            .filter(|client| safe_identifier(client) && client.as_str() != DYNAMIC_CLIENT)
            .ok_or_else(invalid)?;
        if self
            .registration
            .client
            .as_ref()
            .is_some_and(|old| old != client)
        {
            return Err(invalid());
        }
        Ok((Zeroizing::new(code.clone()), client.clone()))
    }
}
fn host_id() -> Result<String, Diagnostic> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| invalid())?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut hex = String::with_capacity(32);
    for byte in bytes {
        std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}")).map_err(|_| invalid())?;
    }
    Ok(format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}
fn random() -> Result<String, Diagnostic> {
    let mut bytes = [0; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| invalid())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}
fn secret(value: &Value, name: &str) -> Result<Zeroizing<String>, Diagnostic> {
    value[name]
        .as_str()
        .filter(|secret| {
            !secret.is_empty()
                && secret.len() <= 16384
                && secret.bytes().all(|byte| byte.is_ascii_graphic())
        })
        .map(|secret| Zeroizing::new(secret.into()))
        .ok_or_else(invalid)
}
fn read_target(stream: &mut TcpStream, cancel: &AtomicBool) -> Result<String, Diagnostic> {
    stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .map_err(|_| invalid())?;
    stream
        .set_write_timeout(Some(Duration::from_millis(300)))
        .map_err(|_| invalid())?;
    let deadline = Instant::now();
    let mut bytes = Vec::new();
    let mut chunk = [0; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        super::super::protocol::check_cancel(cancel)?;
        if deadline.elapsed() > Duration::from_secs(2) || bytes.len() >= MAX_CALLBACK_BYTES {
            return Err(invalid());
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err(invalid()),
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(_) => return Err(invalid()),
        }
    }
    let line = std::str::from_utf8(&bytes)
        .map_err(|_| invalid())?
        .lines()
        .next()
        .ok_or_else(invalid)?;
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "GET" || parts[2] != "HTTP/1.1" {
        return Err(invalid());
    }
    Ok(parts[1].into())
}
fn respond(stream: &mut TcpStream, accepted: bool) {
    let body = if accepted {
        "Sign-in callback received. Return to RAVEN to check verification."
    } else {
        "Sign-in callback rejected. Return to RAVEN."
    };
    let status = if accepted {
        "200 OK"
    } else {
        "400 Bad Request"
    };
    let _=write!(stream,"HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len());
}
fn load_registration(path: &Path) -> Result<Registration, Diagnostic> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= 8192 =>
        {
            let file = std::fs::File::open(path).map_err(|_| invalid())?;
            if !file.metadata().map_err(|_| invalid())?.is_file() {
                return Err(invalid());
            }
            let mut bytes = Vec::new();
            file.take(8193)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid())?;
            if bytes.len() > 8192 {
                return Err(invalid());
            }
            let value: Registration = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
            if !safe_identifier(&value.host)
                || value
                    .client
                    .as_ref()
                    .is_some_and(|client| !safe_identifier(client))
                || value
                    .subject
                    .as_ref()
                    .is_some_and(|subject| subject.len() > 256)
                || value.client.is_some() != value.subject.is_some()
            {
                return Err(invalid());
            }
            Ok(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let value = Registration {
                host: host_id()?,
                client: None,
                subject: None,
            };
            save_registration(path, &value)?;
            Ok(value)
        }
        _ => Err(Diagnostic::new(
            "AiRegistration",
            "Could not read RAVEN's local ChatGPT registration metadata.",
        )),
    }
}
fn save_registration(path: &Path, value: &Registration) -> Result<(), Diagnostic> {
    let parent = path.parent().ok_or_else(invalid)?;
    std::fs::create_dir_all(parent).map_err(|_| invalid())?;
    if std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(invalid());
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| invalid())?;
    serde_json::to_writer(&mut temporary, value).map_err(|_| invalid())?;
    temporary.as_file_mut().sync_all().map_err(|_| invalid())?;
    temporary.persist(path).map_err(|_| invalid())?;
    Ok(())
}
pub(super) fn visible_models(data: &Value) -> Result<Vec<(String, String)>, Diagnostic> {
    let invalid_catalog = || {
        Diagnostic::new("AiChatGptModels", "ChatGPT model catalog was not recognized. Expected a models/slug list or an object=list data/id list. No analysis was sent.")
    };
    tracing::info!(target:"viewer::ai", models_array=data["models"].is_array(), data_array=data["data"].is_array(), object_list=data["object"]=="list", "ChatGPT model envelope shape");
    let (entries, subscription) = match (data.get("models"), data.get("data")) {
        (Some(entries), None) => (entries.as_array().ok_or_else(invalid_catalog)?, true),
        (None, Some(entries)) if data["object"] == "list" => {
            (entries.as_array().ok_or_else(invalid_catalog)?, false)
        }
        _ => return Err(invalid_catalog()),
    };
    if entries.len() > 512 {
        return Err(invalid_catalog());
    }
    tracing::info!(target:"viewer::ai",format=if subscription {"models/slug"} else {"data/id"},entry_count=entries.len(),"ChatGPT model catalog received");
    let mut models = Vec::new();
    for model in entries {
        if subscription && model["visibility"] != "list" {
            continue;
        }
        if !subscription && model["object"] != "model" {
            return Err(invalid_catalog());
        }
        let slug = model[if subscription { "slug" } else { "id" }]
            .as_str()
            .filter(|slug| safe_identifier(slug))
            .ok_or_else(invalid_catalog)?;
        let label = if subscription {
            model["display_name"]
                .as_str()
                .filter(|label| {
                    !label.is_empty() && label.len() <= 128 && !label.chars().any(char::is_control)
                })
                .ok_or_else(invalid_catalog)?
        } else {
            slug
        };
        if models.iter().any(|(id, _)| id == slug) {
            return Err(invalid_catalog());
        }
        models.push((slug.into(), label.into()));
    }
    if models.is_empty() {
        return Err(Diagnostic::new("AiChatGptModels", "No visible ChatGPT models were returned for this account. Check plan access; no analysis was sent."));
    }
    Ok(models)
}
pub(super) fn verify_identity(
    token: &str,
    jwks: &Value,
    client: &str,
    nonce: &str,
    time: u64,
) -> Result<String, Diagnostic> {
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || token.len() > 32768 {
        return Err(invalid());
    }
    let decode = |part: &str| URL_SAFE_NO_PAD.decode(part).map_err(|_| invalid());
    let header = parse_json(&decode(parts[0])?, 8192)?;
    if header["alg"] != "RS256" || !header["crit"].is_null() {
        return Err(invalid());
    }
    let kid = header["kid"].as_str().ok_or_else(invalid)?;
    let keys = jwks["keys"]
        .as_array()
        .filter(|keys| keys.len() <= 64)
        .ok_or_else(invalid)?;
    let key = keys
        .iter()
        .find(|key| {
            key["kid"] == kid
                && key["kty"] == "RSA"
                && (key["use"].is_null() || key["use"] == "sig")
                && (key["alg"].is_null() || key["alg"] == "RS256")
        })
        .ok_or_else(invalid)?;
    let modulus = decode(key["n"].as_str().ok_or_else(invalid)?)?;
    let exponent = decode(key["e"].as_str().ok_or_else(invalid)?)?;
    signature::RsaPublicKeyComponents {
        n: &modulus,
        e: &exponent,
    }
    .verify(
        &signature::RSA_PKCS1_2048_8192_SHA256,
        format!("{}.{}", parts[0], parts[1]).as_bytes(),
        &decode(parts[2])?,
    )
    .map_err(|_| invalid())?;
    let claims = parse_json(&decode(parts[1])?, 16384)?;
    let audience = &claims["aud"];
    if claims["iss"] != ISSUER
        || !(audience == client
            || audience
                .as_array()
                .is_some_and(|values| values.iter().any(|value| value == client)))
        || claims["nonce"] != nonce
        || claims["exp"].as_u64().is_none_or(|expiry| expiry <= time)
        || claims["nbf"]
            .as_u64()
            .is_some_and(|not_before| not_before > time + 30)
        || claims["iat"]
            .as_u64()
            .is_none_or(|issued| issued > time + 30)
        || (audience.as_array().is_some_and(|values| values.len() > 1) && claims["azp"] != client)
    {
        return Err(invalid());
    }
    claims["sub"]
        .as_str()
        .filter(|subject| {
            !subject.is_empty() && subject.len() <= 256 && !subject.chars().any(char::is_control)
        })
        .map(str::to_owned)
        .ok_or_else(invalid)
}
