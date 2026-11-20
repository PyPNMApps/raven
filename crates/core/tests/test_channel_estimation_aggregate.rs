/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
use pnm::pnm::analysis::channel_estimation::{ChannelEstimateAggregate, GroupKey, Series};
use std::f64::consts::{PI, TAU};
fn snapshot(iq: Vec<Option<[f64; 2]>>, epoch: u32) -> Series {
    Series {
        key: GroupKey {
            mac_address: [0, 202, 0, 0, 0, 1],
            channel_id: 34,
            major_version: 1,
            minor_version: 0,
            zero_frequency_hz: 100_000_000,
            first_active_subcarrier_index: 10,
            spacing_hz: 25_000,
            sample_count: iq.len(),
        },
        source: format!("capture-{epoch}"),
        capture_time: epoch,
        iq,
    }
}
#[test]
fn averages_complex_coefficients_before_phase_and_uses_pypnm_endpoint_slopes() {
    let first = snapshot(vec![Some([1.0, 0.0]); 3], 10);
    let second = snapshot(
        vec![Some([1.0, 0.0]), Some([0.0, 3.0]), Some([-3.0, 0.0])],
        20,
    );
    let result = ChannelEstimateAggregate::calculate(&[&second, &first]).unwrap();
    assert_eq!(
        result.average_iq,
        vec![Some([1.0, 0.0]), Some([0.5, 1.5]), Some([-1.0, 0.0])]
    );
    let expected = [
        -3.0_f64.atan() / (TAU * 25000.0),
        -1.0 / (4.0 * 25000.0),
        -(PI - 3.0_f64.atan()) / (TAU * 25000.0),
    ];
    for (actual, expected) in result.group_delay_s.iter().zip(expected) {
        assert!((actual.unwrap() - expected).abs() < 1e-14);
    }
    assert!((result.average_magnitude_db[1].unwrap() - 3.979400086720376).abs() < 1e-12);
    assert_eq!(result.minimum_magnitude_db[1], Some(0.0));
    assert!((result.maximum_magnitude_db[1].unwrap() - 9.542425094393248).abs() < 1e-12);
    assert!((result.group_delay_s[0].unwrap() - (-1.0 / (8.0 * 25000.0))).abs() > 1e-6);
    assert_eq!(result.frequencies_hz, vec![100250000, 100275000, 100300000]);
    assert_eq!(
        result
            .sources
            .iter()
            .map(|s| s.capture_time)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
}
#[test]
fn missing_values_are_not_zero_and_complex_cancellation_has_undefined_phase() {
    let a = snapshot(vec![None, Some([1.0, 0.0]), Some([0.0, 0.0]), None], 10);
    let b = snapshot(
        vec![None, Some([-1.0, 0.0]), Some([2.0, 0.0]), Some([1.0, 0.0])],
        20,
    );
    let result = ChannelEstimateAggregate::calculate(&[&a, &b]).unwrap();
    assert_eq!(result.valid_counts, vec![0, 2, 2, 1]);
    assert_eq!(
        result.average_iq,
        vec![None, Some([0.0, 0.0]), Some([1.0, 0.0]), Some([1.0, 0.0])]
    );
    assert_eq!(result.average_magnitude_db[1], None);
    assert_eq!(
        result.group_delay_s,
        vec![None, None, Some(-0.0), Some(-0.0)]
    );
}
#[test]
fn refuses_incompatible_grids_lengths_nonfinite_values_and_overflow() {
    let first = snapshot(vec![Some([1.0, 0.0]); 3], 10);
    for field in 0..8 {
        let mut second = first.clone();
        match field {
            0 => second.key.channel_id += 1,
            1 => second.key.mac_address[5] += 1,
            2 => second.key.major_version += 1,
            3 => second.key.zero_frequency_hz += 1,
            4 => second.key.first_active_subcarrier_index += 1,
            5 => second.key.spacing_hz *= 2,
            6 => second.key.sample_count += 1,
            _ => {
                second.iq.pop();
            }
        }
        assert!(ChannelEstimateAggregate::calculate(&[&first, &second]).is_err());
    }
    let mut bad = first.clone();
    bad.iq[0] = Some([f64::NAN, 0.0]);
    assert!(ChannelEstimateAggregate::calculate(&[&bad]).is_err());
    bad = first.clone();
    bad.key.zero_frequency_hz = u32::MAX;
    assert!(bad.key.frequencies_hz().is_err());
    assert!(ChannelEstimateAggregate::calculate(&[]).is_err());
}
#[test]
fn pure_delays_unwrap_across_turns_and_retain_the_delay_sign() {
    for delay in [-1e-6, 1e-6] {
        let first = snapshot(
            (0..80)
                .map(|index| {
                    let phase = -TAU * delay * (index as f64 * 25000.0);
                    Some([phase.cos(), phase.sin()])
                })
                .collect(),
            10,
        );
        let result = ChannelEstimateAggregate::calculate(&[&first]).unwrap();
        for actual in result.group_delay_s {
            assert!((actual.unwrap() - delay).abs() < 1e-13);
        }
    }
}
