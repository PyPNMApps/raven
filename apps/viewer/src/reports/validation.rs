/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Validation and local resolution. Diagnostics never reflect provider-controlled strings.
use super::{
    context::Context,
    failure, markdown,
    schema::{Action, Block, Report},
    ReportResult, MAX_BLOCKS,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}
fn text(value: &str, limit: usize) -> ReportResult<()> {
    if value.len() > limit
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(failure(
            "ReportText",
            "Report text is too long or contains unsupported control characters.",
        ));
    }
    Ok(())
}
fn collect(blocks: &[Block], depth: usize, ids: &mut BTreeSet<String>) -> ReportResult<()> {
    if depth > 4 {
        return Err(failure(
            "ReportDepth",
            "Report sections exceed four nesting levels.",
        ));
    }
    for block in blocks {
        if !identifier(block.id()) || !ids.insert(block.id().into()) {
            return Err(failure(
                "ReportId",
                "Block IDs must be unique bounded identifiers.",
            ));
        }
        if ids.len() > MAX_BLOCKS {
            return Err(failure("ReportLimit", "Report exceeds 48 total blocks."));
        }
        if let Block::Expandable { blocks, .. } = block {
            collect(blocks, depth + 1, ids)?;
        }
    }
    Ok(())
}
fn table(columns: &[String], rows: &[Vec<Option<String>>]) -> ReportResult<()> {
    if columns.is_empty() || columns.len() > 12 || rows.len() > 64 {
        return Err(failure(
            "ReportTable",
            "Table exceeds 12 columns or 64 rows, or has no columns.",
        ));
    }
    for column in columns {
        text(column, 128)?;
    }
    for row in rows {
        if row.len() != columns.len() {
            return Err(failure(
                "ReportTable",
                "Table rows must match the declared columns.",
            ));
        }
        for cell in row.iter().flatten() {
            text(cell, 256)?;
        }
    }
    Ok(())
}
/// Validates a typed report and resolves its ordered blocks against a local generation.
/// # Arguments
/// * `report`, `context` - Untrusted definition and trusted current local registry.
/// # Returns
/// Bounded presentation JSON with locally attached provenance and no executable paths.
/// # Errors
/// Rejects invalid schema/IDs, excessive complexity, unsafe Markdown or unresolved evidence/assets.
pub fn validate(report: &Report, context: &Context) -> ReportResult<Value> {
    if report.schema_version != "1.0" || !identifier(&report.report_id) || report.blocks.is_empty()
    {
        return Err(failure(
            "ReportSchema",
            "Expected a nonempty schema 1.0 report with a valid ID.",
        ));
    }
    text(&report.title, 200)?;
    if report.source.context_id != context.id || report.source.generation != context.generation {
        return Err(failure(
            "ReportGeneration",
            "Report does not belong to the active capture or simulation generation.",
        ));
    }
    let mut ids = BTreeSet::new();
    collect(&report.blocks, 1, &mut ids)?;
    let blocks = resolve(&report.blocks, context, &ids)?;
    fn render_cost(blocks: &[Value]) -> (usize, usize) {
        blocks.iter().fold((0, 0), |(cells, points), block| {
            let table_cells = block["rows"].as_array().map_or(0, |rows| {
                rows.len() * block["columns"].as_array().map_or(0, Vec::len)
            });
            let chart_points = block["viewport"]["points"].as_array().map_or(0, Vec::len);
            let nested = block["blocks"]
                .as_array()
                .map_or((0, 0), |blocks| render_cost(blocks));
            (
                cells + table_cells + nested.0,
                points + chart_points + nested.1,
            )
        })
    }
    let (cells, points) = render_cost(&blocks);
    if cells > 2048 || points > 4096 {
        return Err(failure(
            "ReportLimit",
            "Report exceeds 2048 table cells or 4096 plotted display points across all sections.",
        ));
    }
    let result = json!({"schema_version":"1.0","report_id":report.report_id,"title":report.title,
        "context_id":context.id,"generation":context.generation.to_string(),"classification":context.classification,"blocks":blocks});
    if result.to_string().len() > 524_288 {
        return Err(failure(
            "ReportLimit",
            "Resolved report exceeds its 512 KiB presentation budget.",
        ));
    }
    Ok(result)
}
fn resolve(
    blocks: &[Block],
    context: &Context,
    ids: &BTreeSet<String>,
) -> ReportResult<Vec<Value>> {
    blocks.iter().map(|block| {
        let mut result = match block {
            Block::Markdown {content, ..} => {
                text(content, 16_384)?;
                let markdown = markdown::validate(content, ids)?;
                json!({"type":"markdown","content":content,"display_content":markdown.display_content,"links":markdown.links,"navigation":markdown.navigation})
            }
            Block::Chart {dataset_id, title, ..} => {
                text(title, 200)?;
                if !identifier(dataset_id) { return Err(failure("ReportDataset", "Chart dataset ID is invalid.")); }
                match context.charts.get(dataset_id) {
                    Some(data) if data.generation == context.generation => {
                        if data.viewport.points.len() > 1024 || data.viewport.points.iter().any(|point| point.value.is_some_and(|value| !value.is_finite())) {
                            return Err(failure("ReportDataset", "Chart display data exceeds bounds or contains invalid values."));
                        }
                        json!({"type":"chart","chart_type":"rxmer_frequency","title":title,"dataset_id":dataset_id,
                            "viewport":data.viewport,"minimum":0,"maximum":64,"unit":"dB","axis_label":"RxMER","classification":data.classification})
                    }
                    _ => json!({"type":"unavailable","title":title,"reason":"This chart dataset is unavailable for the current generation."}),
                }
            }
            Block::Table {dataset_id, title, ..} => {
                text(title, 200)?;
                if !identifier(dataset_id) { return Err(failure("ReportDataset", "Table dataset ID is invalid.")); }
                match context.tables.get(dataset_id) {
                    Some(data) if data.generation == context.generation => {
                        table(&data.columns, &data.rows)?;
                        json!({"type":"table","title":title,"columns":data.columns,"rows":data.rows,"classification":data.classification})
                    }
                    _ => json!({"type":"unavailable","title":title,"reason":"This table dataset is unavailable for the current generation."}),
                }
            }
            Block::StaticTable {title, columns, rows, ..} => {
                text(title, 200)?;
                let rows: Vec<_> = rows.iter().map(|row| row.iter().cloned().map(Some).collect()).collect();
                table(columns, &rows)?;
                json!({"type":"table","title":title,"columns":columns,"rows":rows,"classification":"explanatory_not_measurement"})
            }
            Block::Image {asset_id, caption, ..} => {
                text(caption, 512)?;
                let (dark,light) = match asset_id.as_str() {
                    "raven-icon" => ("qrc:/branding/icon-dark.png","qrc:/branding/icon-light.png"),
                    "report-jpeg" => ("qrc:/reports/example.jpg","qrc:/reports/example.jpg"),
                    "report-webp" => ("qrc:/reports/example.webp","qrc:/reports/example.webp"),
                    _ => return Err(failure("ReportAsset", "Image asset ID is not registered by this application.")),
                };
                json!({"type":"image","source_dark":dark,"source_light":light,"caption":caption,"classification":"application_asset"})
            }
            Block::Evidence {evidence_ids, label, action, ..} => {
                text(label, 200)?;
                if evidence_ids.is_empty() || evidence_ids.len() > 16 { return Err(failure("ReportEvidence", "Evidence block requires one to sixteen unique references.")); }
                let mut seen = BTreeSet::new();
                let mut values = Vec::new();
                for id in evidence_ids {
                    let value = context.evidence.get(id).filter(|value| value.generation == context.generation)
                        .ok_or_else(|| failure("ReportEvidence", "Evidence reference is unknown or belongs to another generation."))?;
                    if !seen.insert(id) { return Err(failure("ReportEvidence", "Evidence references must not be duplicated.")); }
                    text(&value.label, 200)?; text(&value.unit, 64)?; text(&value.scope, 128)?;
                    if let Some(value) = &value.value {text(value, 256)?;}
                    values.push(value);
                }
                let navigation = match action {
                    Some(Action::OpenRxmer {target_id}) => {
                        if context.charts.get(target_id).is_none_or(|data| data.generation != context.generation) {
                            return Err(failure("ReportAction", "Navigation target is not an active registered RxMER dataset."));
                        }
                        json!({"enabled":false,"reason":"Navigation to the source workspace is not connected for this report. Review the evidence here."})
                    }
                    None => Value::Null,
                };
                json!({"type":"evidence","label":label,"values":values,"action":navigation})
            }
            Block::Expandable {title, blocks, ..} => {
                text(title, 200)?;
                json!({"type":"expandable","title":title,"blocks":resolve(blocks, context, ids)?})
            }
        };
        result["id"] = json!(block.id());
        Ok(result)
    }).collect()
}
