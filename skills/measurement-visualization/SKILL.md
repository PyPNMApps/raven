---
name: measurement-visualization
description: Add or change RAVEN measurement charts and collection views with consistent units, zoom, trace controls, missing-data handling, examples and validation.
---

# Measurement visualization baseline

Apply when adding or changing a measurement display or collection analysis.
Inspect the existing numerical owner and shared charts before adding a renderer.
Rust owns decoding, grouping, averaging, phase operations, units, export and
source-range selection; QML presents bounded values and transient control state.

For every display define its source, quantity, axis units, averaging/statistic
meaning and missing-data behavior. Frequency plots use MHz on X and a vertical
Y title with quantity/unit in the same font as X. Do not silently label average
complex coefficients as average dB or average delay.

Use MeasurementChart for compatible line plots. Provide drag-to-zoom, double-click
reset and a visible Reset zoom action in new collection workspaces. Zoom indexes
must reference full source carriers, not decimated points. Retain endpoints,
trace extrema and gaps; never draw across unavailable spans. Preserve zoom when
switching metrics on the same grid; reset when selecting another group.

Provide distinct trace colors and matching outlined toggles for multiple curves.
Use the shared Theme line width/colors and controls; check light, dark and System
appearance, including popups, errors and disabled state. Do not add invented PLC
bands or carrier labels without a trusted carrier-type source. Do not append
unrequested explanatory footnotes to the chart; put scientific details in docs.

Collections use binary metadata for MAC/channel/grid grouping and epoch ordering,
never filenames. Reuse the bounded importer, native pickers, shared device tree,
async generation guards and error popups. Preserve a usable prior session after
failed replacement; cancel superseded work and reject stale callbacks.

Each new analysis needs the existing Reports entry point and a versioned prepared
question through [analysis assistance](../pnm-analysis-assistance/SKILL.md).
Keep local reports offline; an entry point does not authorize new live AI routes.
Full JSON export preserves all values, counts, units and source provenance.

## Defaults by display type

- Frequency lines: explicit quantity/unit, MHz X axis, source-based zoom/reset,
  missing gaps, and distinct show/mute controls for overlaid traces.
- I/Q dots/density: equal I/Q bounds and aspect, Rust-owned point sampling and
  density counts, and clear Dots/Density choices. Red means denser and blue means
  sparser. A coefficient scatter is not a received-symbol constellation or power
  measurement. Preserve full-capture fitting unless spatial zoom is requested.
- Histograms: label the actual bins/categories and count units; binning and
  statistics stay in Rust. Do not add frequency controls to a non-frequency axis.
- Time series/playback: order decoded epochs in Rust, identify the active capture
  in UTC, and pause when hidden/blocked or opening a report. Do not infer time
  from filenames or let a late frame replace a new source.
- Tables/reports: label units/scopes and unavailable values, wrap readable text,
  provide scrolling when needed and retain the original precision for export.
  Use the existing report renderer, not an analysis-specific HTML surface.

These defaults guide new work; they do not authorize a blanket redesign of
existing measurement views or extra scientific calculations.

## Acceptance

- Test numerical semantics independently, including gaps, units and invalid grids.
- Test actual rendered metric controls, trace visibility and mouse zoom/reset.
- Run Qt adapter/QML/native build checks; do not infer native OS validation.
- Add sanitized binary examples and a short walkthrough; label synthetic data.
- Add current native dark/light screenshots and link the feature from README.
- Check expanded demo hygiene before creating release ZIPs; register manifests.

Use [coding style](../coding-style/SKILL.md) and
[PNM testing](../pnm-testing/SKILL.md) for implementation conventions.
