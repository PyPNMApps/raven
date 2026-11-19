/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Offline definitions and a sanitized real PNM RxMER capture, plus labeled synthetic tables.
use super::{
    context::{Chart, Classification, Context, EvidenceValue, Table},
    failure, parse, ReportResult,
};
use crate::Capture;
use serde_json::{json, Value};

/// Lists deterministic offline examples without reading files or contacting providers.
/// # Returns
/// Stable IDs and labels for a finite example selector.
pub fn catalog() -> Value {
    json!([{"id":"markdown","label":"Markdown basics"},{"id":"engineering","label":"Engineering report"},
        {"id":"rich","label":"Rich report"},{"id":"invalid","label":"Rejected report example"}])
}
/// Builds a trusted isolated demo context using the normal bounded capture adapter.
/// # Returns
/// Measured RxMER points, one evidence reference and a clearly synthetic profile table.
/// # Errors
/// Propagates failure to parse the bundled sanitized capture.
pub fn context() -> ReportResult<Context> {
    let mut context = Context::new("offline-report-demo", 1, Classification::Demonstration);
    let capture = Capture::from_bytes(
        include_bytes!("../../../../resources/reports/rxmer.bin").to_vec(),
        "Bundled RxMER demonstration".into(),
    )?;
    let metadata = capture.chart_metadata()?;
    context.charts.insert(
        "rxmer.demo".into(),
        Chart {
            classification: Classification::Measured,
            viewport: capture.chart_range(0, metadata.sample_count)?,
            generation: 1,
        },
    );
    context.tables.insert(
        "profiles.demo".into(),
        Table {
            columns: vec![
                "Profile".into(),
                "Mean LDPC iterations".into(),
                "Post-FEC BER (ratio)".into(),
            ],
            rows: vec![
                vec![
                    Some("A (synthetic)".into()),
                    Some("2.5".into()),
                    Some("0".into()),
                ],
                vec![Some("B (synthetic)".into()), None, Some("0.001".into())],
            ],
            classification: Classification::Demonstration,
            generation: 1,
        },
    );
    context.evidence.insert(
        "rxmer.samples".into(),
        EvidenceValue {
            id: "rxmer.samples".into(),
            label: "Capture sample count".into(),
            value: Some(metadata.sample_count.to_string()),
            unit: "subcarriers".into(),
            scope: "bundled capture".into(),
            classification: Classification::Measured,
            generation: 1,
        },
    );
    Ok(context)
}
/// Reads a bundled definition by allowlisted ID, retaining its original order.
/// # Arguments
/// * `id` - One of the IDs in the example catalog.
/// # Returns
/// Untrusted example JSON, intended to pass through the normal report parser.
/// # Errors
/// Rejects unrecognized example IDs without accepting resource paths.
pub fn definition(id: &str) -> ReportResult<&'static str> {
    match id {
        "markdown" => Ok(include_str!("../../../../resources/reports/markdown.json")),
        "engineering" => Ok(include_str!(
            "../../../../resources/reports/engineering.json"
        )),
        "rich" => Ok(include_str!("../../../../resources/reports/rich.json")),
        "invalid" => Ok(include_str!("../../../../resources/reports/invalid.json")),
        _ => Err(failure("ReportFixture", "Unknown offline report example.")),
    }
}
/// Resolves one offline example through the same validation used for future providers.
/// # Arguments
/// * `id` - Allowlisted deterministic example ID.
/// # Returns
/// Approved presentation, or the expected validation error for the invalid example.
/// # Errors
/// Rejects invalid example definitions or corrupt bundled measurements.
pub fn render(id: &str) -> ReportResult<Value> {
    parse(definition(id)?, &context()?)
}
