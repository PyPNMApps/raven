/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Descriptive upstream RxMER statistics; arithmetic dB mean, not power averaging.
use crate::pnm::parser::cmts_us_ofdma_rxmer::CmtsUsOfdmaRxMer;
use serde::Serialize;

/// Summary over available carriers only; all-unavailable captures retain nulls.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RxMerStatistics {
    pub sample_count: usize,
    pub valid_count: usize,
    pub unavailable_count: usize,
    pub minimum_db: Option<f64>,
    pub arithmetic_mean_db: Option<f64>,
    pub maximum_db: Option<f64>,
}
/// Summarizes existing decoded dB values without interpolating missing carriers.
/// # Arguments
/// * `capture` - Immutable validated upstream observation.
/// # Returns
/// Counts and descriptive dB extrema/mean, absent for zero valid observations.
pub fn statistics(capture: &CmtsUsOfdmaRxMer<'_>) -> RxMerStatistics {
    let mut summary = RxMerStatistics {
        sample_count: capture.raw_values().len(),
        valid_count: 0,
        unavailable_count: 0,
        minimum_db: None,
        arithmetic_mean_db: None,
        maximum_db: None,
    };
    let mut sum = 0.0;
    for value in capture.values_db().flatten() {
        summary.valid_count += 1;
        sum += value;
        summary.minimum_db = Some(
            summary
                .minimum_db
                .map_or(value, |minimum| minimum.min(value)),
        );
        summary.maximum_db = Some(
            summary
                .maximum_db
                .map_or(value, |maximum| maximum.max(value)),
        );
    }
    summary.unavailable_count = summary.sample_count - summary.valid_count;
    if summary.valid_count > 0 {
        summary.arithmetic_mean_db = Some(sum / summary.valid_count as f64);
    }
    summary
}
