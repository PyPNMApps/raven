/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Offline report trust-boundary and source-owner regression tests.
use serde_json::{json, Value};
use viewer::{
    reports::{self, context::Classification},
    Capture,
};
fn fixture() -> Value {
    serde_json::from_str(reports::fixtures::definition("engineering").unwrap()).unwrap()
}
fn reject(value: Value, code: &str) {
    let error =
        reports::parse(&value.to_string(), &reports::fixtures::context().unwrap()).unwrap_err();
    assert_eq!(error.code, code, "{}", error.message);
    assert!(!error.message.contains("secret-payload"));
}
#[test]
fn offline_fixtures_preserve_block_order_and_measured_values() {
    for name in ["markdown", "engineering", "rich"] {
        let model = reports::fixtures::render(name).unwrap();
        assert_eq!(model["blocks"][0]["id"], "summary");
        assert_eq!(model["classification"], "demonstration");
    }
    assert!(reports::fixtures::render("invalid").is_err());
    let model = reports::fixtures::render("engineering").unwrap();
    let chart = &model["blocks"][1];
    assert_eq!(chart["unit"], "dB");
    let capture = Capture::from_bytes(pnm_fixtures::RXMER.to_vec(), "Fixture".into()).unwrap();
    let source = capture
        .chart_range(0, capture.chart_metadata().unwrap().sample_count)
        .unwrap();
    assert_eq!(chart["viewport"], serde_json::to_value(source).unwrap());
    assert!(chart["viewport"]["points"].as_array().unwrap().len() <= 1024);
    assert_eq!(model["blocks"][2]["rows"][1][1], Value::Null);
    assert_eq!(model["blocks"][3]["action"]["enabled"], false);
}
#[test]
fn closed_schema_rejects_versions_duplicates_unknown_blocks_and_actions() {
    let mut value = fixture();
    value["schema_version"] = json!("2.0");
    reject(value, "ReportSchema");
    let mut value = fixture();
    value["blocks"][0]["type"] = json!("secret-payload.qml");
    reject(value, "ReportSchema");
    let mut value = fixture();
    value["blocks"][1]["id"] = json!("summary");
    reject(value, "ReportId");
    let mut value = fixture();
    value["blocks"][3]["action"] = json!({"type":"execute_shell","command":"secret-payload"});
    reject(value, "ReportSchema");
    let mut value = fixture();
    value["source"]["url"] = json!("https://secret-payload");
    reject(value, "ReportSchema");
    let text = reports::fixtures::definition("markdown").unwrap().replacen(
        "\"schema_version\": \"1.0\"",
        "\"schema_version\":\"1.0\",\"schema_version\":\"1.0\"",
        1,
    );
    assert_eq!(
        reports::parse(&text, &reports::fixtures::context().unwrap())
            .unwrap_err()
            .code,
        "ReportJson"
    );
}
#[test]
fn complexity_and_table_shape_are_bounded() {
    let mut value = fixture();
    value["blocks"][0]["content"] = json!("x".repeat(16_385));
    reject(value, "ReportText");
    let mut value = fixture();
    value["blocks"] = json!((0..49)
        .map(|index| json!({"id":format!("block-{index}"),"type":"markdown","content":"Text"}))
        .collect::<Vec<_>>());
    reject(value, "ReportLimit");
    let mut nested = json!({"type":"markdown","id":"leaf","content":"Text"});
    for index in 0..5 {
        nested = json!({"type":"expandable","id":format!("nested-{index}"),"title":"Details","blocks":[nested]});
    }
    let mut value = fixture();
    value["blocks"] = json!([nested]);
    reject(value, "ReportDepth");
    let mut value = fixture();
    value["blocks"] = json!([{"type":"static_table","id":"table","title":"Explanatory","columns":["One","Two"],"rows":[["Only one"]]}]);
    reject(value, "ReportTable");
    let mut value = fixture();
    value["blocks"] = json!([{"type":"static_table","id":"table","title":"Explanatory","columns":["One"],"rows":vec![vec!["value"];65]}]);
    reject(value, "ReportTable");
    assert_eq!(
        reports::parse(
            &" ".repeat(reports::MAX_REPORT_BYTES + 1),
            &reports::fixtures::context().unwrap()
        )
        .unwrap_err()
        .code,
        "ReportJson"
    );
}
#[test]
fn missing_and_stale_datasets_never_leak_old_values() {
    let mut value = fixture();
    value["blocks"][1]["dataset_id"] = json!("missing.capture");
    let context = reports::fixtures::context().unwrap();
    let result = reports::parse(&value.to_string(), &context).unwrap();
    assert_eq!(result["blocks"][1]["type"], "unavailable");
    assert!(result["blocks"][1]["viewport"].is_null());
    let mut context = reports::fixtures::context().unwrap();
    context.charts.get_mut("rxmer.demo").unwrap().generation = 99;
    let result = reports::parse(&fixture().to_string(), &context);
    assert!(result.is_err()); // The evidence navigation target is stale too.
    let mut value = fixture();
    value["blocks"][3]["action"] = Value::Null;
    assert_eq!(
        reports::parse(&value.to_string(), &context).unwrap()["blocks"][1]["type"],
        "unavailable"
    );
    let mut value = fixture();
    value["source"]["generation"] = json!(2);
    reject(value, "ReportGeneration");
    let mut value = fixture();
    value["source"]["context_id"] = json!("other-capture");
    reject(value, "ReportGeneration");
}
#[test]
fn references_assets_and_privileged_paths_fail_closed() {
    for reference in [
        "missing.evidence",
        "file:///secret-payload",
        "https://secret-payload",
    ] {
        let mut value = fixture();
        value["blocks"][3]["evidence_ids"] = json!([reference]);
        reject(value, "ReportEvidence");
    }
    for asset in [
        "../secret-payload.png",
        "file:///secret-payload",
        "https://secret-payload",
        "unknown",
    ] {
        let mut value = fixture();
        value["blocks"] = json!([{"id":"asset","type":"image","asset_id":asset,"caption":"Image"}]);
        reject(value, "ReportAsset");
    }
    let mut context = reports::fixtures::context().unwrap();
    context
        .evidence
        .get_mut("rxmer.samples")
        .unwrap()
        .generation = 5;
    assert_eq!(
        reports::parse(&fixture().to_string(), &context)
            .unwrap_err()
            .code,
        "ReportEvidence"
    );
}
#[test]
fn markdown_html_remote_images_urls_and_javascript_links_are_rejected() {
    for content in [
        "<img src='https://secret-payload'>",
        "<script>secret-payload</script>",
        "![image](https://secret-payload)",
        "![local](file:///secret-payload)",
        "[run](javascript:secret-payload)",
        "[external](https://secret-payload)",
        "[data](data:text/html,secret-payload)",
        "[unknown](#missing)",
        "[escaped](java&#x73;cript:secret-payload)",
    ] {
        let mut value = fixture();
        value["blocks"][0]["content"] = json!(content);
        assert!(
            reports::parse(&value.to_string(), &reports::fixtures::context().unwrap()).is_err()
        );
    }
    let mut value = fixture();
    value["blocks"][0]["content"] =
        json!("```javascript\nconsole.log('literal, not executed')\n```\n[Details](#details)");
    let result =
        reports::parse(&value.to_string(), &reports::fixtures::context().unwrap()).unwrap();
    assert_eq!(result["blocks"][0]["links"], json!(["#details"]));
    assert!(!result["blocks"][0]["display_content"]
        .as_str()
        .unwrap()
        .contains("](#details)"));
    assert_eq!(result["blocks"][0]["navigation"][0]["label"], "Details");
}
#[test]
fn all_single_measurements_and_collection_snapshots_share_the_renderer_contract() {
    for bytes in [
        pnm_fixtures::RXMER,
        pnm_fixtures::FEC_SUMMARY,
        pnm_fixtures::CHANNEL_ESTIMATE,
        pnm_fixtures::CONSTELLATION,
        pnm_fixtures::HISTOGRAM,
        pnm_fixtures::UPSTREAM_PRE_EQ,
        pnm_fixtures::UPSTREAM_PRE_EQ_LAST,
        pnm_fixtures::SPECTRUM,
        pnm_fixtures::MODULATION_PROFILE,
    ] {
        let capture = Capture::from_bytes(bytes.to_vec(), "Sanitized example".into()).unwrap();
        let model = reports::adapter::from_capture(&capture, 7).unwrap();
        assert_eq!(model["generation"], "7");
        assert_eq!(model["classification"], "measured");
        assert!(model["prepared_question"]
            .as_str()
            .unwrap()
            .contains("Review"));
    }
    for title in [
        "RxMER Min-Avg-Max",
        "RxMER Playback",
        "Spectrum Min-Avg-Max",
        "Spectrum Playback",
        "Multi Channel Estimation",
        "US Pre-EQ Compare",
        "OFDM Profile Performance",
    ] {
        let model = reports::adapter::from_snapshot(
            title,
            "owned-analysis",
            9,
            &json!({"sample_count":7480,"unavailable":null,"capture_time":"2026-01-01"}),
            Classification::Derived,
        )
        .unwrap();
        assert_eq!(model["blocks"][1]["rows"][0][1], "2026-01-01");
        assert!(model["blocks"][2]["values"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value["value"].is_null()));
    }
}

