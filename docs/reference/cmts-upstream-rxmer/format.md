# CMTS upstream OFDMA RxMER

Initial support is for `0x504E4E69` (PNN105), header **major 2**, vendor minor
retained. This is distinct from downstream `0x504E4E04` (PNN4). Binary files do
not identify their DOCSIS generation: JSON keeps that context unknown.

[Rust API](api.md) · [Python API](../../../bindings/python/README.md) · [Validation](validation.md) · [Desktop guide](../../ui/measurements/cmts-upstream-rxmer/README.md)

## Source inventory and compatibility

| Source | Relevant evidence | Support |
| --- | --- | --- |
| CCAP-OSSIv3.1-I18-200610 | 7.3.2.5; 7.3.3.1 Table 397; 7.3.5.5 Tables 409/410, printed pp396–398/418–420 | PNN major 2 |
| [CCAP-OSSIv4.0-I02-200311](https://account.cablelabs.com/server/alfresco/ce99a2ac-f15e-40bc-ab5d-35bcb0d1849c) | Same sections, Table 411 | Same PNN major 2 fields; not a claim about later revisions |
| CM-OSSIv3.1-I27 / CM-OSSIv4.0-I13 | Downstream RxMER definitions | Not upstream layout authority |
| [CANN-I20-200715](https://account.cablelabs.com/server/alfresco/9fe67300-bfa8-426e-8363-c67ca6c73066), 12.1.4 Table 22 | PNM/PNN RxMER type codes | Identify both; layout support separate |
| DOCS-PNM-MIB 2024-07-05, docsPnmCmtsUsOfdmaRxMerFileName | Both signatures; delegates layout to CCAP | Corroboration only |
| PHYv3.1-I20 / PHYv4.0-I07 | Upstream probe/numerology context | Not file layout authority |

Private/local PDFs and hashes live in ignored `development/specs/`; absent from
fresh clones. The I18 Table 410 and scaling page 396 and I02 Table 411 PDF page420 were visually inspected. Source text extracts
alone do not establish rendered-page verification. No real CMTS capture has
been validated. Synthetic tests establish implementation behavior, not device
interoperability or general DOCSIS compliance.

## Field evidence matrix: PNN major 2

Offsets are zero-based bytes. All rows apply to the two CCAP revisions above;
field order/width come from Table 410/411 and definitions 7.3.5.5.7.1–14.
Multibyte unsigned fields use most-significant octet first, interpreting the
specified right-justified hexadecimal field order; this interpretation still
needs a vendor capture check. Limits below are parser policy unless noted.

| Field | Offset | Width | Type/order | Units/scaling | Validation / reserved values |
| --- | ---: | ---: | --- | --- | --- |
| File type | 0 | 4 | ordered octets | `504E4E69` | exact signature |
| Major | 4 | 1 | unsigned | header version | only 2; unknowns unsupported |
| Minor | 5 | 1 | unsigned | vendor version | retain 0–255; no generation inference |
| Capture time | 6 | 4 | unsigned BE | Unix UTC seconds | retain 0–u32 max |
| IfIndex | 10 | 4 | unsigned BE | upstream interface | preserve; never truncate to channel ID |
| Unique CCAP ID | 14 | 256 | original octets | NUL-terminated IP/FQDN string | require terminator and UTF-8 prefix; retain all padding |
| RPD ID | 270 | 6 | octets | identifier | retain zero default for integrated CCAP |
| RPD port | 276 | 1 | unsigned | port | zero default retained |
| CM MAC | 277 | 6 | octets | target modem | retain six bytes |
| Averages | 283 | 2 | unsigned BE | probe averaging metadata | retain encoded value; no averaging performed |
| PreEq | 285 | 1 | raw octet | copy of PreEqOnOff | preserve; Boolean wire coding not established here |
| Subcarrier-zero center | 286 | 4 | unsigned BE | Hz interpretation | checked frequency arithmetic |
| First active index | 290 | 2 | unsigned BE | physical subcarrier | checked last index |
| Spacing | 292 | 1 | unsigned | kHz | support 25/50; other encoding unsupported |
| Data length | 293 | 4 | unsigned BE | payload octets | exact EOF; caller sample limit |
| RxMER data | 297 | length | unsigned octets | raw/4 dB | 0–254 valid; 255 unavailable |

Header length is `4+1+1+4+4+256+6+1+6+2+1+4+2+1+4 = 297`.
RxMerData/Value (7.3.2.5/7.3.2.5.1) explicitly defines quarter-dB values,
0–63.5 dB and `0xFF` unavailable. No other byte is reserved; excluded and
unmeasured cannot be distinguished from this sentinel. Keep nulls and positions.
For sample `i`, physical index is `first+i` and center frequency is
`zero_hz+(first+i)*spacing_khz*1000`. This follows contiguous physical-carrier
ordering in RxMerData; holes are retained rather than compressed/interpolated.

## Unsupported layouts

| Identity | Established evidence | Decoder policy |
| --- | --- | --- |
| PNM105 `504E4D69` | CANN code; CCAP says no major/minor fields | Recognized, UnsupportedLayout; complete historical layout not verified |
| PNN105 major1 | I18 describes major2 adding RPD fields | UnsupportedVersion; no reconstruction by deleting fields |
| PNN105 other major | No verified definition | UnsupportedVersion, no fallback |

The prompt's legacy field list totals 288 bytes, but is an investigation baseline,
not evidence authorizing a legacy encoder/decoder. No valid legacy fixture is
claimed. Raw input remains available through PnmFile even after a diagnostic.

PreEq wire Boolean codes, later CCAP revisions and legacy format evidence remain
open. Preserve their bytes or reject unsupported layouts rather than guessing.

PHYv3.1-I20 section 9.4.8 describes a CMTS upstream-probe measurement, not a
downstream data-symbol measurement. PHYv4.0-I07 section 9 delegates PNM to that
3.1 section. Those clear text extracts were reviewed for measurement context;
they do not replace either CCAP table. The parser performs no probe averaging
or receiver simulation. The statistics API labels its arithmetic dB mean.
