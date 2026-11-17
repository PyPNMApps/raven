/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Checkbox selection, shared trace budgets and independently scoped run results.
#![cfg(feature = "merlin-fec")]
use std::sync::Arc;
use viewer::{
    simulation::fec::{batch::BatchSession, engine, workspace::Inputs},
    Capture,
};
fn inputs() -> Inputs {
    Inputs {
        profile: Some(Arc::new(
            Capture::from_bytes(
                pnm_fixtures::MODULATION_PROFILE.to_vec(),
                "profile.bin".into(),
            )
            .unwrap(),
        )),
        rxmer: Some(Arc::new(
            Capture::from_bytes(pnm_fixtures::RXMER.to_vec(), "rxmer.bin".into()).unwrap(),
        )),
        channel: None,
    }
}
#[test]
fn selection_is_validated_and_traces_share_a_display_budget() {
    for ids in [vec![], vec![0, 0], vec![15]] {
        assert!(BatchSession::prepare(inputs(), ids, false, 86_400).is_err());
    }
    let mut batch = BatchSession::prepare(inputs(), vec![3, 0, 1, 2], false, 86_400).unwrap();
    let chart = batch.chart(0, 0).unwrap();
    assert_eq!(chart["selected_profiles"], serde_json::json!([0, 1, 2, 3]));
    assert_eq!(chart["available_profiles"], serde_json::json!([0, 1, 2, 3]));
    let traces = chart["profiles"].as_array().unwrap();
    assert_eq!(traces.len(), 4);
    let points: usize = traces
        .iter()
        .map(|trace| trace["points"].as_array().unwrap().len())
        .sum();
    assert!(points <= viewer::MAX_VIEW_SAMPLES);
    assert!(batch.select(vec![1, 1]).is_err());
    assert!(batch.select(vec![15]).is_err());
    assert_eq!(
        batch.chart(0, 0).unwrap()["selected_profiles"],
        serde_json::json!([0, 1, 2, 3])
    );
    assert!(batch.select(vec![3, 0]).unwrap());
    let chart = batch.chart(100, 200).unwrap();
    assert_eq!(chart["selected_profiles"], serde_json::json!([0, 3]));
    assert_eq!(chart["profiles"].as_array().unwrap().len(), 2);
    assert_eq!(chart["viewport"]["count"], 200);
}
#[test]
fn empty_subset_disables_execution_but_can_be_reselected() {
    let mut batch = BatchSession::prepare(inputs(), vec![0, 3], false, 86_400).unwrap();
    assert!(!batch.select(vec![]).unwrap());
    assert!(batch.chart(0, 0).unwrap()["profiles"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(batch.run(&engine::RunControl::new()).is_err());
    assert!(batch.select(vec![3]).unwrap());
}
#[test]
fn cancellation_keeps_a_separate_not_started_row_for_every_selected_profile() {
    let mut batch = BatchSession::prepare(inputs(), vec![0, 1, 3], false, 86_400).unwrap();
    let control = engine::RunControl::new();
    control.cancel();
    let report = batch.run(&control).unwrap();
    assert_eq!(report["status"], "Cancelled");
    let entries = report["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    for (entry, profile) in entries.iter().zip([0, 1, 3]) {
        assert_eq!(entry["profile_id"], profile);
        assert_eq!(entry["status"], "NotStarted");
        assert!(entry["fec"].is_null());
    }
}
