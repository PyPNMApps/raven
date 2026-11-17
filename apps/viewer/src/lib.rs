/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Native desktop services organized by responsibility.
pub mod ai;
pub mod capture;
pub mod identity;
pub mod measurements;
pub mod resources;
#[cfg(feature = "merlin-fec")]
pub mod simulation;
pub mod support;
pub mod updates;

// Preserve the established capture type convenience imports for library callers.
pub use capture::{
    Capture, Diagnostic, Metadata, Point, Viewport, MAX_INPUT_BYTES, MAX_VIEW_SAMPLES,
};
