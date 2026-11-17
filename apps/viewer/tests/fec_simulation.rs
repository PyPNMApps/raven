/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Actual parser-to-MERLIN contracts; no duplicate FEC or channel math.
#![cfg(feature = "merlin-fec")]
use viewer::{
    simulation::fec::{engine, FecError, FecExperiment, FecSettings},
    Capture,
};

// The original fixtures were acquired at different epochs. Accept that explicitly.
const FIXTURE_TIME_TOLERANCE_SECONDS: u32 = 90_000;
const FIXTURE_MAX_RUNTIME_MS: u64 = 300_000;

fn settings() -> FecSettings {
    let mut options = engine::EvaluationOptions::default().with_profile_tolerance(
        engine::AssociationTolerance::new(FIXTURE_TIME_TOLERANCE_SECONDS).unwrap(),
    );
    options.observation = engine::ObservationDuration::new(1).unwrap();
    options.limits.runtime = engine::RuntimeLimit::new(FIXTURE_MAX_RUNTIME_MS).unwrap();
    FecSettings {
        profile_id: 0,
        protocol: engine::ProtocolScope::Docsis31,
        ocd_overrides: engine::OcdOverrides::default(),
        options,
    }
}
fn capture(bytes: &[u8], name: &str) -> Capture {
    Capture::from_bytes(bytes.to_vec(), name.into()).unwrap()
}
fn captures() -> (Capture, Capture, Capture) {
    (
        capture(pnm_fixtures::MODULATION_PROFILE, "profile.bin"),
        capture(pnm_fixtures::RXMER, "rxmer.bin"),
        capture(pnm_fixtures::CHANNEL_ESTIMATE, "channel.bin"),
    )
}

#[test]
fn prepares_actual_three_capture_inputs_without_changing_sources_or_exports() {
    let (profile, rxmer, channel) = captures();
    let before = [
        profile.json().unwrap(),
        rxmer.json().unwrap(),
        channel.json().unwrap(),
    ];
    assert_eq!(FecExperiment::profile_ids(&profile).unwrap(), [0, 1, 2, 3]);
    let experiment = FecExperiment::new(&profile, &rxmer, &channel, settings()).unwrap();
    assert_eq!(
        experiment
            .configuration()
            .unwrap()
            .bandwidth()
            .value()
            .value(),
        192_000_000
    );
    assert_eq!(
        before,
        [
            profile.json().unwrap(),
            rxmer.json().unwrap(),
            channel.json().unwrap()
        ]
    );
}

#[test]
fn rejects_wrong_kinds_and_absent_profiles() {
    let (profile, rxmer, channel) = captures();
    let histogram = capture(pnm_fixtures::HISTOGRAM, "histogram.bin");
    assert!(matches!(
        FecExperiment::new(&profile, &histogram, &channel, settings()),
        Err(FecError::Capture(_))
    ));
    assert!(matches!(
        FecExperiment::new(&profile, &channel, &rxmer, settings()),
        Err(FecError::Engine(_))
    ));
    let mut request = settings();
    request.profile_id = 15;
    assert!(matches!(
        FecExperiment::new(&profile, &rxmer, &channel, request),
        Err(FecError::Engine(_))
    ));
}

#[test]
fn rejects_channel_mismatch_and_does_not_silently_relax_time_tolerance() {
    let (profile, rxmer, channel) = captures();
    let mut request = settings();
    request.options.profile_tolerance = engine::AssociationTolerance::ZERO;
    assert!(matches!(
        FecExperiment::new(&profile, &rxmer, &channel, request),
        Err(FecError::Engine(_))
    ));
    let mut bytes = pnm_fixtures::RXMER.to_vec();
    let payload_offset = pnm::pnm::parser::pnm_file::PnmFile::from_bytes(&bytes)
        .header()
        .unwrap()
        .payload_offset();

    // Channel ID is the first measurement field in the RxMER payload.
    bytes[payload_offset] ^= 1;
    let other_channel = capture(&bytes, "different-channel.bin");
    assert!(matches!(
        FecExperiment::new(&profile, &other_channel, &channel, settings()),
        Err(FecError::Engine(_))
    ));
}