#[test]
fn numeric_formatting_retains_large_integer_strings_and_markdown_depth_is_bounded() {
    let model = reports::adapter::from_snapshot(
        "Precision check",
        "precision",
        1,
        &json!({"exact_bits":9007199254740993u64}),
        Classification::Simulation,
    )
    .unwrap();
    assert_eq!(model["blocks"][1]["rows"][0][1], "9007199254740993");
    let mut value = fixture();
    value["blocks"][0]["content"] = json!(format!("{}nested", "> ".repeat(18)));
    reject(value, "ReportDepth");
    assert_eq!(
        std::include_bytes!("../../../resources/reports/rxmer.bin").as_slice(),
        pnm_fixtures::RXMER
    );
}

#[test]
fn total_widget_and_plot_cost_is_bounded_across_sections() {
    let mut value = fixture();
    value["blocks"]=json!((0..3).map(|index|json!({"type":"static_table","id":format!("table-{index}"),"title":"Bounded table","columns":vec!["Column";12],"rows":vec![vec!["Value";12];64]})).collect::<Vec<_>>());
    reject(value, "ReportLimit");
    let mut value = fixture();
    value["blocks"]=json!((0..5).map(|index|json!({"type":"chart","id":format!("plot-{index}"),"chart_type":"rxmer_frequency","dataset_id":"rxmer.demo","title":"Plot"})).collect::<Vec<_>>());
    reject(value, "ReportLimit");
}
