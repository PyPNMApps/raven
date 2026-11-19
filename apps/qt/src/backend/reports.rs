/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Existing capture/collection owners supply local report snapshots; QML supplies no evidence.
use super::*;
impl bridge::ViewerBackend {
    /// Captures an active single capture, RxMER time series or pre-EQ comparison.
    /// # Arguments
    /// * `scope` - Explicit finite workspace identifier, never a path or provider content.
    /// # Returns
    /// Approved local report JSON, or a safe error with no retained stale report.
    pub fn engineering_report(self: Pin<&mut Self>, scope: &QString) -> QString {
        let state = self.rust();
        let value = |json: &QString| {
            serde_json::from_str::<serde_json::Value>(&json.to_string())
                .unwrap_or(serde_json::Value::Null)
        };
        let result = match scope.to_string().as_str() {
            "single" => state
                .capture
                .as_ref()
                .ok_or_else(|| {
                    Diagnostic::new(
                        "ReportCapture",
                        "Open a PNM capture before creating its engineering report.",
                    )
                })
                .and_then(|capture| {
                    viewer::reports::adapter::from_capture(capture, state.report_generation)
                }),
            "rxmer-collection" | "rxmer-playback" => viewer::reports::adapter::from_snapshot(
                "RxMER collection / time series",
                "rxmer-collection",
                state.report_generation,
                &value(&state.rxmer_collection_view),
                viewer::reports::context::Classification::Derived,
            ),
            "pre-eq" => viewer::reports::adapter::from_snapshot(
                "US Pre-EQ comparison",
                "pre-eq",
                state.report_generation,
                &value(&state.pre_eq_json),
                viewer::reports::context::Classification::Derived,
            ),
            _ => Err(Diagnostic::new(
                "ReportWorkspace",
                "Unsupported report workspace.",
            )),
        };
        QString::from(
            result
                .unwrap_or_else(|error| serde_json::json!({"error":error.message}))
                .to_string(),
        )
    }
}
