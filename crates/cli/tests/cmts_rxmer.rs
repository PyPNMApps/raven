/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Exercise the actual CLI with content detection and canonical upstream JSON.
use std::{path::Path, process::Command};
#[test]
fn cli_matches_core_and_preserves_structured_failure_details() {
    let directory = Path::new(pnm_fixtures::CMTS_RXMER_FIXTURE_DIRECTORY);
    let output = Command::new(env!("CARGO_BIN_EXE_pnm-decode"))
        .arg(directory.join("golden.bin"))
        .arg("--stdout")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(directory.join("golden.bin")).unwrap();
    let expected =
        pnm::pnm::parser::cmts_us_ofdma_rxmer::CmtsUsOfdmaRxMer::parse(&bytes, Default::default())
            .unwrap()
            .to_json(32768)
            .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::from_str::<serde_json::Value>(&expected).unwrap()
    );
    let rejected = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(format!("cmts-cli-unsupported-{}.bin", std::process::id()));
    std::fs::create_dir_all(rejected.parent().unwrap()).unwrap();
    let mut invalid = bytes.clone();
    invalid[4] = 9;
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&rejected)
            .unwrap();
        file.write_all(&invalid).unwrap();
    }
    let failed = Command::new(env!("CARGO_BIN_EXE_pnm-decode"))
        .arg(&rejected)
        .arg("--stdout")
        .output()
        .unwrap();
    std::fs::remove_file(rejected).unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("UnsupportedVersion"));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("byte 4"));
}
