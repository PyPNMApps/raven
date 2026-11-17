/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Single-channel FEC experiments backed by MERLIN's pinned public Rust API.
//! Preparation never runs simulation. Call execution on a worker, not Qt's thread.
pub mod batch;
pub mod workspace;
use crate::{Capture, Diagnostic, MAX_INPUT_BYTES};
use pnm::pnm::{data_type::ProfileId, parser::pnm_file::DecodedPnm};
use std::fmt;

/// MERLIN owns settings, quantities, cancellation, reports and scientific errors.
pub use merlin::api as engine;

/// Exact upstream revision used by the optional dependency and compatibility tests.
pub const MERLIN_REVISION: &str = "096d5a476194d4f2e016863d723b225257bb6037";

/// Keep parser diagnostics and MERLIN's structured errors distinguishable.
#[derive(Debug)]
pub enum FecError {
    /// Original RAVEN parsing/serialization failure, including field/offset.
    Capture(Diagnostic),
    /// Original MERLIN category, message and optional engine diagnostic.
    Engine(engine::ApiError),
}

/// Explicit single-profile settings; all physical defaults come from MERLIN.
#[derive(Debug, Clone)]
pub struct FecSettings {
    /// Captured profile ID, not a modulation order.
    pub profile_id: ProfileId,
    /// DOCSIS interpretation chosen by the caller.
    pub protocol: engine::ProtocolScope,
    /// Explicit overrides for geometry not uniquely recoverable from the profile.
    pub ocd_overrides: engine::OcdOverrides,
    /// Runtime/observation/receiver settings. Source selections are set by this adapter.
    pub options: engine::EvaluationOptions,
}

/// Owned, preflighted session independent of the borrowed RAVEN captures.
pub struct FecExperiment {
    simulation: engine::Simulation,
}

impl FecExperiment {
    /// Loads three captures and preflights one explicitly selected profile.
    /// # Arguments
    /// * `profile`, `rxmer`, `channel_estimate` - Unchanged RAVEN captures.
    /// * `settings` - Profile, protocol, geometry, time tolerance and work bounds.
    /// # Returns
    /// An idle owned session; no worker or simulation starts during construction.
    /// # Errors
    /// Preserves import diagnostics and MERLIN identity/grid/time/settings failures.
    pub fn new(
        profile: &Capture,
        rxmer: &Capture,
        channel_estimate: &Capture,
        settings: FecSettings,
    ) -> Result<Self, FecError> {
        Self::with_optional_channel(profile, rxmer, Some(channel_estimate), settings)
    }

    /// Prepares a profile with measured RxMER and an optional supplied channel.
    /// # Arguments
    /// * `profile`, `rxmer` - Original profile and RxMER captures on one grid.
    /// * `channel_estimate` - Supplied H, or None for an explicit unity channel.
    /// * `settings` - Physical settings, receiver policy and association tolerance.
    /// # Returns
    /// Idle validated experiment. No channel capture means no invented measured H.
    /// # Errors
    /// Rejects incompatible sources, invalid policies and MCE without supplied H.
    pub fn with_optional_channel(
        profile: &Capture,
        rxmer: &Capture,
        channel_estimate: Option<&Capture>,
        settings: FecSettings,
    ) -> Result<Self, FecError> {
        if channel_estimate.is_none()
            && settings.options.receiver_mode == engine::ReceiverMode::MerlinChannelEstimated
        {
            return Err(
                Diagnostic::new("MissingChannel", "Attach channel estimation to use MCE.").into(),
            );
        }
        let (original_profile, decoded_profile) = sources(profile)?;
        let dpd = engine::Dpd::from_decoded(
            &original_profile,
            &decoded_profile,
            engine::ProfileId::new(settings.profile_id)?,
            None,
        )?;
        let (original_rxmer, decoded_rxmer) = sources(rxmer)?;
        let mer = engine::RxMer::from_decoded(&original_rxmer, &decoded_rxmer, None)?;
        let channel = channel_estimate
            .map(|capture| {
                let (original, decoded) = sources(capture)?;
                Ok::<_, FecError>(engine::ChannelEstimate::from_decoded(
                    &original, &decoded, None,
                )?)
            })
            .transpose()?;
        Self::from_inputs(dpd, mer, channel, settings)
    }

    /// Admits three bounded decoded JSON inputs without inventing binary originals.
    /// # Arguments
    /// * `profile`, `rxmer`, `channel_estimate` - Original named JSON captures.
    /// * `settings` - Explicit selected profile and engine settings.
    /// # Returns
    /// Idle preflighted experiment; JSON is retained as its actual source format.
    /// # Errors
    /// Preserves MERLIN format, profile, grid, identity, time and settings errors.
    pub fn from_json(
        profile: &engine::NamedCapture,
        rxmer: &engine::NamedCapture,
        channel_estimate: &engine::NamedCapture,
        settings: FecSettings,
    ) -> Result<Self, FecError> {
        let dpd = engine::Dpd::from_bytes(profile, engine::ProfileId::new(settings.profile_id)?)?;
        let mer = engine::RxMer::from_bytes(rxmer)?;
        let channel = engine::ChannelEstimate::from_bytes(channel_estimate)?;
        Self::from_inputs(dpd, mer, Some(channel), settings)
    }

    /// Discovers captured profile IDs without running or guessing a profile.
    /// # Arguments
    /// * `profile` - A RAVEN modulation-profile capture.
    /// # Returns
    /// Sorted captured IDs; loading and supported simulation admission remain separate.
    /// # Errors
    /// Preserves parser errors and MERLIN catalog validation failures.
    pub fn profile_ids(profile: &Capture) -> Result<Vec<ProfileId>, FecError> {
        let (_, decoded) = sources(profile)?;
        Ok(engine::Dpd::profile_ids_from_bytes(&decoded)?
            .into_iter()
            .map(engine::ProfileId::value)
            .collect())
    }

