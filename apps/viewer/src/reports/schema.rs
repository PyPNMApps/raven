/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Provider-independent untrusted report definitions. No paths, code or numerical datasets.
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: String,
    pub report_id: String,
    pub title: String,
    pub source: Source,
    pub blocks: Vec<Block>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub context_id: String,
    pub generation: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Block {
    Markdown {
        id: String,
        content: String,
    },
    Chart {
        id: String,
        chart_type: ChartType,
        dataset_id: String,
        title: String,
    },
    Table {
        id: String,
        dataset_id: String,
        title: String,
    },
    StaticTable {
        id: String,
        title: String,
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Image {
        id: String,
        asset_id: String,
        caption: String,
    },
    Evidence {
        id: String,
        evidence_ids: Vec<String>,
        label: String,
        action: Option<Action>,
    },
    Expandable {
        id: String,
        title: String,
        blocks: Vec<Block>,
    },
}
impl Block {
    /// Identifies any block without exposing a component path.
    /// # Returns
    /// The untrusted ID, validated before presentation.
    pub fn id(&self) -> &str {
        match self {
            Self::Markdown { id, .. }
            | Self::Chart { id, .. }
            | Self::Table { id, .. }
            | Self::StaticTable { id, .. }
            | Self::Image { id, .. }
            | Self::Evidence { id, .. }
            | Self::Expandable { id, .. } => id,
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    RxmerFrequency,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    OpenRxmer { target_id: String },
}
