/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Bounded SSE acceptance. Only a terminal completed text response can reach report validation.
use super::super::protocol::analysis_error;
use super::*;

/// Each event is bounded independently; repeated deltas never accumulate as a report.
const MAX_EVENT_BYTES: usize = 1024 * 1024;
const MAX_STREAM_BYTES: usize = 16 * 1024 * 1024;
const MAX_STREAM_EVENTS: usize = 32768;

/// Reads a Responses SSE stream while retaining only its completed terminal event.
/// # Arguments
/// * `reader` - HTTP body or a synthetic incremental reader.
/// * `key` - Private credential used only to reject reflected secrets; empty for offline parsing.
/// * `cancel` - Cooperative cancellation checked on every read and event.
/// # Returns
/// A bounded canonical completed-event frame; no partial report is accepted.
/// # Errors
/// Rejects read failures, incomplete/failed streams, duplicate completion, secret
/// reflection, excessive event/total bytes, too many events and cancellation.
pub(in crate::ai) fn read_stream(
    mut reader: impl Read,
    key: &str,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, Diagnostic> {
    let mut frame = Vec::new();
    let mut completed = None;
    let mut total = 0usize;
    let mut events = 0usize;
    let mut chunk = [0u8; 4096];
    loop {
        super::super::protocol::check_cancel(cancel)?;
        let count = reader.read(&mut chunk).map_err(|_| {
            Diagnostic::new(
                "AiNetwork",
                "The ChatGPT event stream was interrupted or timed out.",
            )
        })?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count).ok_or_else(invalid)?;
        if total > MAX_STREAM_BYTES {
            return Err(Diagnostic::new(
                "AiStreamLimit",
                "ChatGPT event stream exceeded its 16 MiB transfer budget. No report was accepted.",
            ));
        }
        for byte in &chunk[..count] {
            // CRLF is normalized only when recognizing complete lines, preserving JSON bytes.
            frame.push(*byte);
            if frame.len() > MAX_EVENT_BYTES {
                return Err(Diagnostic::new(
                    "AiStreamLimit",
                    "A ChatGPT event exceeded its 1 MiB budget. No report was accepted.",
                ));
            }
            if frame.ends_with(b"\n\n") || frame.ends_with(b"\r\n\r\n") {
                super::super::protocol::check_cancel(cancel)?;
                events += 1;
                if events > MAX_STREAM_EVENTS {
                    return Err(Diagnostic::new(
                        "AiStreamLimit",
                        "ChatGPT event count exceeded its budget. No report was accepted.",
                    ));
                }
                if !key.is_empty()
                    && std::str::from_utf8(&frame)
                        .map_err(|_| invalid())?
                        .contains(key)
                {
                    return Err(Diagnostic::new(
                        "AiSecret",
                        "The ChatGPT stream contained credential material and was rejected.",
                    ));
                }
                if let Some(value) = parse_event(&frame)? {
                    if completed.is_some() {
                        return Err(analysis_error(
                            "AiStreamFormat",
                            "an event followed terminal completion",
                        ));
                    }
                    if value["type"] == "response.completed" {
                        completed = Some(format!("data: {value}\n\n").into_bytes());
                    }
                }
                frame.clear();
            }
        }
    }
    super::super::protocol::check_cancel(cancel)?;
    if !frame.is_empty() {
        return Err(analysis_error(
            "AiStreamFormat",
            "stream ended inside an event",
        ));
    }
    tracing::info!(target:"viewer::ai",stream_bytes=total,event_count=events,"ChatGPT stream consumed");
    completed.ok_or_else(|| {
        Diagnostic::new(
            "AiStreamIncomplete",
            "ChatGPT stream ended without a completed response. No report was accepted.",
        )
    })
}
fn parse_event(frame: &[u8]) -> Result<Option<Value>, Diagnostic> {
    let text = std::str::from_utf8(frame)
        .map_err(|_| analysis_error("AiStreamFormat", "stream event is not UTF-8"))?;
    let mut event = None;
    let mut data = Vec::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            if event.replace(value.trim()).is_some() {
                return Err(analysis_error(
                    "AiStreamFormat",
                    "stream frame repeats its event label",
                ));
            }
        }
        if let Some(value) = line.strip_prefix("data:") {
            data.push(value.strip_prefix(' ').unwrap_or(value));
        }
    }
    if data.is_empty() {
        return Ok(None);
    }
    let value = parse_json(data.join("\n").as_bytes(), MAX_EVENT_BYTES).map_err(|_| {
        analysis_error(
            "AiStreamFormat",
            "stream event is invalid JSON or exceeds JSON bounds",
        )
    })?;
    let kind = value["type"]
        .as_str()
        .ok_or_else(|| analysis_error("AiStreamFormat", "stream event has no type"))?;
    if event.is_some_and(|event| event != kind) {
        return Err(analysis_error(
            "AiStreamFormat",
            "stream event label and type disagree",
        ));
    }
    if matches!(kind, "error" | "response.failed" | "response.incomplete") {
        return Err(Diagnostic::new("AiChatGptResponse","ChatGPT could not complete this analysis. Check plan usage and access. No report was accepted or request retried."));
    }
    Ok(Some(value))
}

