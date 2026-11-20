# Prepared AI questions by analysis

[AI design](ai-assistance.md) · [User guide](../ui/ai-assistance.md)

Each AI-enabled analysis requires a prepared question, a version, an explicit
allowlist of evidence from existing Rust owners, and acceptance tests. The user
can edit the question; every edit or template selection invalidates approval.
Nothing is sent until a fresh preview is explicitly approved. Templates do not
add calculations, measurements or provider capabilities.

Only **OFDM Profile Performance** is enabled today. Its three version-1 questions
are owned by `apps/viewer/src/ai/presentation.rs`: profile overview, FEC/decoder
effort, and findings/recommended checks. They share the existing versioned system
prompt and strict evidence validator. Other rows below are requirements for later
work, not available AI features or authorization to upload those captures.

| Analysis | Prepared question requirement | Required Rust evidence and boundary |
| --- | --- | --- |
| OFDM Profile Performance — available | Explain each profile's FEC, BER and LDPC results; separate observations, hypotheses and next checks. | Current immutable MERLIN results, settings, findings and scopes. Never pool profile counters or identify a physical fault from these counters alone. |
| RxMER single capture | Describe the supplied MER distribution, unavailable carriers and Rust-identified regions; explain supported modulation/capacity results when supplied. | Existing RxMER statistics, selected frequency range and authoritative modulation/capacity output. No model-generated thresholds or rates. |
| RxMER Min-Avg-Max | Describe variation across compatible captures and the supplied min/average/max results. | Rust aggregation, capture count/time range and exact device/channel/grid grouping. No pooling incompatible channels or treating missing samples as zero. |
| RxMER time series | Explain supplied changes over capture time and identify observations needing follow-up. | Rust time-indexed summaries and selected group. Gaps remain explicit; do not invent temporal samples or causes. |
| Spectrum Analyzer single/range | Explain supplied spectral features, window average and integrated level for the selected range. | Rust linear-power averaging, integrated dBmV, range and capture settings. Never sum dB values or infer transmitter power. |
| Spectrum time series | Describe supplied changes within the same capture-setting group. | Rust time summaries with matching RBW/grid/settings. Do not compare incompatible acquisitions as equivalent. |
| DS Channel Estimation | Explain supplied magnitude, phase and group-delay behavior and any deterministic findings. | Existing Rust-derived channel estimates, units and valid ranges. Preserve correction/response conventions; never infer an inverse or impairment in the model. |
| Constellation Display | Explain supplied MER/EVM and hard/soft decision observations. | Rust MER/EVM, modulation and bounded existing summaries. No QAM decisions, rescaling or recalculated MER in AI. |
| Histogram | Explain supplied distribution, counts and explicit units. | Rust histogram metadata/statistics. Do not relabel histogram bins as BER, FEC or signal power without evidence. |
| FEC Summary | Explain supplied corrected/uncorrectable codewords and changes over the supplied intervals. | Parsed counter scopes, capture times and Rust deltas when available. Distinguish cumulative counters, interval deltas and simulation. |
| Modulation Profile | Explain per-profile bit loading, carrier roles and supplied capacity estimates. | Rust profile/carrier metadata and calculated throughput with its assumptions. No new LDPC efficiency or capacity arithmetic. |
| US Pre-EQ current | Explain supplied coefficient magnitude, phase, delay and flatness findings. | Existing Rust DSP results and units; keep coefficient response distinct from measured TX power. |
| US Pre-EQ last update | Explain the supplied correction's characteristics. | Rust last-update metadata and DSP summaries; do not claim it was applied or interpret it as the resulting channel response. |
| US Pre-EQ comparison | Compare original and Rust-computed corrected response and supplied delta. | Compatible validated pair, correction mode, timestamps and existing original-times-update result. Do not repeat complex multiplication or level alignment in AI. |

## Readable review and approval

The normal request review uses headings, profile labels, full metric names,
unchanged values/units, explicit scopes, findings and limitations. It is generated
from the same frozen evidence used in the outbound body. It is not a substitute
for the exact request: **Technical details** retains destination, model, digest and
complete original body. Each review/report/history pane wraps plain text, exposes
a vertical scrollbar and has no horizontal scrolling. Do not render model HTML.

## Acceptance mapping

| Requirement | Validation |
| --- | --- |
| Readable default, exact body preserved | `readable_review_and_prepared_questions_preserve_exact_approved_request`: exact large counters/scopes, no secret, dispatched bytes equal preview body. |
| Editable prepared questions, no auto-send | Rust catalog checks plus `test_prepared_question_selection_invalidates_without_sending`. |
| Actual vertical scrolling, both themes | `test_review_is_readable_and_long_content_scrolls_in_both_themes`: bounded width, overflow and scroll position. |
| Safe rendered output and approval lifecycle | Existing QML plain-text, question-edit and explicit-approval tests; Rust stale/replayed/context-change tests. |
| Each future analysis stays disabled until ready | Require a versioned template, allowlisted evidence adapter, numerical-owner review, disclosure/approval integration and domain-specific evidence/security tests before adding an entry point. |

Validation on 2026-11-18: 228 all-feature workspace Rust tests, 178 QML tests,
six Qt adapter tests, formatting, workspace/Qt Clippy and native Ubuntu build
pass. Fixture-backed offscreen native preview reports `AI_PREVIEW_OK`; dark and
light request-review screenshots were visually checked and copied to the user
guide. This validates local UI/evidence/approval behavior, not a live AI answer.
No provider call, credential change, installation or release was performed.

## Local engineering-report availability

Every current PNM measurement and left-gutter analysis can now open a local report
through the shared Reports action. Local report snapshots include prepared question
text, but do not automatically enable/send provider requests. Existing OFDM AI
responses use the same renderer after validation. See
[report coverage](engineering-reports.md#analysis-coverage-and-lifecycles).
New analyses must apply `skills/pnm-analysis-assistance/SKILL.md`.

## Multi-capture channel estimation — prepared question 1.1

Owner: `channel-estimation-collection`, derived local analysis. Review compatible
MAC/channel/grid capture counts, complex-mean magnitude and group delay; preserve
units (dB, µs display, seconds export), excluded-carrier gaps and chronology.
Explain that delay comes from mean I/Q and is not averaged delay, and that the
mean's magnitude can fall below the temporal minimum. Do not infer PLC positions
or physical fault causes. The Reports action supplies the prepared question and
existing local evidence; no new live provider route is enabled.

Validation: core independent mean/delay tests, viewer import/export/zoom tests,
QML metric/trace controls, and native `--channel-estimation-smoke` report creation.

Channel-estimation collection question **1.1** also distinguishes single-capture
playback (`average_method: single_capture`) from temporal aggregation. Frame
index/count and capture UTC derive from the current collection, never filenames.
