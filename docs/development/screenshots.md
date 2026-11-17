# Refresh UI screenshots

[Developer index](README.md) · [Measurement gallery](../ui/measurements.md)

Capture the actual native Qt application from the current build. Do not use
mockups, screenshots from PyPNM, or image edits that invent controls. Use only
the preserved repository fixtures, never customer captures.

## Build and capture

```bash
cd /home/dev01/Projects/raven &&
./scripts/qt-desktop.sh build &&
./scripts/qt-desktop.sh
```

Use a 1280 × 820 application content area for consistent images. A desktop
screenshot tool or an isolated X desktop can capture the real window. Exclude
OS decorations and unrelated windows. Move the pointer away from controls and
wait for file loading, chart rendering and tooltips to settle before capturing.
Do not change the graph data to improve its appearance.

For each theme, choose Settings → System… → Appearance → Dark or Light. Show
the left navigation pane on Home with the top-right panel button. Hide it for
measurement captures to give the charts room. Keep the full reset viewport,
except Spectrum's documented range-tab/power example and the RxMER analysis
popup, which demonstrates the current zoomed range.

Store each measurement’s PNG pairs in `docs/ui/measurements/<file-type>/images/`.
Home images live in `docs/ui/images/home/`. Update the colocated guide and root
README table links with any move. Use the filename pairs below:

| Filename stem (append `-dark.png` and `-light.png`) | Fixture and view |
| --- | --- |
| `home` | Close capture; show navigation |
| `rxmer-bit-loading` | `rxmer.bin`: full range, right-click → Modulation & Capacity… (OSSI defaults) |
| `rxmer-uniform-rates` | Same popup → Estimated gross throughput chip |
| `rxmer` | `rxmer.bin` |
| `spectrum` | `spectrum_analyzer.bin`, 300–350 MHz range tab, WindowAvg N=101 and a selected power band |
| `spectrum-frequency` | Same capture, Frequency dialog with 300000000–350000000 Hz inputs |
| `channel-magnitude` | `channel_estimation.bin`, Magnitude |
| `channel-delay` | Same capture, Group delay |
| `channel-iq` | Same capture, I/Q scatter → Density (blue sparse/red dense) |
| `channel-iq-dots` | Same capture, I/Q scatter → Dots |
| `upstream-pre-eq-iq` / `upstream-pre-eq-iq-dots` | `us_pre_equalizer_coef.bin`, Density / Dots |
| `upstream-pre-eq-last-iq` / `upstream-pre-eq-last-iq-dots` | `us_pre_equalizer_coef_last.bin`, Density / Dots |
| `histogram` | `histogram.bin` |
| `fec-summary` | `fec_summary.bin`, initial profile |
| `modulation-profile` | `modulation_profile.bin`, All profiles, dB |
| `modulation-bits` | Same capture, All profiles, Bit loading |
| `profile-bandwidth` | Same capture, Profile BW grid open |
| `constellation` | `const_display.bin`, all three layers and lower-right MER/EVM overlay enabled |

Fixtures live in `tests/fixtures/pypnm/tests/files/`. Reload each capture before
capturing if previous selections or zoom could persist. Do not include native
file dialogs in the gallery.

## Review before a tag

- Open both images of every pair and check labels, clipping, theme and graph data.
- Confirm Home shows the navigation button and expanded pane.
- Confirm constellation has no numeric axes or sample-count footer; show MER/EVM
  at lower right and its visibility toggle.
- Confirm Spectrum shows the original and 300–350 MHz tabs, compact range-close
  button, N=101, red average, highlighted power band and larger integrated-level,
  frequency-range and bandwidth chips beside the small MAC/RBW chips. The lower
  frequency-selection readout must match the selected power band.
- Confirm modulation images show all four profiles and the correct metric.
- Keep README's Home picture and the gallery's `<picture>` paths consistent.
- Update the user guide when visible controls or supported measurements change.
- Run `./scripts/qt-desktop.sh check` after any UI code changes. A screenshot
  verifies appearance; it does not replace parser, numeric or platform tests.

