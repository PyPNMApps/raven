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
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

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

    /// Identifies and preflights a complete selection before replacing any inputs.
    /// # Arguments
    /// * `paths` - Two or three local captures, in any order; names are ignored.
    /// * `tolerance_seconds` - Maximum capture association separation in seconds.
    /// # Returns
    /// Catalog of compatible originals; omitted channel estimation clears old H.
    /// # Errors
    /// Rejects missing, duplicate, unsupported, malformed or incompatible captures.
    /// Existing inputs remain unchanged on every failure; no simulation is run.
    pub fn select_many(
        &mut self,
        paths: &[PathBuf],
        tolerance_seconds: u32,
    ) -> Result<Value, Diagnostic> {
        if !(2..=3).contains(&paths.len()) {
            return Err(Diagnostic::new("InputCount", "Select a modulation profile, RxMER and optional channel estimation (2 or 3 files)."));
        }
        let mut next = Self::default();
        for path in paths {
            let capture = Arc::new(Capture::read(path)?);
            let slot = match capture.decoded()? {
                DecodedPnm::ModulationProfile(_) => &mut next.profile,
                DecodedPnm::RxMer(_) => &mut next.rxmer,
                DecodedPnm::ChannelEstimate(_) => &mut next.channel,
                _ => {
                    return Err(Diagnostic::new(
                        "CaptureType",
                        format!(
                        "{}: select only modulation profile, RxMER or channel estimation captures.",
                        path.display()
                    ),
                    ))
                }
            };
            if slot.is_some() {
                return Err(Diagnostic::new(
                    "DuplicateInput",
                    format!(
                        "{}: more than one capture has the same input type.",
                        path.display()
                    ),
                ));
            }
            *slot = Some(capture);
        }
        let catalog = next.catalog()?;
        let profile_id = next
            .profile
            .as_ref()
            .map(|profile| FecExperiment::profile_ids(profile))
            .transpose()
            .map_err(diagnostic)?
            .and_then(|ids| ids.first().copied())
            .ok_or_else(|| {
                Diagnostic::new(
                    "MissingInput",
                    "Select a modulation profile containing at least one profile.",
                )
            })?;
        Session::prepare(next.clone(), profile_id, false, tolerance_seconds)?;
        *self = next;
        Ok(catalog)
    }

    /// Creates run inputs without detaching an ignored channel from the selection.
    /// # Arguments
    /// * `ignore_channel` - Use unity for this evaluation when true.
    /// # Returns
    /// Shared immutable snapshots; the attached originals remain unchanged.
    pub fn for_evaluation(&self, ignore_channel: bool) -> Self {
        let mut inputs = self.clone();
        if ignore_channel {
            inputs.channel = None;
        }
        inputs
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
    pub(super) channel_ignored: bool,
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
            channel_ignored: false,
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

    /// Freezes effective settings from the numerical owner for AI evidence.
    /// # Returns
    /// Explicit units and setting origins; no paths, MACs or sample arrays.
    /// # Errors
    /// Preserves engine configuration failures.
    pub fn ai_settings(&self) -> Result<crate::ai::evidence::Settings, Diagnostic> {
        let configuration = self.experiment.configuration().map_err(diagnostic)?;
        Ok(crate::ai::evidence::Settings {
            profile_id: self.profile_id,
            receiver_mode: if self.mce {
                "MCE"
            } else if self.inputs.channel.is_some() {
                "SuppliedChannel"
            } else {
                "Unity"
            }
            .into(),
            channel_state: if self.channel_ignored {
                "Ignored"
            } else if self.inputs.channel.is_some() {
                "Used"
            } else {
                "NotUsed"
            }
            .into(),
            observation_ms: OBSERVATION_MS,
            runtime_limit_ms: RUNTIME_MS,
            bandwidth_hz: configuration.bandwidth().value().value(),
            zero_frequency_hz: configuration.grid().zero_frequency_hz,
            first_subcarrier_index: configuration.grid().first_subcarrier_index,
            spacing_hz: configuration.grid().spacing_hz,
            subcarrier_count: u32::try_from(configuration.grid().sample_count)
                .map_err(diagnostic)?,
            cyclic_prefix_samples: configuration.cyclic_prefix().value().value(),
            roll_off_samples: configuration.roll_off().value().value(),
            interleaver_depth: configuration.interleaver_depth().value().value(),
            bandwidth_origin: format!("{:?}", configuration.bandwidth().origin()),
            cyclic_prefix_origin: format!("{:?}", configuration.cyclic_prefix().origin()),
            roll_off_origin: format!("{:?}", configuration.roll_off().origin()),
            interleaver_origin: format!("{:?}", configuration.interleaver_depth().origin()),
        })
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
