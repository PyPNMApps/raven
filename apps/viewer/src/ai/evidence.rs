/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Closed OFDM evidence contract. Values are copied from MERLIN, never recalculated.
use super::protocol::{digest, invalid, parse_json};
use crate::Diagnostic;
use pnm::pnm::data_type::{FrequencyHz, ProfileId, SubcarrierIdx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// One typed metric, including its original numerical scope and availability.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub id: String,
    pub profile_id: ProfileId,
    pub unit: String,
    pub scope: String,
    /// Decimal text preserves exact u64 counters across the Qt/JavaScript boundary.
    pub value: Option<String>,
}
/// Deterministic rule output; an observation is not a channel-health certification.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub profile_id: ProfileId,
    pub text: String,
    pub references: Vec<String>,
}
/// Redacted immutable OFDM run, separate from general PNM JSON contracts.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub schema_version: u32,
    pub rule_version: u32,
    pub raven_version: String,
    pub merlin_revision: String,
    pub run_id: String,
    pub status: String,
    pub classification: String,
    pub source_aliases: Vec<String>,
    pub source_hashes: Vec<String>,
    pub settings: Vec<Settings>,
    pub metrics: Vec<Metric>,
    pub findings: Vec<Finding>,
    pub limitations: Vec<String>,
}
/// Effective settings captured by the simulation owner before execution.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub profile_id: ProfileId,
    pub receiver_mode: String,
    pub channel_state: String,
    pub observation_ms: u64,
    pub runtime_limit_ms: u64,
    pub bandwidth_hz: FrequencyHz,
    pub zero_frequency_hz: FrequencyHz,
    pub first_subcarrier_index: SubcarrierIdx,
    pub spacing_hz: FrequencyHz,
    pub subcarrier_count: u32,
    pub cyclic_prefix_samples: u16,
    pub roll_off_samples: u16,
    pub interleaver_depth: u8,
    pub bandwidth_origin: String,
    pub cyclic_prefix_origin: String,
    pub roll_off_origin: String,
    pub interleaver_origin: String,
}
impl Evidence {
    /// Copies only approved, numeric result fields and safe effective settings.
    /// # Arguments
    /// * `report` - Native MERLIN-backed batch DTO; not a UI-reparsed number array.
    /// * `settings` - One effective setting snapshot per requested profile.
    /// * `source_hashes` - SHA-256 of selected capture bytes, excluding names/paths.
    /// * `revision` - Pinned engine revision from the adapter's existing owner.
    /// # Returns
    /// Closed evidence envelope with deterministic offline observations.
    /// # Errors
    /// Rejects malformed or incompatible scopes, duplicate IDs and missing settings.
    pub fn from_run(
        report: &Value,
        settings: Vec<Settings>,
        revision: &str,
        source_hashes: Vec<String>,
    ) -> Result<Self, Diagnostic> {
        let status = report["status"].as_str().ok_or_else(invalid)?.to_owned();
        let entries = report["entries"].as_array().ok_or_else(invalid)?;
        if entries.is_empty() || entries.len() > 16 || settings.len() != entries.len() {
            return Err(invalid());
        }
        let mut metrics = Vec::new();
        let mut findings = Vec::new();
        for entry in entries {
            let profile_id = entry["profile_id"]
                .as_u64()
                .and_then(|v| u8::try_from(v).ok())
                .ok_or_else(invalid)?;
            let prefix = format!("profile-{profile_id}");
            let state = entry["status"].as_str().ok_or_else(invalid)?;
            if !matches!(
                state,
                "Complete" | "Cancelled" | "RuntimeExceeded" | "Failed" | "NotStarted"
            ) {
                return Err(invalid());
            }
            let mut add =
                |name: &str, value: &Value, unit: &str, scope: &str| -> Result<(), Diagnostic> {
                    if !value.is_null() && !value.is_number() {
                        return Err(invalid());
                    }
                    metrics.push(Metric {
                        id: format!("{prefix}/{name}"),
                        profile_id,
                        unit: unit.into(),
                        scope: scope.into(),
                        value: value.as_number().map(ToString::to_string),
                    });
                    Ok(())
                };
            for name in ["total_cw", "clean_cw", "ccw", "ufec"] {
                if !entry["fec"].is_null() && entry["fec"]["scope"] != "ObservationWindow" {
                    return Err(invalid());
                }
                add(
                    name,
                    &entry["fec"]["counters"][name],
                    "codewords",
                    "ObservationWindow",
                )?;
            }
            for (name, unit) in [
                ("compared_bits", "bits"),
                ("bit_errors", "bits"),
                ("post_fec_ber", "ratio"),
            ] {
                if !entry["profile"].is_null() && entry["profile"]["scope"] != "ObservationWindow" {
                    return Err(invalid());
                }
                add(name, &entry["profile"][name], unit, "ObservationWindow")?;
            }
            for (name, unit) in [
                ("mean_iterations", "iterations/codeword"),
                ("iteration_sum", "iterations"),
                ("failures", "codewords"),
                ("bch_failures", "codewords"),
                ("ncp_iterations", "iterations"),
                ("plc_iterations", "iterations"),
            ] {
                if !entry["ldpc"].is_null() && entry["ldpc"]["scope"] != "All completed" {
                    return Err(invalid());
                }
                add(name, &entry["ldpc"][name], unit, "AllCompleted")?;
            }
            let bits = entry["profile"]["compared_bits"].as_u64();
            let errors = entry["profile"]["bit_errors"].as_u64();
            if let (Some(bits), Some(errors)) = (bits, errors) {
                if errors > bits {
                    return Err(invalid());
                }
                if bits > 0 {
                    findings.push(Finding { id:format!("{prefix}/observed-bits"), profile_id,
                        text: if errors == 0 { "No bit errors observed in the compared observation-window payload; this is not proof of an error-free channel.".into() } else { "Post-FEC payload bit errors were observed in the observation window.".into() },
                        references:vec![format!("{prefix}/compared_bits"),format!("{prefix}/bit_errors")] });
                }
            }
            findings.push(Finding {
                id: format!("{prefix}/status"),
                profile_id,
                text: format!("Simulation profile outcome: {state}."),
                references: vec![],
            });
        }
        let mut evidence = Self {schema_version:1,rule_version:1,raven_version:env!("CARGO_PKG_VERSION").into(),merlin_revision:revision.into(),run_id:String::new(),status,
            classification:"Simulated; MERLIN owns all numerical values".into(),source_hashes,source_aliases:vec!["modulation-profile".into(),"rxmer".into()],settings,metrics,findings,
            limitations:vec!["Not measured modem counters; no universal MER or LDPC health threshold is applied.".into(),"RxMER region summaries and raw sample arrays are not included; these counters cannot localize an impairment.".into(),"No MAC addresses, filenames, paths, source error strings or payload bytes are included. LDPC and FEC scopes differ.".into()]};
        if evidence.settings.iter().any(|s| s.channel_state == "Used") {
            evidence.source_aliases.push("channel-estimation".into());
        }
        evidence.run_id = digest(
            serde_json::to_string(&evidence)
                .map_err(|_| invalid())?
                .as_bytes(),
        );
        evidence.validate()?;
        Ok(evidence)
    }
    /// Parses a bounded closed snapshot and checks its metric/reference invariants.
    /// # Arguments
    /// * `text` - Native evidence JSON, with exact counters represented as strings.
    /// # Returns
    /// Validated owned snapshot.
    /// # Errors
    /// Rejects unknown versions/fields, duplicate keys/IDs, missing scopes or bad references.
    pub fn parse(text: &str) -> Result<Self, Diagnostic> {
        let value = parse_json(text.as_bytes(), 65536)?;
        let evidence: Self = serde_json::from_value(value).map_err(|_| invalid())?;
        evidence.validate()?;
        Ok(evidence)
    }
    fn validate(&self) -> Result<(), Diagnostic> {
        if self.schema_version != 1
            || self.rule_version != 1
            || self.run_id.len() != 64
            || self.settings.is_empty()
            || self.settings.len() > 16
            || self.metrics.len() > 208
            || self.findings.len() > 32
            || !matches!(
                self.status.as_str(),
                "Complete" | "CompletedWithErrors" | "Cancelled"
            )
        {
            return Err(invalid());
        }
        if self.source_hashes.len() < 2
            || self.source_hashes.len() > 3
            || self
                .source_hashes
                .iter()
                .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(invalid());
        }
        let mut unhashed = self.clone();
        unhashed.run_id.clear();
        if digest(
            serde_json::to_string(&unhashed)
                .map_err(|_| invalid())?
                .as_bytes(),
        ) != self.run_id
        {
            return Err(invalid());
        }
        if self.raven_version != env!("CARGO_PKG_VERSION")
            || self.merlin_revision.len() != 40
            || !self.merlin_revision.bytes().all(|b| b.is_ascii_hexdigit())
            || self.classification != "Simulated; MERLIN owns all numerical values"
            || self.limitations.len() > 8
            || self.limitations.iter().any(|text| text.len() > 1024)
        {
            return Err(invalid());
        }
        let mut expected_aliases = vec!["modulation-profile".to_string(), "rxmer".to_string()];
        if self.settings.iter().any(|s| s.channel_state == "Used") {
            expected_aliases.push("channel-estimation".into());
        }
        if self.source_aliases != expected_aliases
            || self.source_hashes.len() != self.source_aliases.len()
        {
            return Err(invalid());
        }
        let profiles: BTreeSet<_> = self.settings.iter().map(|s| s.profile_id).collect();
        if profiles.len() != self.settings.len() {
            return Err(invalid());
        }
        for setting in &self.settings {
            if !matches!(
                setting.receiver_mode.as_str(),
                "MCE" | "SuppliedChannel" | "Unity"
            ) || !matches!(
                setting.channel_state.as_str(),
                "Used" | "NotUsed" | "Ignored"
            ) || setting.observation_ms == 0
                || setting.runtime_limit_ms == 0
                || setting.bandwidth_hz == 0
                || setting.bandwidth_hz > 192_000_000
                || !matches!(setting.spacing_hz, 25_000 | 50_000)
                || setting.subcarrier_count == 0
                || setting.subcarrier_count > 65_536
                || [
                    &setting.bandwidth_origin,
                    &setting.cyclic_prefix_origin,
                    &setting.roll_off_origin,
                    &setting.interleaver_origin,
                ]
                .iter()
                .any(|origin| !matches!(origin.as_str(), "Explicit" | "Derived" | "Defaulted"))
            {
                return Err(invalid());
            }
        }
        let mut ids = BTreeSet::new();
        for metric in &self.metrics {
            if !profiles.contains(&metric.profile_id)
                || !ids.insert(metric.id.as_str())
                || !metric
                    .id
                    .starts_with(&format!("profile-{}/", metric.profile_id))
                || !matches!(metric.scope.as_str(), "ObservationWindow" | "AllCompleted")
            {
                return Err(invalid());
            }
            let name = metric.id.split('/').nth(1).ok_or_else(invalid)?;
            let (unit, scope) = match name {
                "total_cw" | "clean_cw" | "ccw" | "ufec" => ("codewords", "ObservationWindow"),
                "compared_bits" | "bit_errors" => ("bits", "ObservationWindow"),
                "post_fec_ber" => ("ratio", "ObservationWindow"),
                "mean_iterations" => ("iterations/codeword", "AllCompleted"),
                "iteration_sum" | "ncp_iterations" | "plc_iterations" => {
                    ("iterations", "AllCompleted")
                }
                "failures" | "bch_failures" => ("codewords", "AllCompleted"),
                _ => return Err(invalid()),
            };
            if metric.id != format!("profile-{}/{name}", metric.profile_id)
                || metric.unit != unit
                || metric.scope != scope
            {
                return Err(invalid());
            }
            if let Some(value) = &metric.value {
                let number: serde_json::Number =
                    serde_json::from_str(value).map_err(|_| invalid())?;
                if number.as_f64().is_none_or(|v| !v.is_finite() || v < 0.0) {
                    return Err(invalid());
                }
                if metric.unit != "ratio"
                    && metric.unit != "iterations/codeword"
                    && number.as_u64().is_none()
                {
                    return Err(invalid());
                }
                if metric.unit == "ratio" && number.as_f64().is_none_or(|v| v > 1.0) {
                    return Err(invalid());
                }
            }
        }
        if self.metrics.len() != self.settings.len() * 13 {
            return Err(invalid());
        }
        let mut finding_ids = BTreeSet::new();
        for finding in &self.findings {
            if !profiles.contains(&finding.profile_id)
                || !finding_ids.insert(&finding.id)
                || !finding
                    .id
                    .starts_with(&format!("profile-{}/", finding.profile_id))
                || finding.references.len() > 8
                || finding
                    .references
                    .iter()
                    .any(|id| !id.starts_with(&format!("profile-{}/", finding.profile_id)))
                || finding.text.len() > 512
                || finding
                    .references
                    .iter()
                    .any(|id| !ids.contains(id.as_str()))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    /// Renders local deterministic observations independently of AI availability.
    /// # Returns
    /// Plain text for the immutable local-findings panel.
    pub fn findings_text(&self) -> String {
        self.findings
            .iter()
            .map(|f| format!("Profile {}: {}", f.profile_id, f.text))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
/// Builds the closed narrative response schema accepted by the local validator.
/// # Returns
/// JSON Schema passed as provider assistance; Rust validation remains authoritative.
pub fn report_schema() -> Value {
    let reference = json!({"type":"object","additionalProperties":false,"properties":{"metric_id":{"type":"string"},"value":{"type":"string"},"unit":{"type":"string"},"scope":{"type":"string"}},"required":["metric_id","value","unit","scope"]});
    let claim = json!({"type":"object","additionalProperties":false,"properties":{"kind":{"type":"string","enum":["Observation","Hypothesis"]},"text":{"type":"string"},"references":{"type":"array","items":reference}},"required":["kind","text","references"]});
    json!({"type":"object","additionalProperties":false,"properties":{"schema_version":{"type":"integer","enum":[1]},"summary":{"type":"string"},"claims":{"type":"array","items":claim},"limitations":{"type":"array","items":{"type":"string"}},"suggested_checks":{"type":"array","items":{"type":"string"}}},"required":["schema_version","summary","claims","limitations","suggested_checks"]})
}
