/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Run the licensed MERLIN CMCE notch example through RAVEN's Rust adapter.
use viewer::simulation::fec::{engine, FecExperiment, FecSettings};

/// Loads the original JSON demonstration, executes MERLIN and prints its report.
/// # Returns
/// Unit after printing one owned report; no capture or workbench is modified.
/// # Errors
/// Reports invalid arguments, input/preflight errors and incomplete execution.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let notch = match arguments.as_slice() {
        [] => true,
        [mode] if mode == "--notch" => true,
        [mode] if mode == "--flat" => false,
        _ => return Err("Usage: fec_notch [--notch|--flat]".into()),
    };
    let profile = engine::NamedCapture::new(
        "modulation_profile.json",
        pnm_fixtures::MERLIN_CL_PROFILE.to_vec(),
    )?;
    let (name, bytes) = if notch {
        ("rxmer-notch-10db.json", pnm_fixtures::MERLIN_CL_RXMER_NOTCH)
    } else {
        ("rxmer-flat-42db.json", pnm_fixtures::MERLIN_CL_RXMER_FLAT)
    };
    let rxmer = engine::NamedCapture::new(name, bytes.to_vec())?;
    let channel = engine::NamedCapture::new(
        "channel-estimation-adapted.json",
        pnm_fixtures::MERLIN_CL_CHANNEL.to_vec(),
    )?;
    let mut options = engine::EvaluationOptions::default()
        .with_receiver_mode(engine::ReceiverMode::CmChannelEstimated);
    options.observation = engine::ObservationDuration::new(10)?;
    options.limits.runtime = engine::RuntimeLimit::new(300_000)?;
    let settings = FecSettings {
        profile_id: 3,
        protocol: engine::ProtocolScope::Docsis31,
        ocd_overrides: engine::OcdOverrides {
            bandwidth: Some(engine::BandwidthHz::new(192_000_000)?),
            cyclic_prefix: Some(engine::CyclicPrefixSamples::new(512)?),
            roll_off: Some(engine::RollOffSamples::new(128)?),
            interleaver_depth: Some(engine::InterleaverDepth::new(16)?),
        },
        options,
    };
    let mut experiment = FecExperiment::from_json(&profile, &rxmer, &channel, settings)?;
    let report = experiment.run(&engine::RunControl::new())?;
    println!("{}", report.to_json()?);
    if report.status() != engine::BatchStatus::Complete {
        return Err("Experiment incomplete; inspect the retained report".into());
    }
    Ok(())
}
