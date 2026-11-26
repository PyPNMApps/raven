# Rust API and ownership

[Wire evidence](format.md) · [Python API](../../../bindings/python/README.md)

Use the `pnm` library directly; Qt, HTTP and Python are not required. Run from
the repository root to resolve the example fixture path below.

```rust
use pnm::pnm::{
    parser::{cmts_us_ofdma_rxmer::CmtsUsOfdmaRxMer,
             pnm_header::HasPnmHeader, parse_limits::ParseLimits},
    analysis::cmts_upstream_rxmer::statistics,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read("tests/fixtures/synthetic/cmts-upstream-rxmer/golden.bin")?;
    let capture = CmtsUsOfdmaRxMer::parse(&bytes, ParseLimits::default())?;
    println!("{:?}", capture.pnm_header());
    println!("{:?}", capture.metadata());
    for sample in capture.samples() {
        println!("{} {} {:?}", sample.subcarrier_index, sample.frequency_hz, sample.rxmer_db);
    }
    println!("{:?}", statistics(&capture));
    println!("{}", capture.to_json(1024 * 1024)?);
    Ok(())
}
```

For an executable example with bounded regular-file loading:

```bash
./scripts/cargo-local.sh run -p pnm --example cmts_rxmer --locked -- \
  tests/fixtures/synthetic/cmts-upstream-rxmer/golden.bin
```

Decoded JSON goes to standard output; metadata and statistics go to standard
error. The [CLI](../../cli/usage.md) uses the same parser and JSON representation.

| Operation | Public API | Result |
| --- | --- | --- |
| Decode bytes | `CmtsUsOfdmaRxMer::parse(&bytes, limits)` | Validated capture borrowing `bytes` |
| Common header | `capture.pnm_header()` | Signature, header version and UTC capture time |
| Upstream metadata | `capture.metadata()` | CCAP, RPD, CM, ifIndex and frequency grid |
| Raw samples | `capture.raw_values()` | Borrowed original octets |
| Decoded samples | `capture.samples()` | Lazy typed records, including unavailable positions |
| MER values | `capture.values_db()` | Lazy `Option<f64>` values in dB |
| Frequency axis | `capture.frequencies_hz()` | Lazy physical carrier centers in Hz |
| Summary | `statistics(&capture)` | Available-only counts, extrema and arithmetic dB mean |
| JSON export | `capture.to_json(max_bytes)` | Canonical JSON subject to an output byte budget |

Core parsing borrows the file bytes and never opens files or contacts a server.
Callers choose file loading policy; the viewer and optional Python loader bound
reads and accept regular files. `PnmFile::from_bytes(...).decode(...)` is the
same central dispatch used by `pnm-decode`; original bytes remain inspectable
after unsupported input. Typed immutable metadata prevents callers from mutating
a validated frequency grid. Iterators do not allocate full sample arrays.

Summary minimum/maximum and arithmetic dB mean cover available carriers only;
this is descriptive carrier statistics, not linear-power averaging, modem error
rate or a new PHY health threshold. Zero valid samples produce null extrema/mean.

## Errors and limits

Parsing returns `pnm::diagnostic::ParseError`, containing `code`, `field` and
zero-based source byte `offset`. Match its code rather than parsing error text.

| Code | Meaning for this parser |
| --- | --- |
| `UnsupportedType` | Unknown signature, or a different measurement passed to the typed decoder |
| `UnsupportedLayout` | Recognized legacy PNM105 without an enabled layout |
| `UnsupportedVersion` | Unsupported PNN105 major version; no fallback |
| `Truncated` | Incomplete header or payload; `field` identifies the stage |
| `InvalidField` | Invalid CCAP string or unsupported spacing value |
| `LengthMismatch` | Declared payload length does not match EOF, including trailing data |
| `LimitExceeded` | Declared sample count exceeds `ParseLimits::max_samples` |
| `ArithmeticOverflow` | Physical carrier index or frequency cannot fit its validated type |

The default sample budget is 65,536. JSON has a separate caller-selected byte
budget and returns a serialization error if exceeded. File I/O errors belong
to the caller's loader, as shown in the compiled example.

No reserved RxMER octets other than `0xff` are defined by these revisions. No
sample-encoding selector exists in this profile; an unknown major version is
unsupported rather than guessed. PreEq raw bytes remain uninterpreted because
Boolean wire coding is not established by the reviewed table. No measured
DOCSIS generation, synthetic status or physical exclusion reason is fabricated.

The compiled [Rust example](../../../crates/core/examples/cmts_rxmer.rs) exercises
these public methods. Initial acceptance uses synthetic files; see the wire
contract for supported revisions and the outstanding real-capture check.
