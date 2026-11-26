# CMTS upstream RxMER validation

[Wire evidence](format.md) · [Rust API](api.md) · [Python API](../../../bindings/python/README.md)

Initial acceptance uses specification-derived synthetic data. No vendor-produced
CMTS capture is available. Passing these tests verifies implementation behavior;
it does not establish physical-device interoperability or general DOCSIS
compliance. The evidence matrix identifies the exact supported CCAP revisions.

## Requirements to tests

| Requirement | Implementation and evidence | Validation |
| --- | --- | --- |
| USMER-01: distinct upstream identity and verified layout | Central file registry, common header and [field matrix](format.md) | Literal PNN105 major 2 golden bytes; legacy/type/version rejection; existing decoder regressions |
| USMER-02: bounded Rust API and scientific semantics | Borrowed immutable parser; separate statistics module | Literal fields/endianness, raw/decoded values, frequency centers, every truncation, sample/output limits, EOF, overflow, full octet domain and last index |
| USMER-03: independent synthetic fixtures | [Generator](../../../crates/core/examples/cmts_rxmer_fixtures.rs) and adjacent manifests | Nine checked-in valid scenarios reproduce exactly; 17 generated valid/malformed scenarios; literal golden test independent of generator; existing/out-of-scope output destinations refused |
| USMER-04: optional Rust-backed Python API | Standalone PyO3 crate, hints and [usage guide](../../../bindings/python/README.md) | Installed wheel: bytes/file equivalence, Rust CLI parity, all generated manifests, typed errors and unavailable statistics |
| USMER-05: existing CLI and Qt infrastructure | Canonical JSON, shared chart and offline report adapter | CLI/core JSON equality; viewer metadata, zoom/reset and downstream isolation; native Qt export equality and both-theme screenshots |
| USMER-06: honest traceability and compatibility | Source inventory, manifests, API/CLI/UI guides | MAC hygiene, documentation links, version consistency and explicit unsupported cases |

Primary tests:

- [Core parser and analysis](../../../crates/core/tests/test_cmts_us_ofdma_rxmer.rs)
- [CLI integration](../../../crates/cli/tests/cmts_rxmer.rs)
- [Viewer integration](../../../apps/viewer/tests/cmts_rxmer.rs)
- [Python binding](../../../bindings/python/tests/test_rxmer.py)

The generator is not the only numerical oracle. The core test authors literal
header bytes and expected values separately. It also probes every truncation
of that golden capture, all 256 raw octets, both supported spacings, frequency
overflow and the last representable physical carrier index. The nominal binary independently verifies mean 36 dB and
population standard deviation 0.75 dB after quantization. A missing sample
remains on the frequency grid and never becomes a zero-dB measurement.

## Local acceptance

Verified on Ubuntu 22.04, Rust 1.85.1, Qt 6.8.3 and CPython 3.10 on
2026-11-26:

| Check | Result |
| --- | --- |
| Rust workspace, default features | 246 tests passed |
| Rust workspace, MERLIN enabled | 263 tests passed |
| Rust formatting and all-target Clippy | Passed, including MERLIN-enabled workspace |
| Optional Python wheel | Built and installed; three test methods passed across all 17 generated scenarios |
| Optional Python Rust formatting / Clippy | Passed |
| Full Qt gate | 204 QML tests and six Rust adapter tests passed; native build and unchanged-build reuse passed |
| Native Qt synthetic-notch smoke | Dark/light screenshots inspected; exported JSON equals CLI output for all 128 source samples |
| Native Qt missing-data smoke | Sparse-unavailable, all-unavailable and empty captures opened; complete exports equal CLI output |
| Git/release helper regression suites | 27 and 62 tests passed |
| Fixture-hygiene helper tests | 10 tests passed; read-only capture and installer-demo checks passed |

Debug symbols and incremental compilation were disabled for local Rust builds
to limit generated disk usage; test assertions remained enabled. These are local
development checks, not installed-package or hosted CI results. No release was
published. Screenshots are in the [measurement guide](../../ui/measurements/cmts-upstream-rxmer/README.md#chart-examples).

## Outstanding evidence

- A real CMTS capture to check byte-order/frequency interpretations and vendor behavior.
- Historical legacy PNM105 and PNN105 major 1 layout definitions.
- Boolean wire coding for PreEq; its original octet remains exposed.
- Compatibility evidence for CCAP revisions beyond the two reviewed versions.
- Native Windows/macOS Python wheels and desktop runtime validation.
- Installed-package size and lifecycle checks with this feature on each release platform.

No downstream capacity model, upstream aggregation, live AI provider route or
upstream simulation is added by this foundation.
