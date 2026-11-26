# Optional Rust-backed Python API

This standalone PyO3/maturin target depends on `crates/core`, never Qt or the
viewer. Desktop installers do not contain Python. Version metadata follows the
root `VERSION`; no Python package is published by this task. There was no existing
RAVEN binding framework. PyO3 0.30.1 fits Rust1.85 and provides typed native
exceptions; maturin1.8.3 builds the optional wheel. NumPy is unnecessary for this
bounded scalar API. The wheel is built for the current native CPython/platform;
other platforms need their own builds.

From the project root, using the existing development Rust prerequisites:

```bash
python3 -m venv local/python-api
local/python-api/bin/python -m pip install maturin==1.8.3
RUSTUP_HOME="$PWD/local/rustup" CARGO_HOME="$PWD/local/cargo" \
PATH="$PWD/local/cargo/bin:$PATH" local/python-api/bin/maturin build \
  --manifest-path bindings/python/Cargo.toml --locked --out local/python-wheels
local/python-api/bin/python -m pip install --no-deps local/python-wheels/*.whl
./scripts/cargo-local.sh build -p pnm-decode --locked
local/python-api/bin/python -m unittest discover -s bindings/python/tests -v
```

No administrator access is needed. These are development commands; do not add
Python, maturin or PyO3 to the desktop's dependency tree. Use the intended
interpreter/virtual environment when building for another CPython version.
On Ubuntu, creating the virtual environment requires the `python3-venv` package.

```python
from raven_pnm import CmtsUsOfdmaRxMer, ParseFailure

capture = CmtsUsOfdmaRxMer.from_file(
    'tests/fixtures/synthetic/cmts-upstream-rxmer/golden.bin'
)
print(capture.header)
print(capture.metadata['ccap_id'], capture.metadata['if_index'])
print(capture.statistics)
for sample in capture.samples:
    print(sample.subcarrier_index, sample.frequency_hz,
          sample.rxmer_db, sample.valid, sample.status)
assert capture.raw_values[-1] == 255
assert capture.samples[-1].rxmer_db is None
same = CmtsUsOfdmaRxMer.from_bytes(capture.original_bytes())
assert same.to_json() == capture.to_json()
```

| Operation | Python API | Units / missing values |
| --- | --- | --- |
| Decode bytes or a file | `from_bytes(data)`, `from_file(path)` | Complete original binary input |
| Header and identity | `header`, `metadata` | Header version is distinct from DOCSIS generation |
| Sample records | `samples` | Typed immutable records; unavailable MER is `None` |
| Raw octets | `raw_values` | Includes original `255` markers |
| Frequency axis | `frequency_axis` | Tuple of carrier centers in Hz, including gaps |
| Summary | `statistics` | Counts and descriptive arithmetic dB statistics from Rust |
| Export | `to_json()`, `original_bytes()` | Canonical JSON or unmodified source bytes |

Rust validates before retaining one owned original byte buffer. Python parses
canonical Rust JSON once and wraps those already-decoded records as typed frozen
samples; it does no scaling, frequency or statistical calculation. Metadata and
sample properties are presentation snapshots; canonical export remains the native
immutable original result. `raw_values`/`original_bytes` explicitly allocate
Python bytes. Sample wrappers are cached. Inputs and output sizes are bounded.

`ParseFailure` exposes `code`, `field` and original byte `offset`; missing files
raise `OSError`. No layout fallback or desktop launch occurs. Hints and `py.typed`
ship with the package. All-unavailable statistics remain null, with zero valid
count. This is synthetic-only acceptance, not real-CMTS interoperability.

```python
try:
    capture = CmtsUsOfdmaRxMer.from_bytes(b"bad input")
except ParseFailure as error:
    print(error.code, error.field, error.offset)
except OSError as error:
    print(f"Cannot read the capture: {error}")
```

Inputs are limited to 65,833 bytes (297 header bytes plus 65,536 samples);
oversized input raises `ParseFailure` with `code == 'LimitExceeded'` before
retaining an owned snapshot. Export is limited to 32 MiB. A native serialization
failure raises `ValueError`. See the [Rust API error table](../../docs/reference/cmts-upstream-rxmer/api.md#errors-and-limits)
for parser codes shared by both interfaces.

The checked-in examples contain nine valid scenarios. To also exercise all
generated malformed cases, create a new output directory under `local/` and select it for the
Python tests:

```bash
./scripts/cargo-local.sh run -p pnm --example cmts_rxmer_fixtures --locked -- \
  local/python-api-fixtures
RAVEN_FIXTURE_DIR="$PWD/local/python-api-fixtures" \
  local/python-api/bin/python -m unittest discover -s bindings/python/tests -v
```

See [wire contract](../../docs/reference/cmts-upstream-rxmer/format.md) and
[desktop guide](../../docs/ui/measurements/cmts-upstream-rxmer/README.md).
