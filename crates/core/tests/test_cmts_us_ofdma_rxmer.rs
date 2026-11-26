/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Independent Table410 golden octets, not a round-trip encoder oracle.
use pnm::{
    diagnostic::ErrorCode,
    pnm::{
        analysis::cmts_upstream_rxmer::statistics,
        parser::{
            cmts_us_ofdma_rxmer::CmtsUsOfdmaRxMer,
            parse_limits::ParseLimits,
            pnm_file::{DecodedPnm, PnmFile},
            pnm_header::HasPnmHeader,
        },
    },
};
fn golden() -> Vec<u8> {
    let mut bytes = vec![0; 302];
    bytes[..14].copy_from_slice(&[80, 78, 78, 105, 2, 7, 101, 83, 241, 0, 1, 2, 3, 4]);
    bytes[14..19].copy_from_slice(b"ccap\0");
    bytes[270..277].copy_from_slice(&[0, 0xca, 0, 0, 0, 2, 3]);
    bytes[277..286].copy_from_slice(&[0, 0xca, 0, 0, 0, 1, 0, 25, 1]);
    // 20,000,000 Hz literal BE; first16;25kHz; five payload octets.
    bytes[286..].copy_from_slice(&[1, 49, 45, 0, 0, 16, 25, 0, 0, 0, 5, 0, 1, 95, 254, 255]);
    bytes
}
#[test]
fn independent_golden_fields_values_and_json() {
    let bytes = golden();
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default()).unwrap();
    assert_eq!(capture.capture_time(), Some(1_700_000_000));
    assert_eq!(capture.format_version().major, 2);
    assert_eq!(capture.format_version().minor, 7);
    assert_eq!(capture.ccap_id(), "ccap");
    assert_eq!(capture.if_index(), 16_909_060);
    assert_eq!(capture.raw_values().as_ptr(), bytes[297..].as_ptr());
    assert_eq!(
        capture.values_db().collect::<Vec<_>>(),
        vec![Some(0.0), Some(0.25), Some(23.75), Some(63.5), None]
    );
    assert_eq!(
        capture.frequencies_hz().collect::<Vec<_>>(),
        vec![20_400_000, 20_425_000, 20_450_000, 20_475_000, 20_500_000]
    );
    let samples = capture.samples().collect::<Vec<_>>();
    assert_eq!(samples[4].subcarrier_index, 20);
    assert!(!samples[4].valid);
    let json: serde_json::Value = serde_json::from_str(&capture.to_json(10000).unwrap()).unwrap();
    assert_eq!(json["device_role"], "CMTS/CCAP");
    assert_eq!(json["samples"][4]["rxmer_db"], serde_json::Value::Null);
    assert_eq!(json["rpd_port"], 3);
    assert_eq!(json["pre_eq_raw"], 1);
    let summary = statistics(&capture);
    assert_eq!(summary.valid_count, 4);
    assert_eq!(summary.arithmetic_mean_db, Some(21.875));
    assert!(matches!(
        PnmFile::from_bytes(&bytes)
            .decode(ParseLimits::default())
            .unwrap(),
        DecodedPnm::CmtsUpstreamRxMer(_)
    ));
    assert!(capture.to_json(10).is_err());
}
#[test]
fn every_truncation_and_declared_length_is_bounded() {
    let bytes = golden();
    for end in 0..bytes.len() {
        assert!(
            CmtsUsOfdmaRxMer::parse(&bytes[..end], ParseLimits::default()).is_err(),
            "{end}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&trailing, ParseLimits::default())
            .unwrap_err()
            .code,
        ErrorCode::LengthMismatch
    );
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits { max_samples: 4 })
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    for value in [65537, u32::MAX] {
        let mut candidate = bytes.clone();
        candidate[293..297].copy_from_slice(&value.to_be_bytes());
        let error = CmtsUsOfdmaRxMer::parse(&candidate, ParseLimits::default()).unwrap_err();
        assert_eq!(error.offset, 293);
        assert_eq!(error.code, ErrorCode::LimitExceeded);
    }
}
#[test]
fn unsupported_layouts_types_versions_and_encodings_do_not_fall_back() {
    for (offset, value, code) in [
        (2, b'M', ErrorCode::UnsupportedLayout),
        (3, 0x64, ErrorCode::UnsupportedType),
        (3, 4, ErrorCode::UnsupportedVersion),
        (4, 1, ErrorCode::UnsupportedVersion),
        (4, 3, ErrorCode::UnsupportedVersion),
        (292, 0, ErrorCode::InvalidField),
    ] {
        let mut bytes = golden();
        bytes[offset] = value;
        let file = PnmFile::from_bytes(&bytes);
        assert_eq!(file.bytes(), bytes);
        assert_eq!(file.decode(ParseLimits::default()).unwrap_err().code, code);
    }
    let mut bytes = golden();
    bytes[14..270].fill(b'x');
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())
            .unwrap_err()
            .field,
        "ccap_id_terminator"
    );
    bytes[14] = 0xff;
    bytes[15] = 0;
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())
            .unwrap_err()
            .code,
        ErrorCode::InvalidField
    );
}
#[test]
fn null_empty_frequency_and_index_boundaries() {
    let mut bytes = golden();
    bytes[297..].fill(255);
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default()).unwrap();
    assert_eq!(statistics(&capture).arithmetic_mean_db, None);
    assert_eq!(statistics(&capture).unavailable_count, 5);
    bytes.truncate(297);
    bytes[293..297].fill(0);
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits { max_samples: 0 })
            .unwrap()
            .samples()
            .count(),
        0
    );
    bytes = golden();
    bytes[286..290].fill(255);
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())
            .unwrap_err()
            .code,
        ErrorCode::ArithmeticOverflow
    );
    bytes = golden();
    bytes[290..292].fill(255);
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())
            .unwrap_err()
            .offset,
        290
    );
    bytes = golden();
    bytes[292] = 50;
    assert_eq!(
        CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())
            .unwrap()
            .frequencies_hz()
            .next(),
        Some(20_800_000)
    );
}

