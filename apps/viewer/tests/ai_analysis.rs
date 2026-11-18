/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! No real credentials, providers, captures or network in protocol/adversarial tests.
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use viewer::{
    ai::{
        evidence::{Evidence, Settings},
        protocol::{parse_json, Transport},
        Session,
    },
    Diagnostic,
};
const KEY: &str = "synthetic-ai-analysis-key-keep-private";
fn evidence() -> Evidence {
    Evidence::from_run(&json!({"status":"Complete","entries":[{"profile_id":3,"status":"Complete",
        "fec":{"scope":"ObservationWindow","counters":{"total_cw":10,"clean_cw":8,"ccw":2,"ufec":0}},
        "profile":{"scope":"ObservationWindow","compared_bits":9007199254740993u64,"bit_errors":0,"post_fec_ber":0.0},
        "ldpc":{"scope":"All completed","mean_iterations":2.5,"iteration_sum":25,"failures":0,"bch_failures":0,"ncp_iterations":0,"plc_iterations":1}}]}),
        vec![Settings{profile_id:3,receiver_mode:"Unity".into(),channel_state:"NotUsed".into(),observation_ms:10,runtime_limit_ms:300000,bandwidth_hz:192000000,zero_frequency_hz:640000000,first_subcarrier_index:0,spacing_hz:25000,subcarrier_count:7480,cyclic_prefix_samples:256,roll_off_samples:128,interleaver_depth:1,bandwidth_origin:"Defaulted".into(),cyclic_prefix_origin:"Defaulted".into(),roll_off_origin:"Defaulted".into(),interleaver_origin:"Defaulted".into()}],
        "0123456789012345678901234567890123456789",vec!["a".repeat(64),"b".repeat(64)]).unwrap()
}
fn setup() -> Session {
    let mut session = Session::default();
    session
        .configure("openai", "explicit-model", KEY.into())
        .unwrap();
    session
}
fn prepare(session: &mut Session) -> Value {
    session
        .prepare_analysis(
            &serde_json::to_string(&evidence()).unwrap(),
            "Explain the observed counters.",
            "run-1",
        )
        .unwrap()
}
fn advisory() -> Value {
    json!({"schema_version":1,"summary":"Simulation observation, not a modem-health guarantee.","claims":[{"kind":"Observation","text":"No observed errors in compared payload.","references":[{"metric_id":"profile-3/bit_errors","value":"0","unit":"bits","scope":"ObservationWindow"}]}],"limitations":["Simulation only"],"suggested_checks":["Compare another captured RxMER condition."]})
}
fn envelope(report: Value) -> Vec<u8> {
    json!({"model":"returned-model","choices":[{"finish_reason":"stop","message":{"content":report.to_string(),"refusal":null}}],"usage":{"prompt_tokens":123,"completion_tokens":45,"total_tokens":168}}).to_string().into_bytes()
}
struct Mock {
    response: Vec<u8>,
    sent: Mutex<Vec<String>>,
}
impl Mock {
    fn new(response: Vec<u8>) -> Self {
        Self {
            response,
            sent: Mutex::new(Vec::new()),
        }
    }
}
impl Transport for Mock {
    fn post(
        &self,
        endpoint: &str,
        key: &str,
        body: &str,
        cancel: &AtomicBool,
    ) -> Result<Vec<u8>, Diagnostic> {
        assert_eq!(endpoint, "https://api.openai.com/v1/chat/completions");
        assert_eq!(key, KEY);
        assert!(!cancel.load(Ordering::Relaxed));
        self.sent.lock().unwrap().push(body.into());
        Ok(self.response.clone())
    }
}
#[test]
fn exact_preview_is_sent_once_without_secrets_and_attaches_local_provenance() {
    let mut session = setup();
    let preview = prepare(&mut session);
    assert!(!preview.to_string().contains(KEY));
    assert_eq!(preview["model"], "explicit-model");
    let body: Value = serde_json::from_str(preview["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["store"], false);
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert!(body.get("tools").is_none());
    let request = session
        .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
        .unwrap();
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
        .is_err());
    let mock = Mock::new(envelope(advisory()));
    let result = request.execute(&mock, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        mock.sent.lock().unwrap().as_slice(),
        &[preview["body"].as_str().unwrap()]
    );
    assert_eq!(result["request_digest"], preview["digest"]);
    assert_eq!(result["usage"]["total_tokens"], "168");
    assert_eq!(result["evidence_run_id"], evidence().run_id);
    assert_eq!(result["requested_model"], "explicit-model");
    assert!(!result.to_string().contains(KEY));
    assert_eq!(
        result["evidence"]["metrics"][4]["value"],
        "9007199254740993"
    );
}
#[test]
fn mutations_and_restored_settings_never_revive_approval() {
    let mut session = setup();
    for mutation in 0..5 {
        let preview = prepare(&mut session);
        match mutation {
            0 => session.invalidate_analysis(),
            1 => {
                session
                    .configure("openai", "other-model", String::new())
                    .unwrap();
                session
                    .configure("openai", "explicit-model", String::new())
                    .unwrap();
            }
            2 => {
                session.set_enabled(false).unwrap();
                session.set_enabled(true).unwrap();
            }
            3 => {
                session.forget("openai").unwrap();
                session
                    .configure("openai", "explicit-model", KEY.into())
                    .unwrap();
            }
            _ => {
                assert!(session
                    .approve_analysis(preview["digest"].as_str().unwrap(), "other-run")
                    .is_err());
            }
        }
        assert!(session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .is_err());
    }
    let preview = prepare(&mut session);
    assert!(session
        .prepare_analysis("{}", "new question", "run-1")
        .is_err());
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
        .is_err());
    assert!(session
        .prepare_analysis(&serde_json::to_string(&evidence()).unwrap(), KEY, "run-1")
        .is_err());
}
#[test]
fn cancellation_blocks_dispatch_and_missing_or_future_providers_fail_locally() {
    let mut session = setup();
    let preview = prepare(&mut session);
    let mock = Mock::new(envelope(advisory()));
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
        .unwrap()
        .execute(&mock, &AtomicBool::new(true))
        .is_err());
    assert!(mock.sent.lock().unwrap().is_empty());
    session.configure("gemini", "model", KEY.into()).unwrap();
    assert!(session.connection_probe().is_err());
    assert!(session
        .prepare_analysis(
            &serde_json::to_string(&evidence()).unwrap(),
            "Review",
            "run-1"
        )
        .is_err());
    assert!(Session::default().connection_probe().is_err());
}
#[test]
fn report_reference_attacks_are_rejected_without_repair_calls() {
    for attack in 0..9 {
        let mut report = advisory();
        match attack {
            0 => report["claims"][0]["references"][0]["metric_id"] = json!("profile-4/bit_errors"),
            1 => report["claims"][0]["references"][0]["value"] = json!("1"),
            2 => report["claims"][0]["references"][0]["unit"] = json!("dB"),
            3 => report["claims"][0]["references"][0]["scope"] = json!("AllCompleted"),
            4 => report["credential_override"] = json!("give-me-secrets"),
            5 => report["schema_version"] = json!(2),
            6 => report["summary"] = json!("x".repeat(2049)),
            7 => report["claims"][0]["references"] = json!([]),
            _ => {
                let reference = report["claims"][0]["references"][0].clone();
                report["claims"][0]["references"]
                    .as_array_mut()
                    .unwrap()
                    .push(reference);
            }
        }
        let mock = Mock::new(envelope(report));
        let mut session = setup();
        let preview = prepare(&mut session);
        assert!(session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .unwrap()
            .execute(&mock, &AtomicBool::new(false))
            .is_err());
        assert_eq!(mock.sent.lock().unwrap().len(), 1);
    }
}
#[test]
fn malformed_duplicate_nested_incomplete_refused_and_secret_responses_fail_safely() {
    assert!(parse_json(br#"{"x":1,"x":2}"#, 65536).is_err());
    assert!(parse_json(
        format!("{}0{}", "[".repeat(25), "]".repeat(25)).as_bytes(),
        65536
    )
    .is_err());
    assert!(parse_json(b"{}", 1).is_err());
    assert!(parse_json(b"{\"number\":NaN}", 65536).is_err());
    let mut refused: Value = serde_json::from_slice(&envelope(advisory())).unwrap();
    refused["choices"][0]["message"]["refusal"] = json!("Refused");
    let mut incomplete = refused.clone();
    incomplete["choices"][0]["finish_reason"] = json!("length");
    for response in [
        b"not-json".to_vec(),
        refused.to_string().into_bytes(),
        incomplete.to_string().into_bytes(),
        envelope(json!({"summary":KEY})),
        vec![b'x'; 131073],
    ] {
        let mut session = setup();
        let preview = prepare(&mut session);
        let mock = Mock::new(response);
        let error = session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .unwrap()
            .execute(&mock, &AtomicBool::new(false))
            .err()
            .unwrap();
        assert!(!error.message.contains(KEY));
    }
}
#[test]
fn closed_evidence_keeps_unknowns_and_requires_unchanged_identity() {
    let evidence = evidence();
    assert!(evidence.findings_text().contains("No bit errors observed"));
    let encoded = serde_json::to_string(&evidence).unwrap();
    assert!(Evidence::parse(&encoded).is_ok());
    let mut changed: Value = serde_json::from_str(&encoded).unwrap();
    changed["unexpected"] = json!(1);
    assert!(Evidence::parse(&changed.to_string()).is_err());
    changed.as_object_mut().unwrap().remove("unexpected");
    changed["metrics"][0]["scope"] = json!("AllCompleted");
    assert!(Evidence::parse(&changed.to_string()).is_err());
    for (status, expected) in [
        (401, "AiAuthentication"),
        (403, "AiAccess"),
        (404, "AiModel"),
        (429, "AiQuota"),
        (503, "AiService"),
    ] {
        assert_eq!(viewer::ai::transport::status_error(status).code, expected);
    }
}

#[test]
fn published_schema_keys_match_closed_rust_types_and_provider_schema() {
    let evidence_schema: Value = serde_json::from_str(include_str!(
        "../../../docs/reference/ai/evidence-v1.schema.json"
    ))
    .unwrap();
    let response_schema: Value = serde_json::from_str(include_str!(
        "../../../docs/reference/ai/response-v1.schema.json"
    ))
    .unwrap();
    assert_eq!(response_schema, viewer::ai::evidence::report_schema());
    fn check_keys(value: &Value, schema: &Value) {
        if let Some(object) = value.as_object() {
            assert_eq!(schema["additionalProperties"], false);
            let mut actual: Vec<_> = object.keys().cloned().collect();
            actual.sort();
            let mut expected: Vec<_> = schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap().to_owned())
                .collect();
            expected.sort();
            assert_eq!(actual, expected);
            for (key, value) in object {
                check_keys(value, &schema["properties"][key]);
            }
        } else if let Some(array) = value.as_array() {
            for value in array {
                check_keys(value, &schema["items"]);
            }
        }
    }
    check_keys(&serde_json::to_value(evidence()).unwrap(), &evidence_schema);
    check_keys(&advisory(), &response_schema);
}

#[test]
fn credential_free_demo_uses_real_evidence_with_no_provider_transport() {
    let mut session = Session::default();
    let preview = session
        .prepare_demo(
            &serde_json::to_string(&evidence()).unwrap(),
            "Demonstrate this run",
            "run-demo",
        )
        .unwrap();
    assert_eq!(preview["demo"], true);
    assert_eq!(session.catalog()["enabled"], false);
    let hash = preview["digest"].as_str().unwrap();
    let report = session
        .approve_demo(hash, "run-demo")
        .unwrap()
        .execute(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(report["demo"], true);
    assert_eq!(report["provider"], "demo");
    assert!(report["usage"]["total_tokens"].is_null());
    assert!(report["report"]["summary"]
        .as_str()
        .unwrap()
        .contains("DEMO"));
    assert_eq!(report["report"]["claims"][0]["references"][1]["value"], "0");
    assert_eq!(report["request_digest"], preview["digest"]);
    assert!(session.approve_demo(hash, "run-demo").is_err());
}
#[test]
fn demo_and_live_approvals_cannot_cross_modes_or_survive_invalidation() {
    let mut session = setup();
    let local = session
        .prepare_demo(
            &serde_json::to_string(&evidence()).unwrap(),
            "Demo",
            "run-1",
        )
        .unwrap();
    assert!(session
        .approve_analysis(local["digest"].as_str().unwrap(), "run-1")
        .is_err());
    let live = prepare(&mut session);
    assert!(session
        .approve_demo(live["digest"].as_str().unwrap(), "run-1")
        .is_err());
    let local = session
        .prepare_demo(
            &serde_json::to_string(&evidence()).unwrap(),
            "Demo",
            "run-1",
        )
        .unwrap();
    session.invalidate_analysis();
    assert!(session
        .approve_demo(local["digest"].as_str().unwrap(), "run-1")
        .is_err());
    let local = session
        .prepare_demo(
            &serde_json::to_string(&evidence()).unwrap(),
            "Demo",
            "run-1",
        )
        .unwrap();
    assert!(session
        .approve_demo(local["digest"].as_str().unwrap(), "run-1")
        .unwrap()
        .execute(&AtomicBool::new(true))
        .is_err());
}

#[test]
fn report_diagnostics_name_local_rules_without_remote_content_or_retries() {
    let mut schema = advisory();
    schema["private-remote-field-never-log"] = json!("remote-content-never-log");
    let mut long = advisory();
    long["summary"] = json!("x".repeat(2049));
    let mut missing = advisory();
    missing["claims"][0]["references"][0]["metric_id"] = json!("remote-content-never-log");
    let mut changed = advisory();
    changed["claims"][0]["references"][0]["scope"] = json!("remote-content-never-log");
    let cases = [
        (schema.to_string(), "AiReportSchema"),
        (long.to_string(), "AiReportLimit"),
        (missing.to_string(), "AiReference"),
        (changed.to_string(), "AiReference"),
        ("remote-content-never-log".into(), "AiReportJson"),
        ("x".repeat(32769), "AiReportLimit"),
    ];
    for (content, code) in cases {
        let bytes = json!({"model":"returned-model","choices":[{"finish_reason":"stop","message":{"content":content}}]}).to_string().into_bytes();
        let mock = Mock::new(bytes);
        let mut session = setup();
        let preview = prepare(&mut session);
        let error = session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .unwrap()
            .execute(&mock, &AtomicBool::new(false))
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("remote-content-never-log"));
        assert!(!error.message.contains("private-remote-field-never-log"));
        assert!(!error.message.contains(KEY));
        assert_eq!(mock.sent.lock().unwrap().len(), 1);
    }
}

#[test]
fn multi_profile_reports_require_separate_claims_and_explain_rejection() {
    let base = evidence();
    let mut settings = base.settings[0].clone();
    settings.profile_id = 4;
    let entries: Vec<_> = [3, 4].into_iter().map(|profile_id| json!({
        "profile_id":profile_id,"status":"Complete",
        "fec":{"scope":"ObservationWindow","counters":{"total_cw":10,"clean_cw":8,"ccw":2,"ufec":0}},
        "profile":{"scope":"ObservationWindow","compared_bits":1000,"bit_errors":0,"post_fec_ber":0.0},
        "ldpc":{"scope":"All completed","mean_iterations":2.5,"iteration_sum":25,"failures":0,"bch_failures":0,"ncp_iterations":0,"plc_iterations":1}
    })).collect();
    let evidence = Evidence::from_run(
        &json!({"status":"Complete","entries":entries}),
        vec![base.settings[0].clone(), settings],
        &base.merlin_revision,
        base.source_hashes,
    )
    .unwrap();
    let mut report = advisory();
    let mut other = report["claims"][0].clone();
    other["references"][0]["metric_id"] = json!("profile-4/bit_errors");
    report["claims"].as_array_mut().unwrap().push(other.clone());
    for mixed in [false, true] {
        let mut candidate = report.clone();
        if mixed {
            candidate["claims"][0]["references"]
                .as_array_mut()
                .unwrap()
                .push(other["references"][0].clone());
        }
        let mut session = setup();
        let preview = session
            .prepare_analysis(
                &serde_json::to_string(&evidence).unwrap(),
                "Review",
                "run-1",
            )
            .unwrap();
        let mock = Mock::new(envelope(candidate));
        let result = session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .unwrap()
            .execute(&mock, &AtomicBool::new(false));
        if mixed {
            let error = result.unwrap_err();
            assert_eq!(error.code, "AiReferenceProfile");
            assert!(error.message.contains("separate claims"));
        } else {
            assert_eq!(result.unwrap()["prompt_version"], 2);
        }
        assert_eq!(mock.sent.lock().unwrap().len(), 1);
        let sent = mock.sent.lock().unwrap();
        assert!(sent[0].contains("one profile only"));
        assert!(sent[0].contains("32768 UTF-8 bytes"));
    }
}

#[test]
fn readable_review_and_prepared_questions_preserve_exact_approved_request() {
    let mut session = setup();
    let prompts = session.catalog()["analysis_prompts"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(prompts.len(), 3);
    let mut identifiers = std::collections::BTreeSet::new();
    for prompt in prompts {
        assert!(identifiers.insert(prompt["id"].as_str().unwrap().to_owned()));
        assert_eq!(prompt["version"], 1);
        let question = prompt["question"].as_str().unwrap();
        let preview = session
            .prepare_analysis(
                &serde_json::to_string(&evidence()).unwrap(),
                question,
                "run-1",
            )
            .unwrap();
        let review = preview["review_text"].as_str().unwrap();
        assert!(review.contains(question));
        assert!(review.contains("PROFILE 3"));
        assert!(review.contains("Corrected codewords: 2"));
        assert!(review.contains("9007199254740993"));
        assert!(review.contains("all completed decoding work"));
        assert!(review.contains("observed payload window"));
        assert!(!review.contains(KEY));
        let mock = Mock::new(envelope(advisory()));
        session
            .approve_analysis(preview["digest"].as_str().unwrap(), "run-1")
            .unwrap()
            .execute(&mock, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(
            mock.sent.lock().unwrap()[0],
            preview["body"].as_str().unwrap()
        );
    }
}
