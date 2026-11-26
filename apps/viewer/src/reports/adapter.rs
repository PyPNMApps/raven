/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Existing Rust-owned analysis snapshots and accepted narrative reports into native reports.
use super::markdown::literal as escape;
use super::{
    context::{Chart, Classification, Context, EvidenceValue, Table},
    failure,
    schema::{Block, ChartType, Report, Source},
    validate, ReportResult,
};
use crate::{ai::evidence::Evidence, Capture};
use serde_json::{json, Value};

fn definition(title: &str, context: &Context, blocks: Vec<Block>) -> Report {
    Report {
        schema_version: "1.0".into(),
        report_id: "local-engineering-report".into(),
        title: title.into(),
        source: Source {
            context_id: context.id.clone(),
            generation: context.generation,
        },
        blocks,
    }
}
fn markdown(id: &str, content: &str) -> Block {
    Block::Markdown {
        id: id.into(),
        content: content.into(),
    }
}
fn scalars(value: &Value, prefix: &str, rows: &mut Vec<Vec<Option<String>>>) {
    if rows.len() >= 64 {
        return;
    }
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if ["points", "iq", "samples", "mac_address", "filename", "path"]
                    .contains(&key.as_str())
                {
                    continue;
                }
                let label = if prefix.is_empty() {
                    key.replace('_', " ")
                } else {
                    format!("{prefix} · {}", key.replace('_', " "))
                };
                scalars(value, &label, rows);
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().take(16).enumerate() {
                scalars(value, &format!("{prefix} · {}", index + 1), rows);
            }
        }
        _ => rows.push(vec![
            Some(prefix.to_owned()),
            value.as_str().map(str::to_owned).or_else(|| {
                if value.is_null() {
                    None
                } else {
                    Some(value.to_string())
                }
            }),
        ]),
    }
}
/// Builds a bounded local report for any existing PNM analysis/test snapshot.
/// # Arguments
/// * `title`, `owner`, `generation` - Existing Rust backend's identity and lifecycle.
/// * `snapshot`, `classification` - Rust-owned result/settings JSON, never a provider response.
/// # Returns
/// Local summary table and exact scalar evidence; no AI call or new calculation.
/// # Errors
/// Rejects invalid table shape or context metadata. Up to 64 fields are explicitly sampled.
pub fn from_snapshot(
    title: &str,
    owner: &str,
    generation: u64,
    snapshot: &Value,
    classification: Classification,
) -> ReportResult<Value> {
    build_snapshot(title, owner, generation, snapshot, classification, None)
}
fn build_snapshot(
    title: &str,
    owner: &str,
    generation: u64,
    snapshot: &Value,
    classification: Classification,
    chart: Option<Chart>,
) -> ReportResult<Value> {
    let mut context = Context::new(
        &format!(
            "{owner}-{}",
            crate::ai::protocol::digest(snapshot.to_string().as_bytes())
        ),
        generation,
        classification.clone(),
    );
    let mut rows = Vec::new();
    scalars(snapshot, "", &mut rows);
    context.tables.insert(
        "analysis.summary".into(),
        Table {
            columns: vec!["Field / original unit".into(), "Rust-owned value".into()],
            rows: rows.clone(),
            classification: classification.clone(),
            generation,
        },
    );
    for (index, row) in rows.iter().take(16).enumerate() {
        let id = format!("analysis.field-{index}");
        context.evidence.insert(
            id.clone(),
            EvidenceValue {
                id,
                label: row[0].clone().unwrap_or_default(),
                value: row[1].clone(),
                unit: "as labeled by source".into(),
                scope: title.into(),
                classification: classification.clone(),
                generation,
            },
        );
    }
    let mut blocks = vec![markdown("summary","## Local engineering report\nThis view presents existing Rust-owned results. No AI service was contacted and no new measurements were calculated. The table includes at most 64 scalar fields; use the existing full JSON export for complete data."),
        Block::Table{id:"results".into(),dataset_id:"analysis.summary".into(),title:title.into()}];
    if !context.evidence.is_empty() {
        blocks.push(Block::Evidence {
            id: "evidence".into(),
            evidence_ids: context.evidence.keys().cloned().collect(),
            label: "Source evidence".into(),
            action: None,
        });
    }
    if let Some(chart) = chart {
        context.charts.insert("rxmer.active".into(), chart);
        blocks.insert(
            1,
            Block::Chart {
                id: "rxmer-plot".into(),
                chart_type: ChartType::RxmerFrequency,
                dataset_id: "rxmer.active".into(),
                title: "Measured RxMER".into(),
            },
        );
    }
    let mut result = validate(&definition(title, &context, blocks), &context)?;
    result["prepared_question"] = json!(format!("Review the supplied {title} results in plain language. Separate observations from hypotheses, preserve all original units/scopes and explain unavailable evidence. Do not calculate new values or infer a physical cause without evidence."));
    Ok(result)
}
/// Uses the existing capture and chart owners for every single-capture measurement.
/// # Arguments
/// * `capture`, `generation` - Current immutable capture and its backend generation.
/// # Returns
/// Local result report, including a real bounded chart when this is RxMER.
/// # Errors
/// Propagates existing measurement parsing, metadata and report validation failures.
pub fn from_capture(capture: &Capture, generation: u64) -> ReportResult<Value> {
    let metadata = capture.chart_metadata()?;
    let snapshot = serde_json::to_value(&metadata)
        .map_err(|_| failure("ReportSnapshot", "Could not format local capture metadata."))?;
    let chart = if metadata.axis_label == "RxMER" {
        Some(Chart {
            classification: Classification::Measured,
            viewport: capture.chart_range(0, metadata.sample_count)?,
            generation,
        })
    } else {
        None
    };
    let mut report = build_snapshot(
        metadata.title,
        "single-capture",
        generation,
        &snapshot,
        Classification::Measured,
        chart,
    )?;
    if metadata.title == "Upstream OFDMA RxMER — CMTS" {
        report["prepared_question_id"] = json!("cmts-upstream-rxmer");
        report["prepared_question_revision"] = json!(1);
        report["prepared_question"] = json!("Review the supplied CMTS upstream OFDMA probe RxMER. Describe the source CCAP/interface, measured carrier values and unavailable positions. Preserve supplied units and scope. Do not apply downstream capacity thresholds, infer a physical cause or invent missing evidence.");
    }
    Ok(report)
}
/// Adapts only the existing accepted AI narrative/evidence contract to native blocks.
/// # Arguments
/// * `result` - Accepted local AI result, with strict evidence and report validation repeated.
/// # Returns
/// Provider-independent native report; the caller retains its existing fallback on error.
/// # Errors
/// Rejects malformed narrative/evidence or unsafe Markdown. No request is made or repaired.
pub fn from_ai_result(result: &Value) -> ReportResult<Value> {
    let evidence = Evidence::parse(&result["evidence"].to_string())?;
    let narrative = crate::ai::protocol::validate_report(&result["report"].to_string(), &evidence)?;
    let mut context = Context::new(&evidence.run_id, 1, Classification::Simulation);
    for metric in &evidence.metrics {
        context.evidence.insert(
            metric.id.clone(),
            EvidenceValue {
                id: metric.id.clone(),
                label: metric.id.clone(),
                value: metric.value.clone(),
                unit: metric.unit.clone(),
                scope: metric.scope.clone(),
                classification: Classification::Simulation,
                generation: 1,
            },
        );
    }
    let mut blocks = vec![markdown(
        "summary",
        &format!(
            "## AI interpretation\n{}",
            escape(narrative["summary"].as_str().unwrap_or_default())
        ),
    )];
    for (index, claim) in narrative["claims"].as_array().unwrap().iter().enumerate() {
        blocks.push(markdown(
            &format!("claim-{index}"),
            &format!(
                "### {}\n{}",
                claim["kind"].as_str().unwrap_or("Claim"),
                escape(claim["text"].as_str().unwrap_or_default())
            ),
        ));
        let refs: Vec<_> = claim["references"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|reference| reference["metric_id"].as_str().map(str::to_owned))
            .collect();
        if !refs.is_empty() {
            blocks.push(Block::Evidence {
                id: format!("refs-{index}"),
                evidence_ids: refs,
                label: "Simulation evidence · unchanged Rust values".into(),
                action: None,
            });
        }
    }
    for (key, label) in [
        ("limitations", "Limitations"),
        ("suggested_checks", "Suggested checks"),
    ] {
        let text = narrative[key]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(|text| format!("- {}", escape(text)))
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(markdown(key, &format!("## {label}\n{text}")));
    }
    let mut model = validate(
        &definition("OFDM AI engineering report", &context, blocks),
        &context,
    )?;
    model["interpretation"] = json!(if result["demo"] == true {
        "local_demo"
    } else {
        "ai_interpretation"
    });
    Ok(model)
}
