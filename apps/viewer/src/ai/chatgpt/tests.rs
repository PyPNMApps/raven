/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Offline synthetic OAuth, catalog and streaming tests. No real account or secret.
use super::*;
use crate::ai::{
    evidence::{Evidence, Settings},
    protocol::Transport,
    Session,
};
use std::sync::atomic::{AtomicBool, Ordering};
const ID_TOKEN: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6InJhdmVuLXN5bnRoZXRpYy10ZXN0In0.eyJpc3MiOiJodHRwczovL2F1dGgub3BlbmFpLmNvbSIsInN1YiI6InN5bnRoZXRpYy11c2VyIiwiYXVkIjoib2FpYXBwX3N5bnRoZXRpYyIsIm5vbmNlIjoic3ludGhldGljLW5vbmNlIiwiZXhwIjoyMDAwMDAwMDAwLCJpYXQiOjE3MDAwMDAwMDB9.BTBeKLdZhmY52Y3aY3hihMcvbVh4aCNtPvCqrdvqMD74QeELp5c7YEPbRIFsBhuZRvDpk_g4OtSpDnuqsF22R_UmkmI5u0Ny7Db3O1qDUQbD8xKLt7w_mbe82KGg-bQnccQm104mr2xKwGDMyeCDjW4bv-D5-BWUyoznM9pFypUtawVF4O8m-ZvFR9p2erFjCiGC1i6pDD9vgD0y4T3FuzrkJPPq6XOPWkRdEHohUVhKX0kpJVmRFZRKWDgdIbu9FaExRIbbXWWKtQf7dmh93wg6tpLeFdKeYevzTJoq9QIQsLQnUjALXXZS4SOgj2CFcE98hXQK_zZZZsBnQ35h_g";
fn jwks() -> Value {
    serde_json::from_str(r#"{"keys": [{"kid": "raven-synthetic-test", "kty": "RSA", "alg": "RS256", "use": "sig", "n": "3bhmRGEK15vOoGxw8RAW2-1AqFC4eyyCG8qHCAaeJhH52UK8ZoJHcDUih5PjAeLxZUGVEFLOcpY8WPiwPkSW84DqdtvTPXqO4ueJoe4zPkYZ_wLBB-JlW9V6L3yWnMbwZAmi9uoTTUwTWHSGT8mJym8edEp_8yIsgVld5-ywizpF7IzOMnuI2JBDMzY5hrbXn23OjbKx5OzGl4q2j1gMrI9bT6NC_tf8I1CMfpqenrW_lhLsFejJrP3b_cgW7iw06hJOXng17qpTVkgUaON5-oDx4rz5hzzMBmrJFxWNpOIDm2Ugh1jOHfoSDmuQFEGvPkM3mxq8gMWqEQDPvlV6uw", "e": "AQAB"}]}"#).unwrap()
}
fn evidence() -> Evidence {
    Evidence::from_run(&json!({"status":"Complete","entries":[{"profile_id":3,"status":"Complete",
        "fec":{"scope":"ObservationWindow","counters":{"total_cw":10,"clean_cw":8,"ccw":2,"ufec":0}},
        "profile":{"scope":"ObservationWindow","compared_bits":9007199254740993u64,"bit_errors":0,"post_fec_ber":0.0},
        "ldpc":{"scope":"All completed","mean_iterations":2.5,"iteration_sum":25,"failures":0,"bch_failures":0,"ncp_iterations":0,"plc_iterations":1}}]}),
        vec![Settings{profile_id:3,receiver_mode:"Unity".into(),channel_state:"NotUsed".into(),observation_ms:10,runtime_limit_ms:300000,bandwidth_hz:192000000,zero_frequency_hz:640000000,first_subcarrier_index:0,spacing_hz:25000,subcarrier_count:7480,cyclic_prefix_samples:256,roll_off_samples:128,interleaver_depth:1,bandwidth_origin:"Defaulted".into(),cyclic_prefix_origin:"Defaulted".into(),roll_off_origin:"Defaulted".into(),interleaver_origin:"Defaulted".into()}],
        "0123456789012345678901234567890123456789",vec!["a".repeat(64),"b".repeat(64)]).unwrap()
}
fn connection() -> Connection {
    Connection {
        access: Zeroizing::new("synthetic-oauth-access".into()),
        refresh: Zeroizing::new("synthetic-oauth-refresh".into()),
        client: "oaiapp_synthetic".into(),
        expires: now().unwrap() + 3600,
        model: "account-model".into(),
        models: vec![("account-model".into(), "Account model".into())],
    }
}
fn completed() -> Vec<u8> {
    let report = json!({"schema_version":1,"summary":"Synthetic test advisory","claims":[],"limitations":[],"suggested_checks":[]});
    let value = json!({"type":"response.completed","response":{"status":"completed","model":"account-model","output":[{"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":report.to_string(),"annotations":[]}]}],"usage":{"input_tokens":10,"output_tokens":20,"total_tokens":30}}});
    format!("event: response.completed\ndata: {value}\n\n").into_bytes()
}
#[test]
fn oidc_requires_signature_issuer_audience_nonce_and_time() {
    assert_eq!(
        login::verify_identity(
            ID_TOKEN,
            &jwks(),
            "oaiapp_synthetic",
            "synthetic-nonce",
            1800000000
        )
        .unwrap(),
        "synthetic-user"
    );
    for (client, nonce, time) in [
        ("other", "synthetic-nonce", 1800000000),
        ("oaiapp_synthetic", "wrong", 1800000000),
        ("oaiapp_synthetic", "synthetic-nonce", 2100000000),
        ("oaiapp_synthetic", "synthetic-nonce", 1600000000),
    ] {
        assert!(login::verify_identity(ID_TOKEN, &jwks(), client, nonce, time).is_err());
    }
    let mut damaged = ID_TOKEN.to_owned().into_bytes();
    damaged[100] = if damaged[100] == b'A' { b'B' } else { b'A' };
    assert!(login::verify_identity(
        std::str::from_utf8(&damaged).unwrap(),
        &jwks(),
        "oaiapp_synthetic",
        "synthetic-nonce",
        1800000000
    )
    .is_err());
    let mut wrong = jwks();
    wrong["keys"][0]["kid"] = json!("unknown");
    assert!(login::verify_identity(
        ID_TOKEN,
        &wrong,
        "oaiapp_synthetic",
        "synthetic-nonce",
        1800000000
    )
    .is_err());
}
#[test]
fn callback_requires_unique_state_code_and_issued_client() {
    let directory = tempfile::tempdir().unwrap();
    let attempt = SignIn::begin(&directory.path().join("registration.json")).unwrap();
    let url = url::Url::parse(&attempt.authorization_url()).unwrap();
    let query = url
        .query_pairs()
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(query["client_id"], "dynamic_agent_client");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["agent_name_hint"], "RAVEN");
    assert!(query["redirect_uri"].starts_with("http://127.0.0.1:"));
    let state = &query["state"];
    let valid = format!("/auth/callback?code=test&client_id=oaiapp_synthetic&state={state}");
    assert!(attempt.callback(&valid).is_ok());
    for target in [
        valid.replace(state.as_ref(), "wrong"),
        format!("{valid}&state={state}"),
        valid.replace("oaiapp_synthetic", "dynamic_agent_client"),
        valid.replace("/auth/callback", "/other"),
        format!("/auth/callback?code=test&state={state}"),
    ] {
        assert!(attempt.callback(&target).is_err());
    }
    assert!(attempt
        .callback(&format!("/auth/callback?error=access_denied&state={state}"))
        .is_err());
    let saved = std::fs::read_to_string(directory.path().join("registration.json")).unwrap();
    assert!(!saved.contains("token"));
    assert!(attempt.complete(&AtomicBool::new(true)).is_err());
}
#[test]
fn sign_in_metadata_is_stable_and_redirects_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("registration.json");
    let first = SignIn::begin(&path).unwrap().authorization_url();
    let second = SignIn::begin(&path).unwrap().authorization_url();
    let host = |text: &str| {
        url::Url::parse(text)
            .unwrap()
            .query_pairs()
            .find(|(name, _)| name == "ext_agent_host_id")
            .unwrap()
            .1
            .into_owned()
    };
    assert_eq!(host(&first), host(&second));
    assert!(host(&first).starts_with("urn:uuid:"));
    assert_eq!(host(&first).len(), 45);
    assert_ne!(first, second);
    std::fs::write(&path, "{}").unwrap();
    assert!(SignIn::begin(&path).is_err());
}
#[test]
fn model_catalog_is_filtered_and_bound_to_session() {
    let models=login::visible_models(&json!({"models":[{"slug":"hidden","display_name":"Hidden","visibility":"hidden"},{"slug":"account-model","display_name":"Account model","visibility":"list"}]})).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].0, "account-model");
    assert!(login::visible_models(&json!({"models":[]})).is_err());
    let mut credentials = connection();
    assert!(credentials.select_model("made-up-model").is_err());
    credentials.expires = 0;
    assert!(credentials.select_model("account-model").is_err());
    let mut session = Session::default();
    assert!(session
        .configure("chatgpt", "model", "api-key".into())
        .is_err());
    let catalog = session.connect_chatgpt(connection());
    assert_eq!(catalog["selected"], "chatgpt");
    assert!(catalog["analysis_available"].as_bool().unwrap());
    assert!(!catalog.to_string().contains("synthetic-oauth"));
    assert!(session.disconnect_chatgpt().is_some());
    assert!(!session.catalog()["enabled"].as_bool().unwrap());
}
#[test]
fn sse_requires_completion_and_rejects_tools_refusals_failures_and_duplicates() {
    let bytes = completed();
    assert_eq!(
        responses::envelope(&bytes).unwrap()["usage"]["total_tokens"],
        30
    );
    assert!(responses::envelope(
        b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n"
    )
    .is_err());
    let mut doubled = bytes.clone();
    doubled.extend_from_slice(&bytes);
    assert!(responses::envelope(&doubled).is_err());
    for text in [
        String::from_utf8(bytes.clone())
            .unwrap()
            .replace("response.completed", "response.failed"),
        String::from_utf8(bytes.clone())
            .unwrap()
            .replace("output_text", "refusal"),
        String::from_utf8(bytes.clone())
            .unwrap()
            .replace("message", "function_call"),
    ] {
        assert!(responses::envelope(text.as_bytes()).is_err());
    }
    assert!(responses::envelope(
        b"data: {\"type\":\"response.completed\",\"type\":\"response.completed\"}\n\n"
    )
    .is_err());
}
#[test]
fn approved_subscription_request_never_uses_api_key_endpoint_or_fallback() {
    struct Captured {
        expected: String,
    }
    impl Transport for Captured {
        fn post(
            &self,
            endpoint: &str,
            key: &str,
            body: &str,
            _cancel: &AtomicBool,
        ) -> Result<Vec<u8>, Diagnostic> {
            assert_eq!(endpoint, RESPONSES_URL);
            assert_eq!(key, "synthetic-oauth-access");
            assert_eq!(body, self.expected);
            Ok(completed())
        }
    }
    let mut session = Session::default();
    session.connect_chatgpt(connection());
    let preview = session
        .prepare_analysis(
            &serde_json::to_string(&evidence()).unwrap(),
            "Explain results",
            "source",
        )
        .unwrap();
    let body = preview["body"].as_str().unwrap();
    let request: Value = serde_json::from_str(body).unwrap();
    assert_eq!(request["store"], false);
    assert_eq!(request["stream"], true);
    assert!(request["tools"].is_null());
    assert!(request["messages"].is_null());
    assert!(request["max_output_tokens"].is_null());
    assert!(!body.contains("synthetic-oauth"));
    let approved = session
        .approve_analysis(preview["digest"].as_str().unwrap(), "source")
        .unwrap();
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "source")
        .is_err());
    let report = approved
        .execute(
            &Captured {
                expected: body.into(),
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(report["provider"], "chatgpt");
    assert_eq!(report["usage"]["total_tokens"], "30");
    assert!(!report.to_string().contains("synthetic-oauth"));
    let preview = session
        .prepare_analysis(
            &serde_json::to_string(&evidence()).unwrap(),
            "Explain results",
            "source",
        )
        .unwrap();
    session.select_chatgpt_model("account-model").unwrap();
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "source")
        .is_err());
    let preview = session
        .prepare_analysis(
            &serde_json::to_string(&evidence()).unwrap(),
            "Explain results",
            "source",
        )
        .unwrap();
    session.disconnect_chatgpt();
    assert!(session
        .approve_analysis(preview["digest"].as_str().unwrap(), "source")
        .is_err());
    assert!(session.connection_probe().is_err());
    let cancel = AtomicBool::new(false);
    cancel.store(true, Ordering::Relaxed);
    assert!(crate::ai::protocol::check_cancel(&cancel).is_err());
}
#[test]
fn standard_and_subscription_model_catalogs_use_only_valid_server_ids() {
    let models=login::visible_models(&json!({"object":"list","data":[{"object":"model","id":"account-model","created":123,"owned_by":"openai"}]})).unwrap();
    assert_eq!(
        models,
        vec![("account-model".into(), "account-model".into())]
    );
    for value in [
        json!({"object":"list","data":[{"object":"model","id":"account-model"}],"models":[]}),
        json!({"data":[{"object":"model","id":"account-model"}]}),
        json!({"object":"list","data":[{"object":"other","id":"account-model"}]}),
        json!({"object":"list","data":[{"object":"model","id":"unsafe\nname"}]}),
        json!({"object":"list","data":[{"object":"model","id":"same"},{"object":"model","id":"same"}]}),
        json!({"models":[{"slug":"model","visibility":"list","display_name":"unsafe\nlabel"}]}),
    ] {
        let error = login::visible_models(&value).unwrap_err();
        assert!(error.code.starts_with("AiChatGpt"));
        assert!(!error.message.contains("unsafe"));
        assert!(!error.message.contains("evidence contract"));
    }
}
#[test]
fn authentication_http_diagnostics_exclude_remote_bodies_and_tokens() {
    let cancel = AtomicBool::new(false);
    let remote = ureq::Response::new(401, "Unauthorized", "synthetic-token-never-reflect").unwrap();
    let error = read_response(Err(ureq::Error::Status(401, remote)), &cancel).unwrap_err();
    assert!(error.message.contains("401"));
    assert!(!error.message.contains("synthetic-token"));
    let diagnostic = sign_in_stage("model-catalog retrieval", error);
    assert!(diagnostic.message.contains("model-catalog retrieval"));
    assert!(!diagnostic.message.contains("synthetic-token"));
    let invalid = sign_in_stage("identity verification", invalid());
    assert!(invalid.message.contains("identity verification"));
    assert!(!invalid.message.contains("evidence contract"));
}
#[test]
fn rich_model_catalog_exceeding_auth_budget_loads_with_its_own_bound() {
    let cancel = AtomicBool::new(false);
    let body=json!({"models":[{"slug":"account-model","display_name":"Account model","visibility":"list","capabilities":{"description":"x".repeat(350_000)}}]}).to_string();
    assert!(body.len() > MAX_REPLY_BYTES);
    assert!(read_response(Ok(ureq::Response::new(200, "OK", &body).unwrap()), &cancel).is_err());
    let bytes = read_response_with_limit(
        Ok(ureq::Response::new(200, "OK", &body).unwrap()),
        &cancel,
        MAX_MODEL_CATALOG_BYTES,
    )
    .unwrap();
    let models =
        login::visible_models(&parse_json(&bytes, MAX_MODEL_CATALOG_BYTES).unwrap()).unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].0, "account-model");
    let boundary = "x".repeat(MAX_MODEL_CATALOG_BYTES);
    assert_eq!(
        read_response_with_limit(
            Ok(ureq::Response::new(200, "OK", &boundary).unwrap()),
            &cancel,
            MAX_MODEL_CATALOG_BYTES
        )
        .unwrap()
        .len(),
        MAX_MODEL_CATALOG_BYTES
    );
    let oversized = boundary + "x";
    assert!(read_response_with_limit(
        Ok(ureq::Response::new(200, "OK", &oversized).unwrap()),
        &cancel,
        MAX_MODEL_CATALOG_BYTES
    )
    .is_err());
    assert!(read_response_with_limit(
        Ok(ureq::Response::new(200, "OK", &body).unwrap()),
        &AtomicBool::new(true),
        MAX_MODEL_CATALOG_BYTES
    )
    .is_err());
}
#[test]
fn incremental_sse_accepts_large_delta_transfer_with_a_small_final_report() {
    struct Chunked {
        input: std::io::Cursor<Vec<u8>>,
    }
    impl std::io::Read for Chunked {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let bound = buffer.len().min(7);
            self.input.read(&mut buffer[..bound])
        }
    }
    let delta = format!(
        "event: response.output_text.delta\r\ndata: {}\r\n\r\n",
        json!({"type":"response.output_text.delta","delta":"é".repeat(80)})
    );
    let mut bytes = delta.repeat(2000).into_bytes();
    bytes.extend_from_slice(&completed());
    assert!(bytes.len() > 262144);
    let terminal = responses::read_stream(
        Chunked {
            input: std::io::Cursor::new(bytes),
        },
        "secret-not-in-stream",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(terminal.len() < 4096);
    let report = responses::envelope(&terminal).unwrap();
    assert_eq!(report["usage"]["total_tokens"], 30);
    assert!(report["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .contains("Synthetic test advisory"));
}
#[test]
fn streaming_preserves_failure_completion_cancellation_and_secret_guards() {
    let cancel = AtomicBool::new(false);
    let terminal = completed();
    let mut reflected = b": synthetic-private-token\n\n".to_vec();
    reflected.extend_from_slice(&terminal);
    assert!(responses::read_stream(
        std::io::Cursor::new(reflected),
        "synthetic-private-token",
        &cancel
    )
    .is_err());
    let mut failed = b"data: {\"type\":\"response.failed\"}\n\n".to_vec();
    failed.extend_from_slice(&terminal);
    assert!(responses::read_stream(std::io::Cursor::new(failed), "", &cancel).is_err());
    let mut doubled = terminal.clone();
    doubled.extend_from_slice(&terminal);
    assert!(responses::read_stream(std::io::Cursor::new(doubled), "", &cancel).is_err());
    let mut truncated = terminal.clone();
    truncated.pop();
    assert!(responses::read_stream(std::io::Cursor::new(truncated), "", &cancel).is_err());
    assert!(
        responses::read_stream(std::io::Cursor::new(terminal), "", &AtomicBool::new(true)).is_err()
    );
}

#[test]
fn stream_and_output_diagnostics_do_not_reflect_provider_fields() {
    let frames = [
        ("data: [DONE]\n\n".to_owned(), "AiStreamFormat"),
        ("data: {\"private-remote-field-never-log\":true}\n\n".to_owned(), "AiStreamFormat"),
        ("data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[{\"type\":\"private-remote-field-never-log\"}]}}\n\n".to_owned(), "AiEnvelope"),
    ];
    for (frame, code) in frames {
        let error = responses::envelope(frame.as_bytes()).unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("private-remote-field-never-log"));
        assert!(!error.message.contains("evidence contract"));
    }
}