These images show a local Ubuntu native build. They do not demonstrate Windows,
macOS or packaged installation support. Keep hosted CI evidence separate.

For Correction & Delta, use the current sanitized Pre-EQ fixture and an ignored
copy of the last-update fixture with its encoded capture timestamp increased by
8 seconds (label the copy synthetic). Attach through the left gutter, pass the
precheck and explicitly select Adjust. Keep the CM and CMTS MACs sanitized.
Store `pre-eq-comparison-{dark,light}.png`, `pre-eq-comparison-delay-{dark,light}.png`
and `pre-eq-comparison-iq-{dark,light}.png` beside the current Pre-EQ guide. Never
publish screenshots of private lab identities without separate sanitization.

For OFDM Profile Performance, use the native
[`--ofdm-smoke` workflow](ofdm-profile-performance.md#validation), which renders
real input snapshots and runs the Rust decoder. Store dark/light workspace and
results pairs under `docs/ui/measurements/ofdm-profile-performance/images/`.
The results image captures the real popup's counter content. Use supplied CH-EST
and `--mce --ofdm-profiles=0,1,2,3` with the bundled single captures; do not manufacture
counter values or substitute QML test mocks for the documentation pictures.

## Refresh the Home pair

The existing native harness can capture Home without loading inputs or running
FEC. After building, run from the repository root on the Linux development host:

```bash
QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software \
LD_LIBRARY_PATH="$PWD/local/qt/6.8.3/gcc_64/lib" \
apps/qt/target/debug/raven --ofdm-smoke none none none \
"$PWD/docs/ui/images/home/home" --home-only
```

Require `HOME_SCREENSHOTS_OK`. Review both themes for the expanded navigation,
including OFDM Profile Performance, with the RxMER, Spectrum Analyzer and US Pre-EQ
groups collapsed at startup. Check that the banner blends into the sidebar
in both themes without a rectangular edge and that its lettering remains readable.
The command replaces the existing Home pair;
do not keep dated duplicate PNGs. Before removing other images, check documentation,
QML and packaging references and update any remaining links. Keep active measurement
screenshots and original branding resources.

## Capture single-measurement axes

The native smoke helper can export a capture and save both themed chart frames:

```bash
./scripts/qt-desktop.sh --skip-build --smoke-test \
"file://$PWD/tests/fixtures/pypnm/tests/files/rxmer.bin" \
"file://$PWD/local/rxmer-screenshot-export.json" \
--screenshots "$PWD/docs/ui/measurements/rxmer/images/rxmer"
```

The JSON output must be a new path. Require `UI_SCREENSHOTS_OK`. Optional
`--view INDEX` selects a non-default capture view before export and rendering;
indices follow that capture's view buttons. Append `--density` after `--view 2`
to capture I/Q Density; omit it for Dots. Invalid views fail the capture.
The default installed `--smoke-test INPUT OUTPUT` still performs the original
render/export lifecycle check. Screenshots use actual parsed files and Rust
viewports, not QML mocks. Review vertical Y titles, explicit units, matching X/Y
title fonts, and labels that fit beside the tick values in both themes. OFDM
frames must also show **PLC** inside the shaded protected band.

Qt event-loop exit statuses propagate to the launcher. Failed native smoke or
screenshot requests return nonzero; successful runs still require their expected
render marker and a fresh export.

## AI Assistance setup

Run the [offline AI setup smoke](ai-assistance.md#validation) to refresh
`docs/ui/images/ai/ai-services-{dark,light}.png`. Use no real credentials. Confirm
the gray Off chip opens setup and System → AI reaches that same popup. Refresh
the Home pair when the top bar changes. Configured must never imply verified
access or a working analysis request before those capabilities are implemented.


For the native AI request preview, add `--ai-preview-only` to the OFDM smoke
command, select profile 0 and use output prefix `docs/ui/images/ai/ofdm`.
Require `AI_PREVIEW_OK`; the key is synthetic and no provider operation runs.
