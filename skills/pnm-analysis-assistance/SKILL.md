---
name: pnm-analysis-assistance
description: Add or extend a RAVEN PNM analysis or left-gutter test with a Rust-owned engineering report, prepared AI question, evidence contract and validation. Use for new analysis features, not generic UI edits.
---

# PNM analysis reporting and AI assistance

Every new RAVEN PNM analysis/test needs a local engineering-report entry point and
an explicitly defined AI-assistance path. Reuse `viewer::reports` and the shared
native renderer; do not create an independent report engine for each analysis.
Use the current Reports action for single captures and left-gutter workspaces.

Before implementation, identify its existing Rust numerical owner, capture/run
identity and replacement/cancellation lifecycle. Document the associated design,
prepared question and version, permitted evidence/units/scopes, missing-data policy,
classification (measured/simulation/derived/interpretation), and tests. Extend
`docs/development/ai-prepared-prompts.md` and the report coverage matrix.

Reports consume local registered datasets and evidence IDs. Never accept model
values as measurements, perform DSP in QML, pool incompatible profiles/grids, or
load model-authored QML, code, paths or URLs. Keep Markdown, chart, table, image,
evidence and section behavior in the finite renderer registry. New block types
need a schema/version decision, bounded Rust validation and adversarial tests.

Prepared questions are editable, but selection/edit/reset invalidates approval.
Local reports work offline without credentials. Do not automatically send data or
claim live AI support where no approved evidence adapter/provider route exists.
Initially live analysis remains the existing OFDM feature; other analyses have
local reports and prepared-question requirements until explicitly implemented.
Keep the exact-payload, single-use approval and session-only credential policies.

Test all supported analysis modes, missing/stale sources, report replacement,
source provenance, themes, actual scrolling, unauthorized references/actions and
native resource registration. Preserve normal measurement exports and package
size limits. Update user guides/screenshots and mark unverified platforms honestly.