#[test]
fn every_shipped_fixture_matches_its_companion_manifest() {
    let directory = std::path::Path::new(pnm_fixtures::CMTS_RXMER_FIXTURE_DIRECTORY);
    let mut checked = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("bin") {
            continue;
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.with_extension("json")).unwrap()).unwrap();
        assert_eq!(manifest["synthetic"], true);
        let bytes = std::fs::read(&path).unwrap();
        let result = CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default());
        if manifest["expected_result"] == "success" {
            let capture = result.unwrap();
            assert_eq!(
                serde_json::to_value(capture.raw_values()).unwrap(),
                manifest["raw_values"],
                "{}",
                path.display()
            );
            assert_eq!(
                serde_json::to_value(capture.values_db().collect::<Vec<_>>()).unwrap(),
                manifest["expected_values_db"]
            );
            assert_eq!(capture.capture_time(), Some(1_700_000_000));
        } else {
            assert_eq!(
                format!("{:?}", result.unwrap_err().code),
                manifest["expected_result"].as_str().unwrap(),
                "{}",
                path.display()
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 9);
}

#[test]
fn complete_octet_domain_and_last_representable_carrier() {
    let mut bytes = golden();
    bytes.truncate(297);
    bytes[293..297].copy_from_slice(&256u32.to_be_bytes());
    bytes.extend(0..=255);
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, Default::default()).unwrap();
    for (raw, sample) in capture.samples().enumerate() {
        assert_eq!(usize::from(sample.raw), raw);
        assert_eq!(
            sample.rxmer_db,
            if raw == 255 {
                None
            } else {
                Some(raw as f64 / 4.0)
            }
        );
    }
    bytes = golden();
    bytes.truncate(298);
    bytes[286..290].fill(0);
    bytes[290..292].fill(255);
    bytes[293..297].copy_from_slice(&1u32.to_be_bytes());
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, Default::default()).unwrap();
    assert_eq!(capture.samples().next().unwrap().subcarrier_index, 65535);
    assert_eq!(capture.frequencies_hz().next(), Some(1_638_375_000));
}

#[test]
fn nominal_binary_has_requested_mean_and_population_standard_deviation() {
    let path = std::path::Path::new(pnm_fixtures::CMTS_RXMER_FIXTURE_DIRECTORY)
        .join("mer-36db-sd-0p75db.bin");
    let bytes = std::fs::read(path).unwrap();
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, Default::default()).unwrap();
    let samples: Vec<_> = capture.values_db().collect::<Option<Vec<_>>>().unwrap();
    assert_eq!(samples.len(), 1024);
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    assert_eq!(mean, 36.0);
    assert_eq!(variance.sqrt(), 0.75);
    assert_eq!(statistics(&capture).arithmetic_mean_db, Some(36.0));
    assert_eq!(statistics(&capture).minimum_db, Some(33.5));
    assert_eq!(statistics(&capture).maximum_db, Some(38.5));
    assert_eq!(capture.frequencies_hz().next(), Some(20_400_000));
    assert_eq!(capture.frequencies_hz().last(), Some(45_975_000));
}
