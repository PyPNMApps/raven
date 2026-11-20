/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Bounded channel-estimation collections, exact grouping and source-based zoom.
use crate::capture::{collection as importer, Diagnostic};
pub use importer::ImportLimits;
pub use pnm::pnm::analysis::channel_estimation::ChannelEstimateAggregate;
use pnm::pnm::{
    analysis::channel_estimation::{GroupKey, Series, Source},
    parser::{
        cm_ds_ofdm_chan_estimate_coef::CmDsOfdmChanEstimateCoef, parse_limits::ParseLimits,
        pnm_file_type::PnmDecoderKind, pnm_header::PnmHeader,
    },
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, sync::atomic::AtomicBool};

/// Exact compatible channel and ascending binary-header chronology.
#[derive(Serialize)]
pub struct Group {
    pub id: usize,
    pub label: String,
    pub tree_label: String,
    #[serde(flatten)]
    pub key: GroupKey,
    pub capture_count: usize,
    pub sources: Vec<Source>,
}
/// Compact import accounting; measurement arrays remain in Rust.
#[derive(Serialize)]
pub struct Catalog {
    pub groups: Vec<Group>,
    pub accepted_count: usize,
    pub duplicate_count: usize,
    pub skipped_count: usize,
    pub issues: Vec<String>,
}
/// Independent archive/folder collection; filenames do not determine identity.
pub struct Collection {
    pub catalog: Catalog,
    groups: Vec<Vec<Series>>,
}
impl Collection {
    /// Imports local channel-estimation files with shared archive and memory bounds.
    /// # Arguments
    /// * `path` - Native user-selected file, folder or archive.
    /// * `limits` - Shared import resource budgets.
    /// * `cancel` - Cooperative import cancellation.
    /// # Returns
    /// Deduplicated device/channel/grid groups with ascending header epochs.
    /// # Errors
    /// Rejects cancellation, unsafe archives, exhausted limits or no usable files.
    pub fn scan(
        path: &Path,
        limits: ImportLimits,
        cancel: &AtomicBool,
    ) -> Result<Self, Diagnostic> {
        let scanned = importer::scan(path, limits, cancel, decode)?;
        if scanned.series.is_empty() {
            return Err(Diagnostic::new(
                "ChannelEstimationImport",
                "No usable channel-estimation captures found.",
            ));
        }
        let mut grouped: BTreeMap<GroupKey, Vec<Series>> = BTreeMap::new();
        for capture in scanned.series {
            grouped
                .entry(capture.key.clone())
                .or_default()
                .push(capture);
        }
        let mut groups: Vec<_> = grouped.into_values().collect();
        for captures in &mut groups {
            captures.sort_by(|a, b| (a.capture_time, &a.source).cmp(&(b.capture_time, &b.source)));
        }
        let catalog_groups = groups
            .iter()
            .enumerate()
            .map(|(id, captures)| {
                let key = captures[0].key.clone();
                let first_hz = key
                    .frequencies_hz()
                    .expect("Validated channel-estimation grid")[0];
                Group {
                    id,
                    tree_label: format!(
                        "Channel {} · {:.3} MHz",
                        key.channel_id,
                        f64::from(first_hz) / 1e6
                    ),
                    label: format!(
                        "Channel {} · {:.3} MHz · {} kHz · {} carriers",
                        key.channel_id,
                        f64::from(first_hz) / 1e6,
                        key.spacing_hz / 1000,
                        key.sample_count
                    ),
                    key,
                    capture_count: captures.len(),
                    sources: captures
                        .iter()
                        .map(|capture| Source {
                            name: capture.source.clone(),
                            capture_time: capture.capture_time,
                        })
                        .collect(),
                }
            })
            .collect();
        Ok(Self {
            catalog: Catalog {
                groups: catalog_groups,
                accepted_count: groups.iter().map(Vec::len).sum(),
                duplicate_count: scanned.duplicates,
                skipped_count: scanned.skipped,
                issues: scanned.issues,
            },
            groups,
        })
    }
    /// Calculates the complex mean and mean-derived delay for one exact group.
    /// # Arguments
    /// * `group` - Collection-local catalog ID.
    /// # Returns
    /// Full-resolution derived analysis with source provenance.
    /// # Errors
    /// Rejects unknown groups or inconsistent sample metadata.
    pub fn aggregate(&self, group: usize) -> Result<ChannelEstimateAggregate, Diagnostic> {
        let captures = self.groups.get(group).ok_or_else(|| {
            Diagnostic::new(
                "ChannelEstimationGroup",
                "Select a channel-estimation device/channel group.",
            )
        })?;
        ChannelEstimateAggregate::calculate(&captures.iter().collect::<Vec<_>>())
            .map_err(|error| Diagnostic::new("ChannelEstimationAggregate", error))
    }
    /// Computes a fixed full-channel playback axis from the averaged analysis.
    ///
    /// Includes temporal magnitude extrema and complex-mean magnitude so every
    /// valid capture remains visible; uses the existing chart padding policy.
    /// # Arguments
    /// * `group` - Exact collection-local device/channel/grid ID.
    /// # Returns
    /// Lower and upper display bounds in dB, independent of frame and zoom.
    /// # Errors
    /// Rejects unknown groups or invalid aggregate data.
    pub fn playback_magnitude_bounds(&self, group: usize) -> Result<(f64, f64), Diagnostic> {
        let reference = self.aggregate(group)?;
        let view = viewport(&reference, false, 0, reference.frequencies_hz.len())?;
        Ok((
            view["minimum"].as_f64().unwrap(),
            view["maximum"].as_f64().unwrap(),
        ))
    }
    /// Derives one chronological capture without temporal averaging.
    /// # Arguments
    /// * `group`, `frame` - Valid catalog group and ascending header-time index.
    /// # Returns
    /// Full-resolution single-capture coefficients, magnitude and group delay.
    /// # Errors
    /// Rejects unknown groups, frames or invalid coefficients.
    pub fn frame(
        &self,
        group: usize,
        frame: usize,
    ) -> Result<ChannelEstimateAggregate, Diagnostic> {
        let capture = self
            .groups
            .get(group)
            .and_then(|captures| captures.get(frame))
            .ok_or_else(|| {
                Diagnostic::new(
                    "ChannelEstimationFrame",
                    "Select a capture inside this channel group.",
                )
            })?;
        ChannelEstimateAggregate::calculate(&[capture])
            .map_err(|error| Diagnostic::new("ChannelEstimationFrame", error))
    }
}
fn decode(bytes: &[u8], name: &str) -> Result<Option<(Series, usize)>, Diagnostic> {
    let header = match PnmHeader::from_bytes(bytes) {
        Ok(header) if header.descriptor().decoder == PnmDecoderKind::ChannelEstimate => header,
        _ => return Ok(None),
    };
    let capture = CmDsOfdmChanEstimateCoef::from_header(header, ParseLimits::default())
        .map_err(|error| Diagnostic::new("ChannelEstimationImport", error))?;
    let series = Series::from_capture(&capture, name.into())
        .map_err(|error| Diagnostic::new("ChannelEstimationImport", error))?;
    let count = series.iq.len();
    Ok(Some((series, count)))
}
/// Bounded plot from the full aggregate, preserving trace extrema and missing gaps.
/// # Arguments
/// * `result` - Rust-owned full-resolution aggregate.
/// * `delay` - Plot mean-derived delay in microseconds instead of magnitude in dB.
/// * `start`, `count` - Original source-carrier range for drag zoom/reset.
/// # Returns
/// Common chart DTO and temporal statistics only for magnitude mode.
/// # Errors
/// Rejects empty or out-of-bounds ranges.
pub fn viewport(
    result: &ChannelEstimateAggregate,
    delay: bool,
    start: usize,
    count: usize,
) -> Result<Value, Diagnostic> {
    let end = start
        .checked_add(count)
        .filter(|end| *end <= result.frequencies_hz.len() && count > 0)
        .ok_or_else(|| {
            Diagnostic::new(
                "InvalidRange",
                "Select a range inside the channel-estimation group.",
            )
        })?;
    let indices: Vec<usize> = (start..end).collect();
    let value = |index: usize, field: usize| {
        if delay {
            result.group_delay_s[index].map(|s| s * 1e6)
        } else {
            match field {
                0 => result.minimum_magnitude_db[index],
                1 => result.average_magnitude_db[index],
                _ => result.maximum_magnitude_db[index],
            }
        }
    };
    let chosen = importer::joint_indices(&indices, |index, field| value(*index, field));
    let curve = |field| {
        chosen.iter().enumerate().map(|(position,local)| {
        let index=indices[*local];
        let gap=position>0 && (indices[chosen[position-1]]..=index).any(|i|value(i,field).is_none());
        json!({"sample_index":index,"frequency_hz":result.frequencies_hz[index],"value":if gap{None}else{value(index,field)}})
    }).collect::<Vec<_>>()
    };
    let valid: Vec<_> = (start..end)
        .flat_map(|index| [value(index, 0), value(index, 1), value(index, 2)])
        .flatten()
        .collect();
    let low = valid.iter().copied().min_by(f64::total_cmp).unwrap_or(-1.0);
    let high = valid.iter().copied().max_by(f64::total_cmp).unwrap_or(1.0);
    let padding = ((high - low) * 0.08).max(if delay { 0.01 } else { 0.1 });
    let mut view = json!({"points":curve(1),"group":result.group,"start":start,"count":count,"total":result.frequencies_hz.len(),"minimum":low-padding,"maximum":high+padding,"delay":delay,"unit":if delay {"µs"} else {"dB"},"average_method":"complex_iq_mean_valid_pairs","capture_count":result.sources.len(),"first_epoch":result.sources.first().map(|s|s.capture_time),"last_epoch":result.sources.last().map(|s|s.capture_time)});
    if !delay {
        view["channel_estimation_statistics"] = json!({"minimum":curve(0),"maximum":curve(2)});
    }
    Ok(view)
}