/// Converts a completed Responses event into the shared advisory-report envelope.
/// # Arguments
/// * `bytes` - Full synthetic stream or the retained terminal network event.
/// # Returns
/// Message, model and usage for the existing immutable-evidence validator.
/// # Errors
/// Rejects incomplete streams, tools, refusals and unsupported output structures.
pub(in crate::ai) fn envelope(bytes: &[u8]) -> Result<Value, Diagnostic> {
    let terminal = read_stream(std::io::Cursor::new(bytes), "", &AtomicBool::new(false))?;
    let event = parse_event(&terminal)?.ok_or_else(invalid)?;
    let completed = Some(event["response"].clone());
    let response = completed
        .filter(|value| {
            value["status"] == "completed"
                && value["error"].is_null()
                && value["incomplete_details"].is_null()
        })
        .ok_or_else(|| {
            analysis_error(
                "AiEnvelope",
                "terminal response is not completed successfully",
            )
        })?;
    let output = response["output"]
        .as_array()
        .ok_or_else(|| analysis_error("AiEnvelope", "terminal response has no output array"))?;
    let mut content = None;
    for item in output {
        match item["type"].as_str() {
            Some("reasoning") => {}
            Some("message") if item["role"] == "assistant" && item["status"] == "completed" => {
                let parts = item["content"].as_array().ok_or_else(|| {
                    analysis_error("AiEnvelope", "assistant output has no content array")
                })?;
                if parts.len() != 1
                    || parts[0]["type"] != "output_text"
                    || parts[0]["annotations"]
                        .as_array()
                        .is_some_and(|values| !values.is_empty())
                    || content.is_some()
                {
                    return Err(analysis_error("AiEnvelope", "assistant output contains refusal, annotations, multiple messages or unsupported content"));
                }
                content = Some(
                    parts[0]["text"]
                        .as_str()
                        .ok_or_else(|| {
                            analysis_error("AiEnvelope", "assistant output text is missing")
                        })?
                        .to_owned(),
                );
            }
            _ => {
                return Err(analysis_error(
                    "AiEnvelope",
                    "response contains a tool or unsupported output item",
                ))
            }
        }
    }
    Ok(
        json!({"model":response["model"],"choices":[{"finish_reason":"stop","message":{"content":content.ok_or_else(|| analysis_error("AiEnvelope", "response has no completed assistant text"))?}}],"usage":{"prompt_tokens":response["usage"]["input_tokens"],"completion_tokens":response["usage"]["output_tokens"],"total_tokens":response["usage"]["total_tokens"]}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_event_and_transfer_budgets_are_independent_and_bounded() {
        let cancel = AtomicBool::new(false);
        let oversized = std::io::repeat(b'x').take(MAX_EVENT_BYTES as u64 + 1);
        let error = read_stream(oversized, "", &cancel).unwrap_err();
        assert_eq!(error.code, "AiStreamLimit");
        assert!(error.message.contains("1 MiB"));
        let comment = format!(":{}\n\n", "x".repeat(700_000));
        let transfer = comment.repeat(25).into_bytes();
        let error = read_stream(std::io::Cursor::new(transfer), "", &cancel).unwrap_err();
        assert_eq!(error.code, "AiStreamLimit");
        assert!(error.message.contains("16 MiB"));
        let events = ":\n\n".repeat(MAX_STREAM_EVENTS + 1);
        let error =
            read_stream(std::io::Cursor::new(events.into_bytes()), "", &cancel).unwrap_err();
        assert_eq!(error.code, "AiStreamLimit");
        assert!(error.message.contains("count"));
    }
}
