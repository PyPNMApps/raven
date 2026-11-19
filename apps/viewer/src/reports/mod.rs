/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Native engineering reports: closed definitions, trusted evidence and bounded presentation.
pub mod adapter;
pub mod context;
pub mod fixtures;
mod markdown;
pub mod schema;
mod validation;
use crate::Diagnostic;
use serde_json::Value;
pub use validation::validate;
pub type ReportResult<T> = Result<T, Diagnostic>;
pub const MAX_REPORT_BYTES: usize = 131_072;
pub const MAX_BLOCKS: usize = 48;
fn failure(code: &str, message: &str) -> Diagnostic {
    tracing::warn!(target:"viewer::reports",code,reason=message,"Engineering report rejected");
    Diagnostic::new(code, message)
}
/// Parses and resolves an untrusted definition against the current local owner.
/// # Arguments
/// * `json`, `context` - Bounded report JSON and trusted active datasets/evidence.
/// # Returns
/// A sanitized presentation model. No provider JSON reaches QML directly.
/// # Errors
/// Rejects malformed/duplicate JSON, stale ownership, unsupported blocks or unsafe content.
pub fn parse(json: &str, context: &context::Context) -> ReportResult<Value> {
    let value =
        crate::ai::protocol::parse_json(json.as_bytes(), MAX_REPORT_BYTES).map_err(|_| {
            failure(
                "ReportJson",
                "Report JSON is malformed, duplicated or outside byte/depth limits.",
            )
        })?;
    let report: schema::Report = serde_json::from_value(value).map_err(|_| {
        failure(
            "ReportSchema",
            "Report fields or block types do not match schema 1.0.",
        )
    })?;
    validate(&report, context)
}
