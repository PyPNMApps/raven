/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Native workspace preparation and bounded chart/results DTOs, without DSP copies.
use super::{engine, FecExperiment, FecSettings};
use crate::{
    capture::chart::{ChartViewport, MeasurementView},
    measurements::modulation_profile::{CarrierType, ModulationMetric},
    Capture, Diagnostic,
};
use pnm::pnm::{data_type::ProfileId, parser::pnm_file::DecodedPnm};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

/// Explicit native-workspace observation and host-work budgets, in milliseconds.
pub const OBSERVATION_MS: u64 = 10;
pub const RUNTIME_MS: u64 = 300_000;

/// Immutable original capture snapshots; selecting paths never modifies files.
#[derive(Clone, Default)]
pub struct Inputs {
    pub profile: Option<Arc<Capture>>,
    pub rxmer: Option<Arc<Capture>>,
    pub channel: Option<Arc<Capture>>,
}
impl Inputs {
    /// Reads one bounded binary PNM capture and validates its selected role.
    /// # Arguments
    /// * `role` - profile, rxmer or channel.
    /// * `path` - Local native file-picker destination.
    /// # Returns
    /// Updated snapshot and a small catalog. Prior inputs survive failed reads.
    /// # Errors
    /// Rejects file I/O, malformed PNM, unsupported roles and wrong capture kinds.
    pub fn select(&mut self, role: &str, path: &Path) -> Result<Value, Diagnostic> {
        let capture = Arc::new(Capture::read(path)?);
        let decoded = capture.decoded()?;
        let valid = matches!(
            (role, decoded),
            ("profile", DecodedPnm::ModulationProfile(_))
                | ("rxmer", DecodedPnm::RxMer(_))
                | ("channel", DecodedPnm::ChannelEstimate(_))
        );
        if !valid {
            return Err(Diagnostic::new(
                "CaptureType",
                "Select the requested PNM file type.",
            ));
        }
        let mut next = self.clone();
        match role {
            "profile" => next.profile = Some(capture),
            "rxmer" => next.rxmer = Some(capture),
            "channel" => next.channel = Some(capture),
            _ => unreachable!(),
        }
        let catalog = next.catalog()?;
        *self = next;
        Ok(catalog)
    }

    /// Describes selected originals and captured profile IDs without sample arrays.
    /// # Returns
    /// Bounded filename/ID DTO for the input dialog.
    /// # Errors
    /// Propagates profile decoding or MERLIN catalog errors.
    pub fn catalog(&self) -> Result<Value, Diagnostic> {
        let name = |capture: &Option<Arc<Capture>>| {
            capture
                .as_ref()
                .map(|capture| capture.original_source().0.to_string())
        };
        let ids = self
            .profile
            .as_ref()
            .map(|profile| FecExperiment::profile_ids(profile))
            .transpose()
            .map_err(diagnostic)?
            .unwrap_or_default();
        Ok(
            json!({"profile": name(&self.profile), "rxmer": name(&self.rxmer),
            "channel": name(&self.channel), "profiles": ids}),
        )
    }
}

/// A preflighted MERLIN experiment and immutable capture-backed plot sources.
pub struct Session {
    experiment: FecExperiment,
    inputs: Inputs,
    profile_index: usize,
    pub profile_id: ProfileId,
    mce: bool,
}
impl Session {
    /// Preflights one profile before replacing an existing native workspace.
    /// # Arguments
    /// * `inputs` - Required profile/RxMER and optional channel snapshots.
    /// * `profile_id` - Captured profile ID, not dropdown index.
    /// * `mce` - Received-pilot MERLIN estimation; requires supplied channel.
    /// * `tolerance_seconds` - Explicit maximum profile association separation.
    /// # Returns
    /// Idle session with validated identity, grid and engine geometry.
    /// # Errors
    /// Rejects missing/mismatched inputs, invalid profile or unsupported settings.
    pub fn prepare(
        inputs: Inputs,
        profile_id: ProfileId,
        mce: bool,
        tolerance_seconds: u32,
    ) -> Result<Self, Diagnostic> {
        let profile = inputs
            .profile
            .as_ref()
            .ok_or_else(|| Diagnostic::new("MissingInput", "Select a modulation profile."))?;
        let rxmer = inputs
            .rxmer
            .as_ref()
            .ok_or_else(|| Diagnostic::new("MissingInput", "Select an RxMER capture."))?;
        let options = engine::EvaluationOptions {
            observation: engine::ObservationDuration::new(OBSERVATION_MS).map_err(diagnostic)?,
            limits: engine::RunLimits {
                runtime: engine::RuntimeLimit::new(RUNTIME_MS).map_err(diagnostic)?,
                ..Default::default()
            },
            profile_tolerance: engine::AssociationTolerance::new(tolerance_seconds)
                .map_err(diagnostic)?,
            receiver_mode: if mce {
                engine::ReceiverMode::MerlinChannelEstimated
            } else {
                engine::ReceiverMode::CmChannelEstimated
            },
            ..Default::default()
        };
        let settings = FecSettings {
            profile_id,
            protocol: engine::ProtocolScope::Docsis31,
            ocd_overrides: engine::OcdOverrides::default(),
            options,
        };
        let experiment = FecExperiment::with_optional_channel(
            profile,
            rxmer,
            inputs.channel.as_deref(),
            settings,
        )
        .map_err(diagnostic)?;
        let DecodedPnm::ModulationProfile(decoded) = profile.decoded()? else {
            unreachable!()
        };
        let profile_index = decoded
            .profiles
            .iter()
            .position(|profile| profile.profile_id == profile_id)
            .ok_or_else(|| Diagnostic::new("Profile", "Profile ID is absent."))?;
        Ok(Self {
            experiment,
            inputs,
            profile_index,
            profile_id,
            mce,
        })
    }

