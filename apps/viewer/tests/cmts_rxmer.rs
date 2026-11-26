/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Shared single-capture chart/report behavior and downstream isolation.
use viewer::Capture;
#[test]
fn upstream_uses_shared_chart_zoom_export_and_local_report() {
    let path = std::path::Path::new(pnm_fixtures::CMTS_RXMER_FIXTURE_DIRECTORY).join("golden.bin");
    let capture = Capture::read(&path).unwrap();
    let metadata = capture.chart_metadata().unwrap();
    assert_eq!(metadata.title, "Upstream OFDMA RxMER — CMTS");
    assert_eq!(metadata.unit, "dB");
    assert!(metadata
        .chips
        .iter()
        .any(|chip| chip.title == "IfIndex" && chip.value == "16909060"));
    let zoom = capture.chart_range(1, 3).unwrap();
    assert_eq!(zoom.start, 1);
    assert_eq!(zoom.count, 3);
    assert_eq!(zoom.points[1].value, Some(23.75));
    let reset = capture.chart_range(0, 5).unwrap();
    assert_eq!(reset.points[4].value, None);
    assert!(
        capture.metadata().is_err(),
        "DS metadata route must not accept upstream"
    );
    let report = viewer::reports::adapter::from_capture(&capture, 1).unwrap();
    assert!(report["prepared_question"]
        .as_str()
        .unwrap()
        .contains("upstream OFDMA"));
    assert_eq!(report["prepared_question_revision"], 1);
    let exported: serde_json::Value = serde_json::from_str(&capture.json().unwrap()).unwrap();
    assert_eq!(exported["channel_direction"], "upstream");
}
