/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: Apache-2.0
 */
//! MAC-sanitized PyPNM captures for tests and runnable examples only.
//! Binary inputs retain their upstream license and provenance in pypnm/.
#![forbid(unsafe_code)]

// Anchor the single fixture directory to this package, not a caller's source
// location or process working directory. Each filename is declared once below.
macro_rules! capture {
    ($filename:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/pypnm/tests/files/",
            $filename
        ))
    };
}

/// MAC-sanitized `rxmer.bin` capture; see pypnm/provenance.json.
pub const RXMER: &[u8] = capture!("rxmer.bin");

/// MAC-sanitized `fec_summary.bin` capture; see pypnm/provenance.json.
pub const FEC_SUMMARY: &[u8] = capture!("fec_summary.bin");

/// MAC-sanitized `channel_estimation.bin` capture; see pypnm/provenance.json.
pub const CHANNEL_ESTIMATE: &[u8] = capture!("channel_estimation.bin");

/// MAC-sanitized `const_display.bin` capture; see pypnm/provenance.json.
pub const CONSTELLATION: &[u8] = capture!("const_display.bin");

/// MAC-sanitized `histogram.bin` capture; see pypnm/provenance.json.
pub const HISTOGRAM: &[u8] = capture!("histogram.bin");

/// MAC-sanitized `us_pre_equalizer_coef.bin` capture; see pypnm/provenance.json.
pub const UPSTREAM_PRE_EQ: &[u8] = capture!("us_pre_equalizer_coef.bin");

/// MAC-sanitized `us_pre_equalizer_coef_last.bin` capture; see pypnm/provenance.json.
pub const UPSTREAM_PRE_EQ_LAST: &[u8] = capture!("us_pre_equalizer_coef_last.bin");

/// MAC-sanitized `spectrum_analyzer.bin` capture; see pypnm/provenance.json.
pub const SPECTRUM: &[u8] = capture!("spectrum_analyzer.bin");

/// MAC-sanitized `modulation_profile.bin` capture; see pypnm/provenance.json.
pub const MODULATION_PROFILE: &[u8] = capture!("modulation_profile.bin");

// One owner for the MERLIN demonstration fixture directory and filenames.
macro_rules! merlin_capture {
    ($filename:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/merlin/cl-192mhz-8mhz-notch/",
            $filename
        ))
    };
}
/// Synthetic 192 MHz, 4096-QAM Profile 3; retain adjacent MERLIN/PyPNM notices.
pub const MERLIN_CL_PROFILE: &[u8] = merlin_capture!("modulation_profile.json");
/// Synthetic 42 dB RxMER with an 8 MHz, 10 dB notch away from PLC.
pub const MERLIN_CL_RXMER_NOTCH: &[u8] = merlin_capture!("rxmer-notch-10db.json");
/// Synthetic flat 42 dB comparison on the same grid.
pub const MERLIN_CL_RXMER_FLAT: &[u8] = merlin_capture!("rxmer-flat-42db.json");
/// Frequency-adapted complex channel shape; not a field capture at this grid.
pub const MERLIN_CL_CHANNEL: &[u8] = merlin_capture!("channel-estimation-adapted.json");
