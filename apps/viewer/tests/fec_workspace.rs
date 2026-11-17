/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Native OFDM workspace contracts, independent of Qt and chart rendering.
#![cfg(feature = "merlin-fec")]
use std::sync::Arc;
use viewer::{
    simulation::fec::{
        engine,
        workspace::{Inputs, Session},
    },
    Capture,
};

fn capture(bytes: &[u8], name: &str) -> Arc<Capture> {
    Arc::new(Capture::from_bytes(bytes.to_vec(), name.into()).unwrap())
}
fn inputs(channel: bool) -> Inputs {
    Inputs {
        profile: Some(capture(pnm_fixtures::MODULATION_PROFILE, "profile.bin")),
        rxmer: Some(capture(pnm_fixtures::RXMER, "rxmer.bin")),
        channel: channel.then(|| capture(pnm_fixtures::CHANNEL_ESTIMATE, "channel.bin")),
    }
}
#[test]
fn admits_unity_or_supplied_channel_and_bounds_the_overlay() {
    for channel in [false, true] {
        let session = Session::prepare(inputs(channel), 0, false, 86_400).unwrap();
        let chart = session.chart(0, 0).unwrap();
        assert_eq!(chart["supplied_channel"], channel);
        assert_eq!(chart["mce"], false);
        assert_eq!(chart["profile_id"], 0);
        let measured = chart["viewport"]["points"].as_array().unwrap();
        let profile = chart["profile"]["points"].as_array().unwrap();
        assert!(!measured.is_empty() && measured.len() <= viewer::MAX_VIEW_SAMPLES);
        assert!(!profile.is_empty() && profile.len() <= viewer::MAX_VIEW_SAMPLES);
        assert_eq!(
            measured.first().unwrap()["frequency_hz"],
            profile.first().unwrap()["frequency_hz"]
        );
        assert_eq!(
            measured.last().unwrap()["frequency_hz"],
            profile.last().unwrap()["frequency_hz"]
        );
        assert_eq!(
            chart["plc"][0]["last_hz"].as_u64().unwrap()
                - chart["plc"][0]["first_hz"].as_u64().unwrap(),
            6_000_000
        );
        assert!(session.chart(usize::MAX, 50).is_err());
        let zoomed = session.chart(100, 50).unwrap();
        assert_eq!(zoomed["viewport"]["start"], 100);
        assert_eq!(zoomed["viewport"]["count"], 50);
    }
}
#[test]
fn requires_sources_matching_identity_grid_and_explicit_time_tolerance() {
    assert!(Session::prepare(Inputs::default(), 0, false, 86_400).is_err());
    assert!(Session::prepare(inputs(false), 0, true, 86_400).is_err());
    assert!(Session::prepare(inputs(true), 0, true, 86_400)
        .unwrap()
        .chart(0, 0)
        .unwrap()["mce"]
        .as_bool()
        .unwrap());
    assert!(Session::prepare(inputs(false), 0, false, 0).is_err());
    assert!(Session::prepare(inputs(false), 15, false, 86_400).is_err());
    let source = pnm_fixtures::RXMER;
    let offset = pnm::pnm::parser::pnm_file::PnmFile::from_bytes(source)
        .header()
        .unwrap()
        .payload_offset();
    // Measurement payload: channel byte, six MAC bytes, then zero frequency.
    for position in [offset, offset + 1, offset + 7] {
        let mut bytes = source.to_vec();
        bytes[position] ^= 1;
        let mut request = inputs(false);
        request.rxmer = Some(capture(&bytes, "mismatched.bin"));
        assert!(Session::prepare(request, 0, false, 86_400).is_err());
    }
}
#[test]
fn role_selection_is_transactional_and_discovers_profile_ids() {
    let directory = tempfile::tempdir().unwrap();
    let wrong = directory.path().join("not-a-profile.bin");
    std::fs::write(&wrong, pnm_fixtures::RXMER).unwrap();
    let mut selected = Inputs::default();
    assert!(selected.select("profile", &wrong).is_err());
    assert!(selected.profile.is_none());
    let profile = directory.path().join("profile.bin");
    std::fs::write(&profile, pnm_fixtures::MODULATION_PROFILE).unwrap();
    assert_eq!(
        selected.select("profile", &profile).unwrap()["profiles"],
        serde_json::json!([0, 1, 2, 3])
    );
    assert!(selected.select("profile", &wrong).is_err());
    assert_eq!(selected.catalog().unwrap()["profile"], "profile.bin");
}
#[test]
fn cancelled_run_never_invents_zero_fec_or_ldpc_counters() {
    let mut session = Session::prepare(inputs(false), 0, false, 86_400).unwrap();
    let control = engine::RunControl::new();
    control.cancel();
    let result = session.run(&control).unwrap();
    assert_eq!(result["status"], "NotStarted");
    assert!(result["fec"].is_null());
    assert!(result["ldpc"].is_null());
}
