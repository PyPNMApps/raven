/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Session-only AI setup contracts. Synthetic secrets; no network or OS vault use.
use viewer::ai::{Provider, Session};

#[test]
fn default_is_off_and_all_providers_are_unverified() {
    let session = Session::default();
    let catalog = session.catalog();
    assert_eq!(catalog["enabled"], false);
    assert!(catalog["selected"].is_null());
    assert_eq!(catalog["analysis_available"], false);
    let providers = catalog["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 5);
    for (entry, expected) in providers
        .iter()
        .zip(["openai", "chatgpt", "gemini", "claude", "grok"])
    {
        assert_eq!(entry["id"], expected);
        assert_eq!(entry["configured"], false);
        assert_eq!(entry["verified"], false);
        let provider = Provider::from_id(expected).unwrap();
        assert!(provider.endpoint().starts_with("https://"));
        assert!(provider.endpoint().ends_with(if expected == "chatgpt" {
            "/responses"
        } else {
            "/chat/completions"
        }));
    }
    assert!(Provider::from_id("kiro").is_none());
    assert!(Provider::from_id("https://untrusted.invalid").is_none());
}

#[test]
fn keys_are_independent_never_serialized_and_forget_does_not_switch_providers() {
    let mut session = Session::default();
    for provider in ["openai", "gemini", "claude", "grok"] {
        session
            .configure(
                provider,
                "model-test",
                format!("secret-sentinel-{provider}"),
            )
            .unwrap();
    }
    let catalog = session.catalog();
    assert!(!catalog.to_string().contains("sentinel"));
    assert_eq!(catalog["selected"], "grok");
    assert!(catalog["providers"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["configured"] == (entry["id"] != "chatgpt")));
    session.forget("openai").unwrap();
    assert_eq!(session.catalog()["selected"], "grok");
    assert_eq!(session.catalog()["enabled"], true);
    session.forget("grok").unwrap();
    assert!(session.catalog()["selected"].is_null());
    assert_eq!(session.catalog()["enabled"], false);
    assert!(session.set_enabled(true).is_err());
    assert_eq!(
        session.catalog()["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == "gemini")
            .unwrap()["configured"],
        true
    );
}

#[test]
fn disable_and_model_change_retain_the_key_but_a_new_session_does_not() {
    let mut session = Session::default();
    session
        .configure("openai", "model-one", "secret-sentinel".into())
        .unwrap();
    session.set_enabled(false).unwrap();
    assert_eq!(session.catalog()["providers"][0]["configured"], true);
    session.set_enabled(true).unwrap();
    session
        .configure("openai", "model-two", String::new())
        .unwrap();
    assert_eq!(session.catalog()["providers"][0]["model"], "model-two");
    assert!(session
        .configure("gemini", "model-two", String::new())
        .is_err());
    assert_eq!(
        Session::default().catalog()["providers"][0]["configured"],
        false
    );
}

#[test]
fn invalid_submissions_are_bounded_transactional_and_do_not_echo_keys() {
    let mut session = Session::default();
    session
        .configure("openai", "model-one", "secret-sentinel".into())
        .unwrap();
    let original = session.catalog();
    for (provider, model, key) in [
        ("unknown", "model", "sentinel".to_string()),
        ("openai", "", "sentinel".to_string()),
        ("openai", "bad\r\nmodel", "sentinel".to_string()),
        (
            "openai",
            "model",
            "sentinel\r\nAuthorization:bad".to_string(),
        ),
        ("openai", "model", "sentinel with spaces".to_string()),
        ("openai", "model", "x".repeat(4097)),
    ] {
        let error = session.configure(provider, model, key).unwrap_err();
        assert!(!error.message.contains("sentinel"));
        assert_eq!(session.catalog(), original);
    }
    assert!(session
        .configure("openai", &"x".repeat(129), "sentinel".into())
        .is_err());
    assert_eq!(session.catalog(), original);
    assert!(session.forget("unknown").is_err());
    assert_eq!(session.catalog(), original);
}

#[test]
fn key_files_accept_single_line_and_reject_unsafe_or_oversized_input() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("credential.txt");
    for ending in ["", "\n", "\r\n"] {
        std::fs::write(&path, format!("synthetic-file-key{ending}")).unwrap();
        let key = viewer::ai::read_key_file(&path).unwrap();
        assert_eq!(key.as_str(), "synthetic-file-key");
    }
    for content in [
        b"".as_slice(),
        b"two\nlines",
        b" key",
        b"key ",
        b"key\0",
        b"\xff",
        b"key\n\n",
    ] {
        std::fs::write(&path, content).unwrap();
        let error = viewer::ai::read_key_file(&path).err().unwrap();
        assert!(!error.message.contains(path.to_str().unwrap()));
    }
    std::fs::write(&path, vec![b'k'; 4096]).unwrap();
    assert!(viewer::ai::read_key_file(&path).is_ok());
    std::fs::write(&path, vec![b'k'; 4097]).unwrap();
    assert!(viewer::ai::read_key_file(&path).is_err());
    assert!(viewer::ai::read_key_file(directory.path()).is_err());
    assert!(viewer::ai::read_key_file(&directory.path().join("missing")).is_err());
}
