/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Credential-free deterministic demo. This module has no transport capability.
use super::{
    evidence::Evidence,
    protocol::{check_cancel, digest, invalid, validate_context, validate_report, Pending},
    Session,
};
use crate::Diagnostic;
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

/// Frozen local demonstration request, with no credentials or network adapter.
pub struct DemoRequest {
    pending: Pending,
}
impl Session {
    /// Previews a local demonstration without requiring provider configuration.
    /// # Arguments
    /// * `evidence`, `question`, `source` - Strict snapshot, explicit question and run token.
    /// # Returns
    /// Complete local request preview, clearly marked Demo and never sent remotely.
    /// # Errors
    /// Rejects invalid evidence, empty/oversized questions or invalid source tokens.
    pub fn prepare_demo(
        &mut self,
        evidence: &str,
        question: &str,
        source: &str,
    ) -> Result<Value, Diagnostic> {
        self.pending = None;
        validate_context(question, source)?;
        let evidence = Evidence::parse(evidence)?;
        let body = serde_json::to_string(
            &json!({"mode":"demo","schema_version":1,"evidence":evidence,"question":question}),
        )
        .map_err(|_| invalid())?;
        if self
            .configurations
            .values()
            .any(|setting| body.contains(setting._api_key.as_str()))
        {
            return Err(Diagnostic::new(
                "AiSecret",
                "Remove API credentials from the question before creating a preview.",
            ));
        }
        let hash = digest(body.as_bytes());
        let preview = json!({"demo":true,"destination":"Local Rust demo · no network","model":"deterministic-demo-v1","body":body,"digest":hash,"source":source,"review_text":super::presentation::request_review(&evidence,question),"findings":evidence.findings_text()});
        self.pending = Some(Pending {
            body,
            digest: hash,
            source: source.into(),
            evidence,
            question: question.into(),
            demo: true,
        });
        Ok(preview)
    }
    /// Consumes a local-only approval; cannot produce a live provider request.
    /// # Arguments
    /// * `hash`, `source` - Displayed digest and current native run token.
    /// # Returns
    /// Single-use credential-free demo request.
    /// # Errors
    /// Rejects live, stale, cancelled or replayed approvals.
    pub fn approve_demo(&mut self, hash: &str, source: &str) -> Result<DemoRequest, Diagnostic> {
        let pending = self.pending.take().ok_or_else(invalid)?;
        if !pending.demo || pending.digest != hash || pending.source != source {
            return Err(invalid());
        }
        Ok(DemoRequest { pending })
    }
}
impl DemoRequest {
    /// Creates deterministic commentary from available existing findings only.
    /// # Arguments
    /// * `cancel` - Cooperative cancellation token shared with the UI worker.
    /// # Returns
    /// Validated demo report and export metadata; token counts are unavailable.
    /// # Errors
    /// Rejects cancellation or a locally invalid evidence/reference contract.
    pub fn execute(self, cancel: &AtomicBool) -> Result<Value, Diagnostic> {
        check_cancel(cancel)?;
        let evidence = &self.pending.evidence;
        let claims:Vec<_>=evidence.findings.iter().filter(|finding|!finding.references.is_empty()).map(|finding| {
            let references:Vec<_>=finding.references.iter().filter_map(|id|evidence.metrics.iter().find(|metric|&metric.id==id && metric.value.is_some())).map(|metric|json!({"metric_id":metric.id,"value":metric.value,"unit":metric.unit,"scope":metric.scope})).collect();
            json!({"kind":"Observation","text":finding.text,"references":references})
        }).filter(|claim|!claim["references"].as_array().unwrap().is_empty()).collect();
        let report = json!({"schema_version":1,"summary":"DEMO · Simulated response generated locally from existing Rust findings. No language model or provider connection was used.","claims":claims,"limitations":["This is a deterministic UI demonstration, not an independent AI evaluation.","The entered question is recorded but not interpreted by a language model.","Original MERLIN values and scopes remain authoritative."],"suggested_checks":["Review the local findings and original FEC/BER/LDPC counters."]});
        let report = validate_report(&report.to_string(), evidence)?;
        check_cancel(cancel)?;
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| invalid())?
            .as_secs()
            .to_string();
        Ok(
            json!({"schema_version":1,"demo":true,"provider":"demo","requested_model":"deterministic-demo-v1","returned_model":"deterministic-demo-v1","prompt_version":null,"rule_version":1,"source":self.pending.source,"question":self.pending.question,"evidence":evidence,"evidence_run_id":evidence.run_id,"request_digest":self.pending.digest,"completed_epoch":time,"usage":{"input_tokens":null,"output_tokens":null,"total_tokens":null},"report":report,"findings":evidence.findings,"advisory":true}),
        )
    }
}
