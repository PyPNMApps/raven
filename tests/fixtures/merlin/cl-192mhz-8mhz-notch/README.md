# MERLIN 192 MHz / 8 MHz notch inputs

Unmodified subset of MERLIN's `examples/workbench/cl-192mhz-8mhz-notch` at the
revision in source-manifest.json. Synthetic Profile 3 uses 4096-QAM; the active
grid spans 300.000–489.975 MHz at 25 kHz spacing. RxMER is 42 dB outside the
371 MHz inclusive to 379 MHz exclusive notch and 10 dB inside. The flat 42 dB
case is retained for comparison. These are decoded JSON inputs, not binary PNM.

The channel response is adapted from a sanitized PyPNM capture and frequency-
mapped to this synthetic grid, not measured at these synthetic frequencies.
Retain LICENSE, NOTICE, upstream provenance and channel-estimation-provenance.
The upstream provenance also lists saved workbenches not copied in this subset;
source-manifest.json inventories exactly the files copied here.

PyPNM - Proactive Network Maintenance Toolkit

Run the RAVEN adapter example from the repository root:

```bash
scripts/cargo-local.sh run --release -p viewer --features merlin-fec \
  --example fec_notch --locked -- --notch
```

Use `--flat` for the background-only comparison with the same channel response.
Settings match the CMCE companion: 192 MHz, CP 512, roll-off 128, interleaver
16, supplied channel-estimate receiver, PRBS15 seed 32767/noise seed 41,
10 ms observation, 50 LDPC iterations and at most 300 seconds host runtime.
This is a synthetic experiment, not CableLabs validation or an LTE generator.
