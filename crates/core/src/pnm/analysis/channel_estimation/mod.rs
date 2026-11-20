/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Compatible temporal channel estimates: average complex coefficients before delay.
use crate::{
    common::signal_processing::channel_response::{group_delay_seconds_first_order, magnitude_db},
    pnm::{
        data_type::{
            units::HZ_PER_KHZ, CaptureTime, ChannelId, FileVersionComponent, FrequencyHz, IqPair,
            MacAddress, SubcarrierIdx,
        },
        parser::{
            cm_ds_ofdm_chan_estimate_coef::CmDsOfdmChanEstimateCoef, parse_limits::ParseLimits,
            pnm_header::HasPnmHeader,
        },
    },
};
use serde::Serialize;

/// Exact device, format and carrier grid required for coherent averaging.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct GroupKey {
    #[serde(serialize_with = "crate::pnm::data_type::serialize_mac")]
    pub mac_address: MacAddress,
    pub channel_id: ChannelId,
    pub major_version: FileVersionComponent,
    pub minor_version: FileVersionComponent,
    pub zero_frequency_hz: FrequencyHz,
    pub first_active_subcarrier_index: SubcarrierIdx,
    pub spacing_hz: FrequencyHz,
    pub sample_count: usize,
}
impl GroupKey {
    /// Validates and expands the exact carrier centers from binary metadata.
    /// # Returns
    /// Increasing u32 Hz centers; no frequency alignment or resampling.
    /// # Errors
    /// Rejects empty/zero-spacing grids, excessive carrier counts and overflow.
    pub fn frequencies_hz(&self) -> Result<Vec<FrequencyHz>, &'static str> {
        if self.sample_count < 2
            || self.sample_count > ParseLimits::default().max_samples
            || self.spacing_hz == 0
        {
            return Err("Channel estimates require at least two carriers and positive spacing.");
        }
        (0..self.sample_count)
            .map(|index| {
                let frequency = u64::from(self.zero_frequency_hz)
                    + (u64::from(self.first_active_subcarrier_index) + index as u64)
                        * u64::from(self.spacing_hz);
                FrequencyHz::try_from(frequency)
                    .map_err(|_| "Channel estimate frequency grid overflows.")
            })
            .collect()
    }
}
/// Owned decoded snapshot; filenames are provenance only.
#[derive(Debug, Clone)]
pub struct Series {
    pub key: GroupKey,
    pub source: String,
    pub capture_time: CaptureTime,
    pub iq: Vec<Option<IqPair>>,
}
impl Series {
    /// Copies coefficients and validated header metadata into a collection snapshot.
    /// # Arguments
    /// * `capture` - Validated channel-estimation PNM file.
    /// * `source` - Display/provenance name; never used to infer identity or epoch.
    /// # Returns
    /// Owned I/Q preserving excluded carriers.
    /// # Errors
    /// Rejects absent capture time and invalid or overflowing grids.
    pub fn from_capture(
        capture: &CmDsOfdmChanEstimateCoef<'_>,
        source: String,
    ) -> Result<Self, &'static str> {
        let version = capture.format_version();
        let iq: Vec<_> = capture.iq().collect();
        let key = GroupKey {
            mac_address: capture.mac_address,
            channel_id: capture.channel_id,
            major_version: version.major,
            minor_version: version.minor,
            zero_frequency_hz: capture.subcarrier_zero_frequency_hz,
            first_active_subcarrier_index: capture.first_active_subcarrier_index,
            spacing_hz: u32::from(capture.subcarrier_spacing_khz) * HZ_PER_KHZ,
            sample_count: iq.len(),
        };
        key.frequencies_hz()?;
        Ok(Self {
            key,
            source,
            capture_time: capture
                .capture_time()
                .ok_or("Channel estimate has no capture time.")?,
            iq,
        })
    }
}
/// Chronology/provenance for one accepted snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub name: String,
    pub capture_time: CaptureTime,
}
/// Full-resolution values. Group delay is derived from average_iq, not averaged delays.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelEstimateAggregate {
    pub group: GroupKey,
    pub sources: Vec<Source>,
    pub frequencies_hz: Vec<FrequencyHz>,
    pub average_iq: Vec<Option<IqPair>>,
    pub valid_counts: Vec<usize>,
    pub minimum_magnitude_db: Vec<Option<f64>>,
    pub average_magnitude_db: Vec<Option<f64>>,
    pub maximum_magnitude_db: Vec<Option<f64>>,
    pub group_delay_s: Vec<Option<f64>>,
}
impl ChannelEstimateAggregate {
    /// Averages finite I and Q independently at each compatible carrier center.
    /// # Arguments
    /// * `captures` - Nonempty snapshots on exactly the same device/channel/grid.
    /// # Returns
    /// Complex mean, magnitude envelope, counts and delay in seconds in epoch order.
    /// Missing pairs do not contribute. Zero complex means remain zero and yield
    /// undefined magnitude/delay; gaps split phase-unwrapping runs.
    /// # Errors
    /// Rejects incompatible grids, empty collections and nonfinite coefficients.
    pub fn calculate(captures: &[&Series]) -> Result<Self, &'static str> {
        let first = captures
            .first()
            .ok_or("Select channel-estimation captures.")?;
        let frequencies_hz = first.key.frequencies_hz()?;
        if captures
            .iter()
            .any(|capture| capture.key != first.key || capture.iq.len() != first.key.sample_count)
        {
            return Err("Channel-estimation captures have incompatible metadata or carrier grids.");
        }
        let mut ordered = captures.to_vec();
        ordered.sort_by(|a, b| (a.capture_time, &a.source).cmp(&(b.capture_time, &b.source)));
        let count = first.key.sample_count;
        let mut average_iq = vec![None; count];
        let mut valid_counts = vec![0usize; count];
        let mut minimum_magnitude_db = vec![None::<f64>; count];
        let mut maximum_magnitude_db = vec![None::<f64>; count];
        for capture in &ordered {
            for (index, sample) in capture.iq.iter().enumerate() {
                if let Some([i, q]) = sample {
                    if !i.is_finite() || !q.is_finite() {
                        return Err("Channel-estimation coefficients must be finite.");
                    }
                    valid_counts[index] += 1;
                    let mean = average_iq[index].get_or_insert([0.0, 0.0]);
                    mean[0] += (i - mean[0]) / valid_counts[index] as f64;
                    mean[1] += (q - mean[1]) / valid_counts[index] as f64;
                    if !mean.iter().all(|value| value.is_finite()) {
                        return Err("Channel-estimation complex mean overflows.");
                    }
                    if let Some(value) = magnitude_db(*sample) {
                        minimum_magnitude_db[index] =
                            Some(minimum_magnitude_db[index].map_or(value, |old| old.min(value)));
                        maximum_magnitude_db[index] =
                            Some(maximum_magnitude_db[index].map_or(value, |old| old.max(value)));
                    }
                }
            }
        }
        let average_magnitude_db = average_iq
            .iter()
            .map(|sample| magnitude_db(*sample))
            .collect();
        let group_delay_s = group_delay_seconds_first_order(&average_iq, first.key.spacing_hz);
        Ok(Self {
            group: first.key.clone(),
            sources: ordered
                .iter()
                .map(|capture| Source {
                    name: capture.source.clone(),
                    capture_time: capture.capture_time,
                })
                .collect(),
            frequencies_hz,
            average_iq,
            valid_counts,
            minimum_magnitude_db,
            average_magnitude_db,
            maximum_magnitude_db,
            group_delay_s,
        })
    }
}
