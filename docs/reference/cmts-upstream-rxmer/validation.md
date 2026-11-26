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

This is the original foundation acceptance record, before the v0.6.0 publication
and the later consumer review below.

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
- Native Windows/macOS Python wheels and CMTS-specific installed capture tests.
- CMTS-specific installed capture tests on each release platform; general
  v0.6.0 package size/lifecycle gates passed as recorded below.

No downstream capacity model, upstream aggregation, live AI provider route or
upstream simulation is added by this foundation.

## MERLIN consumer contract review

Reviewed on 2026-11-26 against clean RAVEN `v0.6.0`:

- Annotated tag object: `6145dd1114d4841ded2192a3ba2dd1cd0b8e3823`.
- Peeled commit / reviewed HEAD: `40d88f4ecd0658fb921d70063af000d1bb918d99`.
- Local and GitHub tag objects and commit targets agree.
- No parser, common-header, dispatch, diagnostic, serialization, binding or public
  API implementation change was needed. Added regressions cover missing consumer
  boundaries; their code remains uncommitted until separately saved.

**Ready for MERLIN integration; no RAVEN source change required.** Here, source
change means a decoder/API implementation change; the review adds tests and this
acceptance record. MERLIN can pin the existing commit above. There is no newly
created immutable revision or replacement tag.

| Consumer contract | Existing operation / regression evidence |
| --- | --- |
| PNN105 major 2, vendor minor, 297-byte header, distinct downstream route | `PnmHeader::from_bytes`, central `FILE_TYPES`/`DecodedPnm`; `independent_golden_fields_values_and_json`, `unsupported_layouts_types_versions_and_encodings_do_not_fall_back` |
| Raw metadata including zero epoch, full ifIndex, CCAP padding, RPD/CM identities, averages and PreEq octet | `metadata`, `pnm_header`, `to_json`; added `opaque_metadata_and_zero_epoch_are_preserved_losslessly`, including invalid UTF-8/control bytes after the first NUL |
| Strict invalid text before NUL | Extended `unsupported_layouts_types_versions_and_encodings_do_not_fall_back`: invalid UTF-8, ASCII/Unicode controls all return `InvalidField`, field `ccap_id_encoding`, byte offset 14 |
| Independent golden values, all 256 octets and physical frequency positions | `independent_golden_fields_values_and_json`, `complete_octet_domain_and_last_representable_carrier`; only 255 is unavailable |
| Caller-selected 4096-sample limit | Added `explicit_consumer_limit_accepts_4096_and_rejects_4097_before_payload_work`; 4097 is valid under the default budget, but `max_samples: 4096` returns `LimitExceeded`, field `data_length`, offset 293, even when the declared oversized payload is absent |
| Spacing, empty/missing readings, overflow, every truncation and exact EOF | `null_empty_frequency_and_index_boundaries`, `every_truncation_and_declared_length_is_bounded`; strengthened absent extrema/mean assertions |
| Borrowing and bounded output | Raw sample and CCAP-padding pointers borrow original input; lazy sample/value/frequency iterators remain unchanged; golden test rejects an undersized JSON output budget |
| Scientific statistics and nominal fixture | Available-only arithmetic dB mean 21.875 on the golden vector; null extrema/mean for no available readings; existing mean 36 dB / population SD 0.75 dB fixture assertions unchanged |
| Optional Python parity and ownership | Existing bytes/file/manifest tests plus added `test_native_ownership_and_structured_errors_match_rust_contract`: actual Rust CLI JSON parity, original/raw bytes after temporary input release and exact error attributes for both Python entry points |

The accepted 4096/4097 test inputs intentionally extend beyond a 4096-point FFT
grid. RAVEN validates the encoded field widths and checked frequency arithmetic,
not MERLIN's simulation layout. MERLIN retains FFT extent and complete measurement
association checks, its own owned snapshots, source hashes/JSON, resource policy,
synthetic Designer and saved-workbench schema. Its exhaustive diagnostic mapping
must handle `ErrorCode::InvalidField`. None of these consumer responsibilities
adds a dependency to `pnm` or changes RAVEN's JSON contract.

### Checks executed for this review

The existing local Rust toolchain/cache was used, with debug symbols and
incremental compilation disabled as in the foundation checks. No caches were
deleted.

| Check | Result |
| --- | --- |
| Initial upstream core test target at the tag | Seven tests passed before changes |
| Upstream core target with added regressions | Nine tests passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy -p pnm --all-targets --locked -- -D warnings` | Passed |
| Existing `pnm-decode` upstream integration test | Passed against a locally built 0.6.0 CLI |
| Installed Python tests | Four test methods passed across all 17 generated scenarios, compared with the Rust 0.6.0 CLI |

The installed wheel used here reports **0.5.1**, from the earlier local native
build. Its Python wrapper matches current source byte-for-byte; current native
binding source was also reviewed and is unchanged. These are functional parity
results for that installed wheel, not a claim that a fresh 0.6.0 wheel was built
or installed. No wheel, Qt build, desktop package, complete workspace or installed
lifecycle check was rebuilt/rerun locally for this unchanged implementation
review. Shared decoder/header/dispatch/serialization code was untouched, so the
targeted checks were sufficient for the added regressions.

### Hosted status refreshed separately

The [commit CI](https://github.com/PyPNMApps/raven/actions/runs/45870321933)
and [v0.6.0 native release workflow](https://github.com/PyPNMApps/raven/actions/runs/45870321729)
both completed successfully. The release run records successful native jobs for
Ubuntu 22.04/24.04/26.04, Windows and both Mac architectures; three Ubuntu DEB
lifecycle jobs and the Windows/macOS installed MSI/DMG lifecycle steps succeeded.
The workflow's conditional preflight/unused lifecycle branches were skipped;
they are not missing native platform results.

[v0.6.0 is published](https://github.com/PyPNMApps/raven/releases/tag/v0.6.0)
as a stable, non-draft release with five installers and SHA256SUMS. This review
read hosted status/asset metadata only: it did not publish, modify the release,
download/revalidate installer bytes or rerun installed applications. Those hosted
general package results do not establish CMTS field-capture interoperability or
CMTS-specific installed-view coverage. Historical layouts, later CCAP revisions,
PreEq Boolean wire coding and the documented byte-order interpretation remain
subject to the evidence limits above.
