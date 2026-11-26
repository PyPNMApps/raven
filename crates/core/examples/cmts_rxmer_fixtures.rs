/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Deterministic synthetic Table410 files. Literal offsets are independent of decoder constants.
use serde_json::json;
use std::{fs, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let requested = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Supply a new output directory under local/")?,
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let parent = requested
        .parent()
        .ok_or("Supply an output parent")?
        .canonicalize()?;
    if !parent.starts_with(root.join("local")) {
        return Err("Fixture output must stay under this checkout's local/ directory".into());
    }
    let output = parent.join(requested.file_name().ok_or("Supply a directory name")?);
    fs::create_dir(&output)?;
    let scenarios: Vec<(&str, Vec<u8>)> = vec![
        ("golden", vec![0, 1, 95, 254, 255]),
        ("uniform", vec![160; 128]),
        ("uniform-50khz", vec![160; 128]),
        ("mer-36db-sd-0p75db", nominal_mer_profile()),
        (
            "notch",
            (0..128)
                .map(|index| if (48..64).contains(&index) { 48 } else { 160 })
                .collect(),
        ),
        ("slope", (0..128).map(|index| 200 - index).collect()),
        (
            "sparse-unavailable",
            (0..128)
                .map(|index| if index % 13 == 0 { 255 } else { 160 })
                .collect(),
        ),
        ("all-unavailable", vec![255; 128]),
        ("empty", vec![]),
    ];
    for (name, values) in scenarios {
        // Independently authored Table410 offsets, reviewed against the printed table.
        let mut bytes = vec![0; 297];
        bytes[..10].copy_from_slice(&[0x50, 0x4e, 0x4e, 0x69, 2, 0, 0x65, 0x53, 0xf1, 0x00]);
        bytes[10..14].copy_from_slice(&0x01020304u32.to_be_bytes());
        bytes[14..14 + 17].copy_from_slice(b"ccap.example.test");
        bytes[277..283].copy_from_slice(&[0, 0xca, 0, 0, 0, 1]);
        bytes[283..286].copy_from_slice(&[0, 25, 1]);
        bytes[286..290].copy_from_slice(&20_000_000u32.to_be_bytes());
        let spacing = if name == "uniform-50khz" { 50 } else { 25 };
        bytes[290..293].copy_from_slice(&[0, 16, spacing]);
        bytes[293..297].copy_from_slice(&(values.len() as u32).to_be_bytes());
        bytes.extend(&values);
        fs::write(output.join(format!("{name}.bin")), &bytes)?;
        let mut manifest = json!({"synthetic":true,"source":"CM-SP-CCAP-OSSIv3.1-I18-200610 Table410; 7.3.2.5", "file_type":"0x504E4E69","major":2,"minor":0,"expected_result":"success","capture_time":1700000000u32,"if_index":16909060,"ccap_id":"ccap.example.test","first_index":16,"zero_hz":20000000,"spacing_khz":spacing,"raw_values":values,"expected_values_db":values.iter().map(|value|if *value==255 {None} else {Some(f64::from(*value)/4.0)}).collect::<Vec<_>>()});
        if name == "mer-36db-sd-0p75db" {
            manifest["distribution"] = json!("deterministically shuffled symmetric normal-like histogram; not a random field capture");
            manifest["expected_statistics"] = json!({"sample_count":1024,"arithmetic_mean_db":36.0,"population_standard_deviation_db":0.75,"minimum_db":33.5,"maximum_db":38.5});
        }
        fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
    }
    let original = fs::read(output.join("golden.bin"))?;
    for (name, offset, value, length, result) in [
        (
            "legacy-unsupported",
            2,
            0x4d,
            original.len(),
            "UnsupportedLayout",
        ),
        ("unknown-type", 3, 0x64, original.len(), "UnsupportedType"),
        (
            "unsupported-major",
            4,
            9,
            original.len(),
            "UnsupportedVersion",
        ),
        (
            "unsupported-spacing",
            292,
            0,
            original.len(),
            "InvalidField",
        ),
        ("truncated-header", 0, 0x50, 100, "Truncated"),
        ("truncated-payload", 0, 0x50, 300, "Truncated"),
        (
            "contradictory-length",
            296,
            4,
            original.len(),
            "LengthMismatch",
        ),
        (
            "excessive-length",
            293,
            255,
            original.len(),
            "LimitExceeded",
        ),
    ] {
        let mut bytes = original.clone();
        bytes[offset] = value;
        bytes.truncate(length);
        fs::write(output.join(format!("{name}.bin")), &bytes)?;
        fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_vec_pretty(
                &json!({"synthetic":true,"source":"CM-SP-CCAP-OSSIv3.1-I18-200610 Table410; 7.3.2.5", "baseline":"golden.bin", "fixture_kind":"malformed derivative; not a valid alternate layout", "file_type":format!("0x{:02X}{:02X}{:02X}{:02X}",bytes[0],bytes[1],bytes[2],bytes[3]), "major": if bytes[2] == b'N' {Some(bytes[4])} else {None}, "minor": if bytes[2] == b'N' {Some(bytes[5])} else {None}, "expected_result":result,"mutation":{"offset":offset,"value":value,"truncated_length":length}}),
            )?,
        )?;
    }
    fs::write(output.join("README.md"),"# Synthetic CMTS upstream RxMER\n\nGenerated from CCAP-OSSI 3.1 I18 Table410. No vendor captures.\nEvery binary has a manifest with expected results and mutations.\nSee docs/reference/cmts-upstream-rxmer/format.md.\n")?;
    Ok(())
}

/// Builds a quarter-dB histogram with mean 36 dB and population SD 0.75 dB.
/// Counts are symmetric: 1,024 samples and squared raw-offset sum 9,216.
fn nominal_mer_profile() -> Vec<u8> {
    const CENTER_RAW: u8 = 144;
    const COUNTS_BY_ABSOLUTE_OFFSET: [usize; 11] = [134, 129, 109, 83, 58, 33, 18, 9, 3, 2, 1];
    let mut values = Vec::with_capacity(1024);
    for (offset, count) in COUNTS_BY_ABSOLUTE_OFFSET.into_iter().enumerate() {
        for _ in 0..count {
            values.push(CENTER_RAW - offset as u8);
            if offset != 0 {
                values.push(CENTER_RAW + offset as u8);
            }
        }
    }

    // Fixed-seed ordering is only for repeatable visual examples, not physical noise.
    let mut shuffle_state = 0x36_00_07_50u32;
    for index in (1..values.len()).rev() {
        shuffle_state ^= shuffle_state << 13;
        shuffle_state ^= shuffle_state >> 17;
        shuffle_state ^= shuffle_state << 5;
        values.swap(index, shuffle_state as usize % (index + 1));
    }
    values
}