#[test]
fn cancelled_execution_retains_not_started_result_without_fabricated_fec() {
    let (profile, rxmer, channel) = captures();
    let mut experiment = FecExperiment::new(&profile, &rxmer, &channel, settings()).unwrap();
    let control = engine::RunControl::new();
    control.cancel();
    let batch = experiment.run(&control).unwrap();
    assert_eq!(batch.status(), engine::BatchStatus::Cancelled);
    let entries: Vec<_> = batch.iter().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].status(), engine::RunStatus::NotStarted);
    assert!(entries[0].report().is_none());
    let json: serde_json::Value = serde_json::from_str(&batch.to_json().unwrap()).unwrap();
    assert_eq!(json["schema_version"], 1);
}

#[test]
fn actual_prbs_fec_run_reports_engine_results_and_original_provenance() {
    let (profile, rxmer, channel) = captures();
    let mut experiment = FecExperiment::new(&profile, &rxmer, &channel, settings()).unwrap();
    let batch = experiment.run(&engine::RunControl::new()).unwrap();
    let entry = batch.iter().next().unwrap();
    assert_eq!(entry.profile_id().value(), 0);
    assert_eq!(entry.status(), engine::RunStatus::Complete);
    let report = entry
        .report()
        .expect("Run must retain actual executed results");
    assert!(report.all_completed_fec().total_codewords().value() > 0);
    let performance = report
        .profile()
        .expect("Completed run must have observation metrics");
    assert!(performance.compared_bits().value() > 0);
    assert_eq!(performance.bit_errors().value(), 0);
    assert_eq!(performance.post_fec_ber().unwrap().value(), 0.0);
    let json: serde_json::Value = serde_json::from_str(&batch.to_json().unwrap()).unwrap();
    assert_eq!(json["schema_version"], 1);

    // The result retains binary originals rather than claiming JSON was the input.
    use sha2::Digest;
    let original_hash = format!("{:x}", sha2::Sha256::digest(pnm_fixtures::RXMER));
    assert!(batch.to_json().unwrap().contains(&original_hash));
    assert!(batch.to_json().unwrap().contains("profile.bin"));
    assert!(batch.to_json().unwrap().contains("channel.bin"));
    eprintln!(
        "Compared bits: {}; errors: {}",
        performance.compared_bits().value(),
        performance.bit_errors().value()
    );
    eprintln!(
        "FEC status: {:?}; codewords: {}; uncorrectable: {}",
        entry.status(),
        report.all_completed_fec().total_codewords().value(),
        report.all_completed_fec().uncorrectable_codewords().value()
    );
}

#[test]
fn merlin_cl_notch_json_demo_prepares_without_relabeling_it_as_binary() {
    let profile = engine::NamedCapture::new(
        "modulation_profile.json",
        pnm_fixtures::MERLIN_CL_PROFILE.to_vec(),
    )
    .unwrap();
    let rxmer = engine::NamedCapture::new(
        "rxmer-notch-10db.json",
        pnm_fixtures::MERLIN_CL_RXMER_NOTCH.to_vec(),
    )
    .unwrap();
    let channel = engine::NamedCapture::new(
        "channel-estimation-adapted.json",
        pnm_fixtures::MERLIN_CL_CHANNEL.to_vec(),
    )
    .unwrap();
    let mut request = settings();
    request.profile_id = 3;
    request.options.profile_tolerance = engine::AssociationTolerance::ZERO;
    let mut experiment =
        FecExperiment::from_json(&profile, &rxmer, &channel, request.clone()).unwrap();
    assert_eq!(
        experiment.configuration().unwrap().grid().zero_frequency_hz,
        292_600_000
    );
    assert_eq!(
        experiment
            .configuration()
            .unwrap()
            .bandwidth()
            .value()
            .value(),
        192_000_000
    );
    let document: serde_json::Value = serde_json::from_slice(rxmer.bytes()).unwrap();
    let values = document["values"].as_array().unwrap();
    assert_eq!(values.len(), 7600);
    assert_eq!(
        values
            .iter()
            .filter(|value| value.as_f64() == Some(10.0))
            .count(),
        320
    );
    assert_eq!(values[2839], 42.0);
    assert_eq!(values[2840], 10.0);
    assert_eq!(values[3159], 10.0);
    assert_eq!(values[3160], 42.0);
    let control = engine::RunControl::new();
    control.cancel();
    let report = experiment.run(&control).unwrap();
    assert_eq!(report.status(), engine::BatchStatus::Cancelled);
    let json = report.to_json().unwrap();
    assert!(json.contains("rxmer-notch-10db.json"));
    let invalid = engine::NamedCapture::new("broken.json", b"{}".to_vec()).unwrap();
    assert!(FecExperiment::from_json(&profile, &invalid, &channel, request).is_err());
}
