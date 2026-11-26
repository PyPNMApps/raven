/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! Bounded Rust API example: cargo run -p pnm --example cmts_rxmer -- FILE.
use pnm::pnm::{
    analysis::cmts_upstream_rxmer::statistics,
    parser::{
        cmts_us_ofdma_rxmer::{CmtsUsOfdmaRxMer, HEADER_BYTES},
        parse_limits::ParseLimits,
        pnm_header::HasPnmHeader,
    },
};
use std::{fs::File, io::Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Supply one CMTS RxMER file")?;
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("Select a regular capture file".into());
    }
    let limits = ParseLimits::default();
    let mut bytes = Vec::new();
    file.take((HEADER_BYTES + limits.max_samples + 1) as u64)
        .read_to_end(&mut bytes)?;
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, limits)?;
    eprintln!("Header: {:?}", capture.pnm_header().format_version());
    eprintln!("Metadata: {:?}", capture.metadata());
    eprintln!("Statistics: {:?}", statistics(&capture));
    for sample in capture.samples().take(5) {
        eprintln!(
            "{} {} {:?}",
            sample.subcarrier_index, sample.frequency_hz, sample.rxmer_db
        );
    }
    println!("{}", capture.to_json(32 * 1024 * 1024)?);
    Ok(())
}
