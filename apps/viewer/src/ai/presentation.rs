/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Plain-language review of frozen evidence; no numerical calculations or network access.
use super::evidence::Evidence;
use serde_json::{json, Value};

/// Lists prepared questions for the currently supported OFDM analysis.
/// # Returns
/// Versioned editable questions; choosing one never approves or sends a request.
pub(super) fn prepared_questions() -> Value {
    json!([
        {"id":"ofdm-overview","version":1,"label":"Profile performance overview","question":"Review each selected OFDM profile separately. Explain the observed FEC, BER and LDPC results in plain language. Identify what the supplied simulation results establish and what remains unknown. Do not combine counters across profiles."},
        {"id":"ofdm-fec","version":1,"label":"FEC and decoder effort","question":"For each OFDM profile, explain corrected and uncorrectable codewords, payload bit errors and LDPC decoder effort using only supplied results. Keep observation-window and all-completed scopes separate. State unavailable measurements explicitly."},
        {"id":"ofdm-next-checks","version":1,"label":"Findings and recommended checks","question":"Summarize the supplied deterministic findings for each OFDM profile. Separate observations from hypotheses and suggest practical next checks. Do not infer the location or cause of an impairment without supporting evidence."}
    ])
}

/// Formats all approved run settings, metrics and findings for human review.
/// # Arguments
/// * `evidence` - Validated immutable snapshot, not mutable workspace data.
/// * `question` - Exact question included in the approved outbound request.
/// # Returns
/// Plain text with unchanged values and explicit scopes. Exact JSON stays available separately.
pub(super) fn request_review(evidence: &Evidence, question: &str) -> String {
    let mut lines = vec!["QUESTION".to_owned(), question.to_owned(), String::new(),
        "WHAT WILL BE SHARED".to_owned(),
        "The question, selected profiles, simulation settings, results, local findings and provenance. No raw captures, MAC addresses, file paths or credentials.".to_owned(),
        "These are simulated MERLIN results, not measured modem counters.".to_owned()];
    for settings in &evidence.settings {
        lines.push(format!("\nPROFILE {}", settings.profile_id));
        lines.push("Results".into());
        for metric in evidence
            .metrics
            .iter()
            .filter(|metric| metric.profile_id == settings.profile_id)
        {
            let name = metric.id.split('/').next_back().unwrap_or(&metric.id);
            let label = match name {
                "total_cw" => "Total codewords",
                "clean_cw" => "Clean codewords",
                "ccw" => "Corrected codewords",
                "ufec" => "Uncorrectable codewords",
                "compared_bits" => "Compared payload bits",
                "bit_errors" => "Payload bit errors",
                "post_fec_ber" => "Post-FEC bit error rate",
                "mean_iterations" => "Mean LDPC iterations",
                "iteration_sum" => "Total LDPC iterations",
                "failures" => "LDPC failures",
                "bch_failures" => "BCH failures",
                "ncp_iterations" => "NCP iterations",
                "plc_iterations" => "PLC iterations",
                _ => name,
            };
            let scope = match metric.scope.as_str() {
                "ObservationWindow" => "observed payload window",
                "AllCompleted" => "all completed decoding work",
                other => other,
            };
            lines.push(format!(
                "  {label}: {} {} ({scope})",
                metric.value.as_deref().unwrap_or("Unavailable"),
                metric.unit
            ));
        }
        for finding in evidence
            .findings
            .iter()
            .filter(|finding| finding.profile_id == settings.profile_id)
        {
            lines.push(format!("  Finding: {}", finding.text));
        }

        // Keep settings available without pushing the profile's results out of view.
        if let Ok(Value::Object(fields)) = serde_json::to_value(settings) {
            let fields: Vec<_> = fields
                .into_iter()
                .filter(|(name, _)| name != "profile_id")
                .map(|(name, value)| {
                    let text = value
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| value.to_string());
                    format!("{}: {text}", name.replace('_', " "))
                })
                .collect();
            lines.push(format!("Settings: {}", fields.join(" · ")));
        }
    }
    lines.push("\nLIMITATIONS".into());
    lines.extend(evidence.limitations.iter().cloned());
    lines.push("\nTechnical details contains the exact outbound request, destination, model and SHA-256. Approval sends those frozen bytes once.".into());
    lines.join("\n")
}
