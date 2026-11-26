# Synthetic CMTS upstream RxMER

Generated from CCAP-OSSI 3.1 I18 Table 410, also checked against 4.0 I02
Table 411. These are synthetic examples, not vendor captures. Each binary has
an adjacent manifest with expected values and specification references.

The nine checked-in valid scenarios are a literal golden sample, uniform MER
at 25/50 kHz spacing, a localized notch, a slope, sparse unavailable carriers,
all-unavailable carriers, an empty payload and a nominal 36 dB example. The generator additionally
creates eight deliberately malformed cases in a new explicitly selected output
directory; those cases are not installer or sharing examples.

`mer-36db-sd-0p75db.bin` has 1,024 valid carriers, an arithmetic mean of 36 dB
and a population standard deviation of exactly 0.75 dB after quarter-dB encoding.
It uses a deterministically shuffled, symmetric normal-like histogram spanning
33.5–38.5 dB. Spacing is 25 kHz; carrier centers run from 20.400 to 45.975 MHz.
The standard deviation describes variation across the MER readings, not I/Q
noise samples. Keep its adjacent JSON manifest with the binary.

See the [wire contract](../../../../docs/reference/cmts-upstream-rxmer/format.md),
[user guide](../../../../docs/ui/measurements/cmts-upstream-rxmer/README.md) and
[generator](../../../../crates/core/examples/cmts_rxmer_fixtures.rs).
