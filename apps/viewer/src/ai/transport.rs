/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! OpenAI-only bounded HTTPS, no redirects, fallback, remote tools or error reflection.
use super::{
    protocol::{check_cancel, invalid, parse_json, ConnectionProbe, Transport},
    Provider,
};
use crate::Diagnostic;
use std::{io::Read, sync::atomic::AtomicBool, time::Duration};

/// One-shot transport using the desktop's existing TLS dependency.
pub struct HttpsTransport;
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(60))
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(8))
        .build()
}
fn response_bytes(
    result: Result<ureq::Response, ureq::Error>,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Diagnostic> {
    response_body(result, cancel, false, "")
}
fn response_body(
    result: Result<ureq::Response, ureq::Error>,
    cancel: &AtomicBool,
    streaming: bool,
    key: &str,
) -> Result<Vec<u8>, Diagnostic> {
    check_cancel(cancel)?;
    let response=match result {
        Ok(response) if response.status()==200=>response,
        Ok(_)=>return Err(Diagnostic::new("AiRedirect","Unexpected HTTP status or redirect; request was not retried.")),
        Err(ureq::Error::Status(401,_)) if streaming=>return Err(Diagnostic::new("AiAuthentication","ChatGPT rejected this sign-in. It may have expired or been revoked. Connect again; no request was retried.")),
        Err(ureq::Error::Status(status,_))=>return Err(status_error(status)),
        Err(_)=>return Err(Diagnostic::new("AiNetwork","Could not reach the AI service or the request timed out. Check your network and try explicitly again.")),
    };
    if streaming {
        return super::chatgpt::responses::read_stream(response.into_reader(), key, cancel);
    }
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        check_cancel(cancel)?;
        let count = reader.read(&mut chunk).map_err(|_| {
            Diagnostic::new(
                "AiNetwork",
                "The AI response could not be read or timed out.",
            )
        })?;
        if count == 0 {
            break;
        }
        if bytes.len() + count > 262144 {
            return Err(Diagnostic::new(
                "AiLimit",
                "The AI response exceeded the 256 KiB limit.",
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    check_cancel(cancel)?;
    Ok(bytes)
}
/// Maps HTTP statuses without exposing provider bodies or credential-bearing headers.
/// # Arguments
/// * `status` - Returned HTTP status only.
/// # Returns
/// Stable, nonsecret error category and user-actionable message.
pub fn status_error(status: u16) -> Diagnostic {
    let (code,message)=match status {
        401=>("AiAuthentication","The API key was rejected: it may be expired, revoked, or invalid. Open Key & model to replace it. No request was retried."),
        403=>("AiAccess","The key does not have access to this model or project."),
        404=>("AiModel","The selected model was not found or is not accessible."),
        429=>("AiQuota","The provider reported a rate or quota limit. Check API usage and billing."),
        400|422=>("AiModelOptions","The model rejected this structured-output request. Select a compatible model and create a new preview."),
        500..=599=>("AiService","The provider is temporarily unavailable. No automatic retry was made."),
        _=>("AiHttp","The provider returned an unexpected status. No report was accepted."),
    };
    Diagnostic::new(code, message)
}
impl Transport for HttpsTransport {
    /// Posts the frozen body to the approved OpenAI endpoint with normal TLS trust.
    /// # Arguments
    /// * `endpoint`, `key`, `body` - Fixed destination, authorization and exact bytes.
    /// * `cancel` - Cooperative cancellation flag checked before and after I/O.
    /// # Returns
    /// Nonstreaming bytes within 256 KiB, or a bounded completed SSE event.
    /// # Errors
    /// Sanitized cancellation, endpoint, HTTP, timeout or read-limit failure.
    fn post(
        &self,
        endpoint: &str,
        key: &str,
        body: &str,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Diagnostic> {
        if endpoint != Provider::OpenAi.endpoint() && endpoint != Provider::ChatGpt.endpoint() {
            return Err(invalid());
        }
        check_cancel(cancel)?;
        let authorization = zeroize::Zeroizing::new(format!("Bearer {key}"));
        response_body(
            agent()
                .post(endpoint)
                .set("Authorization", &authorization)
                .set("Content-Type", "application/json")
                .send_bytes(body.as_bytes()),
            cancel,
            endpoint == Provider::ChatGpt.endpoint(),
            key,
        )
    }
}
impl ConnectionProbe {
    /// Tests model metadata access with no inference prompt, PNM data or transcript.
    /// # Arguments
    /// * `cancel` - Owned operation cancellation flag.
    /// # Returns
    /// Success means the key can retrieve this model's metadata, not inference support.
    /// # Errors
    /// Sanitized authentication, model-access, quota, timeout or malformed-body failure.
    pub fn execute(self, cancel: &AtomicBool) -> Result<(), Diagnostic> {
        let mut url =
            url::Url::parse("https://api.openai.com/v1/models/").map_err(|_| invalid())?;
        url.path_segments_mut()
            .map_err(|_| invalid())?
            .pop_if_empty()
            .push(&self.model);
        let authorization = zeroize::Zeroizing::new(format!("Bearer {}", self.key.as_str()));
        check_cancel(cancel)?;
        let bytes = response_bytes(
            agent()
                .get(url.as_str())
                .set("Authorization", &authorization)
                .call(),
            cancel,
        )?;
        let response = parse_json(&bytes, 131072)?;
        if response["id"] != self.model {
            return Err(invalid());
        }
        Ok(())
    }
}
