# RAVEN

**RF Analysis, Visualization & Engineering Navigator**

*A cross-platform DOCSIS 3.1/4.0 PNM analysis and visualization toolkit.*

**Under development.** See the signal. Reveal the impairment.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/images/raven-logo-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="docs/images/raven-logo-light.png">
    <img alt="PyPNMApps RAVEN" src="docs/images/raven-logo-light.png" width="461">
  </picture>
</p>

[![CI](https://github.com/PyPNMApps/raven/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/PyPNMApps/raven/actions/workflows/ci.yml)
[![Native releases](https://github.com/PyPNMApps/raven/actions/workflows/release.yml/badge.svg)](https://github.com/PyPNMApps/raven/actions/workflows/release.yml)
[![Ubuntu matrix](https://img.shields.io/badge/Ubuntu-22.04%20%7C%2024.04%20%7C%2026.04-E95420?logo=ubuntu)](docs/development/ci.md)
[![Windows matrix](https://img.shields.io/badge/Windows-x64%20MSI-0078D4)](docs/development/ci.md)
[![macOS matrix](https://img.shields.io/badge/macOS-Apple%20Silicon%20%7C%20Intel%20DMG-555555?logo=apple)](docs/development/ci.md)
[![Component licenses](https://img.shields.io/badge/license-Apache--2.0%20%2F%20GPL--3.0--only-blue)](LICENSE)
[![AI Assistance](https://img.shields.io/badge/AI_Assistance-Optional-14b8a6)](docs/ui/ai-assistance.md)

Tag releases require the full native matrix: Ubuntu DEBs, Windows MSI, and
macOS Apple Silicon/Intel DMGs. [Installation by OS](docs/install/README.md).

An offline desktop viewer for DOCSIS PNM capture files. Open a capture, explore
its graph, and export decoded JSON. The native Qt interface uses a Rust backend.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/ui/images/home/home-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="docs/ui/images/home/home-light.png">
  <img alt="RAVEN starting screen with Single Capture, OFDM Profile Performance, RxMER, Spectrum Analyzer and US Pre-EQ navigation" src="docs/ui/images/home/home-dark.png" width="1000">
</picture>

## Start here

[Downloads](https://github.com/PyPNMApps/raven/releases) ·
[Installation](docs/install/README.md) ·
[Launch the viewer](docs/ui/README.md#launch) ·
[User guide](docs/ui/README.md) ·
[Workspaces](docs/ui/README.md#workspaces) ·
[Screenshots and measurements](docs/ui/measurements.md)

## Workspaces

Use **Single Capture** to open individual files. The left navigation also offers
RxMER and Spectrum Analyzer collections, US Pre-EQ comparison, and
[**OFDM Profile Performance**](docs/ui/measurements/ofdm-profile-performance/README.md).

For OFDM performance, select modulation-profile and RxMER captures, optionally
add channel estimation, and check the profiles to test. The graph overlays RxMER
and profile references; **FEC Performance** runs each checked profile and shows
separate FEC, BER and LDPC results. All simulation runs locally in Rust using
MERLIN. See the [demo walkthrough](docs/ui/examples.md#ofdm-profile-performance).

Optionally sign in with ChatGPT or configure an OpenAI API key through
**AI Assistance** for further analysis of
OFDM Profile Performance results. Preview and approve the exact data before
sending; Rust and MERLIN remain authoritative for calculations. A local Demo AI
mode lets you try the workflow without an API key. See the
[AI Assistance guide](docs/ui/ai-assistance.md).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/ui/measurements/ofdm-profile-performance/images/ofdm-profile-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="docs/ui/measurements/ofdm-profile-performance/images/ofdm-profile-light.png">
  <img alt="OFDM workspace with profile checkboxes, RxMER overlay and shaded PLC carriers" src="docs/ui/measurements/ofdm-profile-performance/images/ofdm-profile-dark.png" width="1000">
</picture>

## Supported views

| Capture | Views |
| --- | --- |
| [OFDM Profile Performance](docs/ui/measurements/ofdm-profile-performance/README.md) | Profile/RxMER overlay, optional channel estimation and MCE, simulated FEC/LDPC counters |
| [RxMER](docs/ui/measurements/rxmer/README.md) | MER by frequency; [Playback](docs/ui/measurements/rxmer/playback.md); [Min-Avg-Max](docs/ui/measurements/rxmer/README.md#min-avg-max); [Modulation & Capacity](docs/ui/measurements/rxmer/README.md#modulation--capacity) |
| [Spectrum Analyzer](docs/ui/measurements/spectrum-analyzer/README.md) | Amplitude/RBW, [Min-Avg-Max and Playback](docs/ui/measurements/spectrum-analyzer/collections.md) |
| [DS Channel Estimation](docs/ui/measurements/channel-estimation/README.md) | Magnitude, group delay and I/Q dots and density |
| [US Pre-EQ (current)](docs/ui/measurements/upstream-pre-eq/README.md) | Coefficient magnitude, group delay, I/Q, and [Correction & Delta](docs/ui/measurements/upstream-pre-eq/README.md#correction--delta-from-the-left-gutter) |
| [US Pre-EQ (last update)](docs/ui/measurements/upstream-pre-eq-last/README.md) | RNG-RSP update magnitude, group delay and I/Q dots and density |
| [Histogram](docs/ui/measurements/histogram/README.md) | Hit counts by bin index |
| [FEC Summary](docs/ui/measurements/fec-summary/README.md) | Per-profile interval codewords over UTC time |
| [Constellation Display](docs/ui/measurements/constellation/README.md) | Soft/hard decisions and symbol boundaries |
| [Modulation Profile](docs/ui/measurements/modulation-profile/README.md) | Ideal Shannon minimum MER and bit loading by frequency |

Choose System, Dark or Light appearance. Drag across a graph to zoom and
double-click to reset; the I/Q dots and density view shows the full capture.

Tag builds automatically publish [native packages](docs/release/packaging.md)
after all required checks and uploaded-byte verification pass. Automatic tag
delivery retains Ubuntu, Windows and both macOS architectures. Local
builds have been exercised on Ubuntu 22.04 x86_64; hosted results for the full
platform matrix remain pending.

## CLI and development

- [Build the standalone `pnm-decode` CLI](docs/install/source.md) · [CLI user guide](docs/cli/usage.md)
- [JSON examples for each PNM file type](docs/cli/usage.md#json-examples-by-file-type)
- [Supported formats and limits](docs/reference/formats.md)
- [Development setup](docs/install/development.md) · [Qt build and checks](docs/development/qt.md)
- [Rust APIs and examples](docs/development/rust.md)
- [Developer guide](docs/development/README.md) · [Version bumps and releases](docs/development/releasing.md)
- [All documentation](docs/README.md) · [Repository conventions](AGENTS.md)

## License

Core libraries and development tooling use Apache-2.0; desktop components use
GPL-3.0-only. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
Built with guidance and example captures from **PyPNM**; third-party material
retains its original notices.
