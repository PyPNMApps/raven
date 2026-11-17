/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Bounded, independently reported profile tests sharing immutable capture inputs.
use super::{
    engine,
    workspace::{Inputs, Session},
};
use crate::{
    capture::chart::MeasurementView,
    measurements::modulation_profile::{CarrierType, ModulationMetric},
    Diagnostic,
};
use pnm::pnm::{data_type::ProfileId, parser::pnm_file::DecodedPnm};
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Finite native-workspace budget, independent of the captured profile count.
pub const MAX_TEST_PROFILES: usize = 16;

/// Preflighted profile cases with one selectable subset and no shared counters.
pub struct BatchSession {
    inputs: Inputs,
    sessions: Vec<Session>,
    selected: BTreeSet<ProfileId>,
}
impl BatchSession {
    /// Validates every requested profile before publishing any new workspace.
    /// # Arguments
    /// * `inputs` - Shared immutable binary capture snapshots.
    /// * `profiles` - Nonempty, unique captured IDs, up to MAX_TEST_PROFILES.
    /// * `mce`, `tolerance` - Same receiver and epoch-association settings for all.
    /// # Returns
    /// Idle, independently preflighted cases in ascending profile-ID order.
    /// # Errors
    /// Rejects empty/duplicate/over-budget selections or any invalid case.
    pub fn prepare(
        inputs: Inputs,
        profiles: Vec<ProfileId>,
        mce: bool,
        tolerance: u32,
    ) -> Result<Self, Diagnostic> {
        let selected: BTreeSet<_> = profiles.iter().copied().collect();
        if selected.is_empty()
            || selected.len() != profiles.len()
            || selected.len() > MAX_TEST_PROFILES
        {
            return Err(Diagnostic::new(
                "ProfileSelection",
                "Select between 1 and 16 unique profiles.",
            ));
        }
        let sessions = selected
            .iter()
            .map(|profile| {
                Session::prepare(inputs.clone(), *profile, mce, tolerance).map_err(|error| {
                    Diagnostic::new(
                        "ProfilePreparation",
                        format!("Profile {profile}: {}", error.message),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            inputs,
            sessions,
            selected,
        })
    }

    /// Changes the run/overlay subset without reinterpreting or rerunning captures.
    /// # Arguments
    /// * `profiles` - Unique prepared IDs; empty is allowed and disables execution.
    /// # Returns
    /// Whether at least one test remains selected.
    /// # Errors
    /// Rejects unknown or duplicate IDs and leaves the previous selection intact.
    pub fn select(&mut self, profiles: Vec<ProfileId>) -> Result<bool, Diagnostic> {
        let selected: BTreeSet<_> = profiles.iter().copied().collect();
        if selected.len() != profiles.len()
            || selected.iter().any(|profile| {
                !self
                    .sessions
                    .iter()
                    .any(|session| session.profile_id == *profile)
            })
        {
            return Err(Diagnostic::new(
                "ProfileSelection",
                "Choose only prepared, unique profile IDs.",
            ));
        }
        self.selected = selected;
        Ok(!self.selected.is_empty())
    }

    /// Builds a bounded multi-profile reference overlay on the shared RxMER grid.
    /// # Arguments
    /// * `start`, `count` - Source-sample window; zero count restores full range.
    /// # Returns
    /// One measured trace and a shared 1,024-point total profile-trace budget.
    /// # Errors
    /// Propagates invalid windows, malformed grids and chart failures.
    pub fn chart(&self, start: usize, count: usize) -> Result<Value, Diagnostic> {
        let mut chart = self.sessions[0].chart(start, count)?;
        let count = chart["viewport"]["count"].as_u64().expect("native count") as usize;
        let profile = self.inputs.profile.as_ref().expect("validated profile");
        let DecodedPnm::ModulationProfile(decoded) = profile.decoded()? else {
            unreachable!()
        };
        let indices = decoded
            .profiles
            .iter()
            .enumerate()
            .filter_map(|(index, profile)| {
                self.selected.contains(&profile.profile_id).then_some(index)
            })
            .collect();
        let references = profile
            .chart_range_for(
                MeasurementView::ModulationProfile {
                    indices: Some(indices),
                    carrier_types: Some(vec![CarrierType::Data]),
                    metric: ModulationMetric::Shannon,
                },
                start,
                count,
            )?
            .modulation_profiles
            .ok_or_else(|| Diagnostic::new("Chart", "Missing profile references."))?;
        let object = chart.as_object_mut().expect("native chart object");
        object.remove("profile");
        object.remove("profile_id");
        object.insert(
            "profiles".into(),
            serde_json::to_value(references.series)
                .map_err(|error| Diagnostic::new("Json", error))?,
        );
        object.insert(
            "available_profiles".into(),
            json!(self
                .sessions
                .iter()
                .map(|session| session.profile_id)
                .collect::<Vec<_>>()),
        );
        object.insert("selected_profiles".into(), json!(self.selected));
        Ok(chart)
    }

    /// Runs selected cases sequentially with one shared cooperative stop handle.
    /// # Arguments
    /// * `control` - Cancels the active case and leaves later cases not started.
    /// # Returns
    /// Separate scoped counters per profile; cases never pool BER or FEC totals.
    /// Each case retains the existing finite 300-second host limit.
    /// # Errors
    /// Rejects an empty selection. Per-case failures are retained with their ID.
    pub fn run(&mut self, control: &engine::RunControl) -> Result<Value, Diagnostic> {
        if self.selected.is_empty() {
            return Err(Diagnostic::new(
                "ProfileSelection",
                "Select at least one profile to test.",
            ));
        }
        let entries: Vec<_> = self.sessions.iter_mut().filter(|session| self.selected.contains(&session.profile_id)).map(|session| {
            session.run(control).unwrap_or_else(|error| json!({"profile_id": session.profile_id, "status":"Failed", "error":error.message}))
        }).collect();
        let status = if entries
            .iter()
            .any(|entry| entry["status"] == "Cancelled" || entry["status"] == "NotStarted")
        {
            "Cancelled"
        } else if entries.iter().any(|entry| entry["status"] != "Complete") {
            "CompletedWithErrors"
        } else {
            "Complete"
        };
        Ok(json!({"status":status, "entries":entries}))
    }
}
