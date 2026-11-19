/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Trusted local datasets and provenance. Never construct these from a provider response.
use crate::capture::chart::ChartViewport;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Measured,
    Simulation,
    Derived,
    Demonstration,
    AiInterpretation,
}
#[derive(Clone, Serialize)]
pub struct EvidenceValue {
    pub id: String,
    pub label: String,
    pub value: Option<String>,
    pub unit: String,
    pub scope: String,
    pub classification: Classification,
    pub generation: u64,
}
#[derive(Clone, Serialize)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub classification: Classification,
    pub generation: u64,
}
pub struct Chart {
    pub classification: Classification,
    pub viewport: ChartViewport,
    pub generation: u64,
}
pub struct Context {
    pub id: String,
    pub generation: u64,
    pub classification: Classification,
    pub charts: BTreeMap<String, Chart>,
    pub tables: BTreeMap<String, Table>,
    pub evidence: BTreeMap<String, EvidenceValue>,
}
impl Context {
    /// Creates an empty registry owned by one local capture or simulation generation.
    /// # Arguments
    /// * `id`, `generation`, `classification` - Trusted owner identity and provenance.
    /// # Returns
    /// A registry to populate from Rust-owned results, never provider-supplied values.
    pub fn new(id: &str, generation: u64, classification: Classification) -> Self {
        Self {
            id: id.into(),
            generation,
            classification,
            charts: BTreeMap::new(),
            tables: BTreeMap::new(),
            evidence: BTreeMap::new(),
        }
    }
}