    /// Creates a decimated overlay with exact source ranges and PLC spans.
    /// # Arguments
    /// * `start`, `count` - Original RxMER sample window; zero count means full.
    /// # Returns
    /// Rust-computed dB references and RxMER with at most two viewport budgets.
    /// # Errors
    /// Rejects invalid ranges and inconsistent decoded frequency grids.
    pub fn chart(&self, start: usize, count: usize) -> Result<Value, Diagnostic> {
        let rxmer = self.inputs.rxmer.as_ref().expect("validated RxMER");
        let profile = self.inputs.profile.as_ref().expect("validated profile");
        let metadata = rxmer.chart_metadata()?;
        let total = metadata.sample_count;
        let count = if count == 0 { total } else { count };
        let measured = rxmer.chart_range_for(MeasurementView::Default, start, count)?;
        let assigned: ChartViewport = profile.chart_range_for(
            MeasurementView::ModulationProfile {
                indices: Some(vec![self.profile_index]),
                carrier_types: Some(vec![CarrierType::Data]),
                metric: ModulationMetric::Shannon,
            },
            start,
            count,
        )?;
        let overlays = assigned
            .modulation_profiles
            .ok_or_else(|| Diagnostic::new("Chart", "No profile trace."))?;
        let protected_plc = self.experiment.plc_neighborhood().map_err(diagnostic)?;
        let plc = vec![json!({"first_hz": protected_plc.start, "last_hz": protected_plc.end})];
        Ok(
            json!({"viewport": measured, "profile": overlays.series.first(), "plc": plc,
            "profile_id": self.profile_id, "chips": metadata.chips, "minimum": 0, "maximum": 64,
            "bandwidth_hz": self.experiment.configuration().map_err(diagnostic)?.bandwidth().value().value(),
            "mce": self.mce, "supplied_channel": self.inputs.channel.is_some(),
            "observation_ms": OBSERVATION_MS}),
        )
    }

    /// Executes on a caller-owned worker and returns compact scoped counters.
    /// # Arguments
    /// * `control` - Cooperative stop handle, safe to clone for the UI thread.
    /// # Returns
    /// Scoped FEC, BER and all-completed LDPC statistics from MERLIN's own report.
    /// # Errors
    /// Preserves preparation/execution errors; partial statuses stay explicit.
    pub fn run(&mut self, control: &engine::RunControl) -> Result<Value, Diagnostic> {
        let batch = self.experiment.run(control).map_err(diagnostic)?;
        let entry = batch
            .iter()
            .next()
            .ok_or_else(|| Diagnostic::new("Simulation", "No experiment result."))?;
        let Some(report) = entry.report() else {
            return Ok(
                json!({"status": format!("{:?}", entry.status()), "profile_id": self.profile_id}),
            );
        };
        let report = serde_json::to_value(report).map_err(diagnostic)?;
        let summary = &report["diagnostics"]["summary"];
        Ok(
            json!({"status": report["status"], "profile_id": self.profile_id,
            "fec": report["fec"], "profile": report["profile"],
            "error": report["construction_error"], "run_error": report["diagnostics"]["error"],
            "ldpc": {"scope": "All completed", "mean_iterations": summary["payload_mean_ldpc_iterations"],
                "iteration_sum": summary["payload_ldpc_iteration_sum"], "histogram": summary["payload_ldpc_iterations"],
                "failures": summary["payload_ldpc_failures"], "ncp_iterations": summary["ncp_ldpc_iterations"],
                "plc_iterations": summary["plc_ldpc_iterations"], "bch_failures": summary["payload_bch_failures"]}}),
        )
    }
}
fn diagnostic(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new("ProfilePerformance", error.to_string())
}