    /// Borrows actual resolved geometry and its explicit/defaulted/derived origins.
    /// # Returns
    /// MERLIN's immutable resolved configuration; no UI-side calculations.
    /// # Errors
    /// Propagates MERLIN configuration diagnostics.
    pub fn configuration(&self) -> Result<&engine::ResolvedConfiguration, FecError> {
        Ok(self.simulation.ocd()?.configuration())
    }

    /// Reads the engine's protected PLC neighborhood for consistent plot shading.
    /// # Returns
    /// Half-open absolute Hz range from the validated MERLIN physical layout.
    /// # Errors
    /// Preserves invalid configuration, layout or frequency arithmetic errors.
    pub fn plc_neighborhood(
        &self,
    ) -> Result<std::ops::Range<pnm::pnm::data_type::FrequencyHz>, FecError> {
        let configuration = self.configuration()?;
        let spacing = [
            engine::SubcarrierSpacing::Khz25,
            engine::SubcarrierSpacing::Khz50,
        ]
        .into_iter()
        .find(|spacing| spacing.frequency_hz() == configuration.grid().spacing_hz)
        .ok_or_else(|| Diagnostic::new("Spacing", "Unsupported OFDM spacing."))?;
        let transmission = merlin::transmission::TransmissionSettings {
            occupied_bandwidth_hz: configuration.bandwidth().value().value(),
            subcarrier_spacing: spacing,
            cyclic_prefix_samples: configuration.cyclic_prefix().value().value(),
            roll_off_samples: configuration.roll_off().value().value(),
            time_interleaver_depth: configuration.interleaver_depth().value().value(),
        };
        let layout =
            merlin::transmission::layout::CarrierLayout::new(&transmission, configuration.layout())
                .map_err(engine::ApiError::from)?;
        Ok(layout
            .plc_neighborhood_hz()
            .map_err(engine::ApiError::from)?)
    }

    /// Executes the existing PRBS/TX/channel/RX/FEC pipeline on the caller's worker.
    /// # Arguments
    /// * `control` - Cloneable cooperative cancellation handle for another thread.
    /// # Returns
    /// Owned MERLIN batch report, retaining partial/not-started states and FEC scopes.
    /// Completion is not a promise of zero bit errors; missing metrics stay absent.
    /// # Errors
    /// Preserves MERLIN errors; started failures remain in the native report.
    pub fn run(&mut self, control: &engine::RunControl) -> Result<engine::BatchReport, FecError> {
        Ok(self.simulation.run_with_control(control)?)
    }

    fn from_inputs(
        dpd: engine::Dpd,
        mer: engine::RxMer,
        channel: Option<engine::ChannelEstimate>,
        settings: FecSettings,
    ) -> Result<Self, FecError> {
        let mut simulation = engine::Simulation::new();
        simulation.load_profiles_with_options(
            settings.protocol,
            vec![dpd],
            settings.ocd_overrides,
        )?;
        let rxmer_id = simulation.add_rxmer(mer)?;
        let channel_selection = match channel {
            Some(channel) => {
                engine::ChannelSelection::Explicit(simulation.add_channel_estimate(channel)?)
            }
            None => engine::ChannelSelection::Unity,
        };
        simulation.set_options(
            settings
                .options
                .with_rxmer_selection(engine::RxMerSelection::Individual(rxmer_id))
                .with_channel_selection(channel_selection),
        )?;
        simulation.prepare()?;
        Ok(Self { simulation })
    }
}

fn sources(capture: &Capture) -> Result<(engine::NamedCapture, engine::NamedCapture), FecError> {
    let decoded = capture.decoded()?;
    if !matches!(
        decoded,
        DecodedPnm::ModulationProfile(_) | DecodedPnm::RxMer(_) | DecodedPnm::ChannelEstimate(_)
    ) {
        return Err(Diagnostic::new(
            "UnsupportedSimulationInput",
            "Select a modulation-profile, RxMER or downstream channel-estimate capture.",
        )
        .into());
    }
    let json = decoded
        .to_json(MAX_INPUT_BYTES)
        .map_err(|error| Diagnostic::new("Serialization", error))?;
    let (name, bytes) = capture.original_source();
    Ok((
        engine::NamedCapture::new(name, bytes.to_vec())?,
        engine::NamedCapture::new(format!("{name}.json"), json.into_bytes())?,
    ))
}

impl From<Diagnostic> for FecError {
    /// Preserves the parser/application diagnostic without flattening its fields.
    /// # Arguments
    /// * `error` - Original RAVEN diagnostic.
    /// # Returns
    /// Capture error variant with unchanged diagnostic.
    fn from(error: Diagnostic) -> Self {
        Self::Capture(error)
    }
}
impl From<engine::ApiError> for FecError {
    /// Retains the original structured MERLIN error.
    /// # Arguments
    /// * `error` - Original engine error.
    /// # Returns
    /// Engine error variant with unchanged category and details.
    fn from(error: engine::ApiError) -> Self {
        Self::Engine(error)
    }
}
impl fmt::Display for FecError {
    /// Formats human context while callers retain access to structured variants.
    /// # Arguments
    /// * `formatter` - Destination formatter.
    /// # Returns
    /// Formatting completion or the destination's error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(error) => write!(formatter, "{}: {}", error.code, error.message),
            Self::Engine(error) => fmt::Display::fmt(error, formatter),
        }
    }
}
impl std::error::Error for FecError {}
