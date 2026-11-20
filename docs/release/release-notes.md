# RAVEN {{TAG}}

**Under development.** Download the package for your system under **Assets**. The native Qt viewer includes the same-version `pnm-decode` CLI and manual; Ubuntu bundles a private CLI; the separate CLI DEB is optional. Rust, Python and a separate Qt SDK are not needed to run the packaged application. APT installs required Ubuntu system libraries and may need network access.

## Choose your download

{{ASSET_TABLE}}

Sizes above are compressed download bytes expressed in decimal MB, not installed sizes. A complete release has five installers and `SHA256SUMS` for verification. Build reports and standalone CLI archives remain workflow artifacts linked under [workflow results]({{RUNS_URL}}). GitHub's **Source code** ZIP/tar archives contain source, not installers.

## Ubuntu 22.04 / 24.04 / 26.04 — x64

### Download and install the latest release from a terminal

Quit RAVEN, then copy and paste the complete block below. It requires `curl` and
`python3`, downloads the **complete desktop DEB**, verifies its checksum,
and installs it. Releases through 0.4.1 also download their required CLI DEB.
APT may download system dependencies and asks for your administrator password. No uninstall is needed for an upgrade.

```bash
(
  set -euo pipefail
  umask 022
  release_version="$(curl -fsSL https://api.github.com/repos/PyPNMApps/raven/releases/latest |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"].removeprefix("v"))')"
  download_dir="$(mktemp -d /tmp/raven-install.XXXXXX)"
  chmod 755 "$download_dir"
  cd "$download_dir"
  release_url="https://github.com/PyPNMApps/raven/releases/download/v${release_version}"
  packages=("raven_${release_version}_amd64.deb")
  if dpkg --compare-versions "$release_version" lt 0.4.2; then
    packages=("pnm-decode_${release_version}_amd64.deb" "${packages[@]}")
  fi
  for asset in SHA256SUMS "${packages[@]}"; do
    curl -fL --retry 3 "$release_url/$asset" -o "$asset"
  done
  sha256sum --ignore-missing --check SHA256SUMS
  sudo apt install "${packages[@]/#/./}"
  if [ -x /opt/raven/pnm-decode ]; then
    /opt/raven/pnm-decode --version
  else
    pnm-decode --version
  fi
  printf 'Downloaded packages: %s\n' "$download_dir"
)
```

The printed temporary directory contains the downloads. If a required package is
missing or verification fails, installation stops. Launch `raven` afterward from
a graphical desktop terminal as your normal user.

### Manual download for this release

Download the desktop DEB and `SHA256SUMS` from this release. In the download directory:

```bash
sha256sum --ignore-missing --check SHA256SUMS &&
sudo apt install ./raven_{{VERSION}}_amd64.deb
```

Launch **RAVEN** from the application menu, or run `raven` inside your graphical desktop session. The bundled CLI is `/opt/raven/pnm-decode`; read its manual with `man /opt/raven/share/pnm-decode.1`. The optional CLI-only DEB provides global `pnm-decode` and `man pnm-decode` commands.

**CLI only:** install just `pnm-decode_{{VERSION}}_amd64.deb` using APT.

**Update:** quit the viewer, then install the new desktop DEB with APT.

**Uninstall the viewer:**

```bash
sudo apt remove raven
```

This also removes the bundled CLI, while preserving any independently installed CLI package. To remove it too, run `sudo apt remove pnm-decode`.

[Ubuntu installation and uninstall guide]({{SOURCE_URL}}/docs/install/ubuntu.md)

## Windows — x64

1. Download and run the **Windows x64 MSI installer** from the table.
2. Select an installation folder and launch **RAVEN** from Start.

The default per-machine location is `%ProgramFiles%\RAVEN`. Qt and Microsoft runtime DLLs are app-local. The bundled CLI is in that folder; the installer does not add it to PATH. From PowerShell at the default location:

```powershell
& "$env:ProgramFiles\RAVEN\pnm-decode.exe" --help
```

For a standalone CLI without the viewer, use the Windows CLI ZIP from the [workflow artifacts]({{RUNS_URL}}), or [build from source]({{SOURCE_URL}}/docs/install/source.md).

The MSI is unsigned. Verify the project release and SHA-256 before approving Windows security/UAC prompts. If double-click does not start installation, use the [PowerShell installer/logging instructions]({{SOURCE_URL}}/docs/install/windows.md#windows-11-security-prompts-or-an-msi-that-does-not-open). These steps do not disable SmartScreen, Defender, or company policy.

**Update:** quit the viewer and install the new MSI (older MSIs upgrade; downgrades are refused).

**Uninstall:** Settings → Apps → Installed apps → RAVEN → Uninstall, managed by Windows Installer. A separately extracted CLI is independent of the desktop installation.

[Windows installation and uninstall guide]({{SOURCE_URL}}/docs/install/windows.md)

## macOS — Apple Silicon or Intel

1. Choose **arm64** for an Apple M-series Mac or **x64** for an Intel Mac.
2. Open the matching DMG and drag **RAVEN** onto **Applications**.
3. Eject the disk image and open the app from Applications.

The deployment target is macOS 13 or newer; native CI uses macOS 15 runners. The app includes Qt and the CLI. It is ad-hoc signed, without Developer ID signing or Apple notarization. If macOS blocks opening it, verify the download source and checksum, then use the operating system's application-approval controls.

The bundled CLI can be run from a terminal:

```bash
"/Applications/RAVEN.app/Contents/MacOS/pnm-decode" --help
```

For a standalone CLI without the viewer, use the matching Darwin CLI archive from [workflow artifacts]({{RUNS_URL}}), or [build from source]({{SOURCE_URL}}/docs/install/source.md).

**Update:** quit the viewer and replace the app in Applications from the new DMG.

**Uninstall:** quit the app and move it from Applications to Trash. Any separately extracted CLI remains independent.

[macOS installation and uninstall guide]({{SOURCE_URL}}/docs/install/macos.md)

## First use

Choose **Open capture**, **Open**, Ctrl+O (Cmd+O on macOS), or drop one local PNM file onto the window. The viewer detects its type from its contents. It supports RxMER, Spectrum Analyzer, DS Channel Estimation, Histogram, FEC Summary, Modulation Profile, Constellation Display, current US pre-EQ and last-update US pre-EQ. Inspect the graph and use the export icon to save full-resolution JSON to a new file. Choose System, Dark or Light under Settings → System → Appearance. Viewing local captures works offline after setup; no modem connection is needed.

[Desktop guide and screenshots]({{SOURCE_URL}}/docs/ui/README.md) · [Measurement gallery]({{SOURCE_URL}}/docs/ui/measurements.md) · [CLI guide]({{SOURCE_URL}}/docs/cli/usage.md) · [Complete JSON examples]({{SOURCE_URL}}/docs/cli/examples/README.md)

## Checksums and validation

Download `SHA256SUMS` with your chosen package and compare its matching entry against your local SHA-256 result. Examples from the download directory:

```bash
# Ubuntu
sha256sum raven_{{VERSION}}_amd64.deb
# macOS Apple Silicon; substitute macos-x64 for an Intel package
shasum -a 256 RAVEN-{{VERSION}}-macos-arm64.dmg
```

```powershell
Get-FileHash .\RAVEN-{{VERSION}}-windows-x64.msi -Algorithm SHA256
```

These packages are assembled only after CodeQL, code checks, the complete native matrix, package integrity/size checks and installed-package lifecycle tests pass for this tag. Lifecycle tests cover install, open/render/export and removal on all three Ubuntu versions, Windows x64, macOS arm64 and macOS x64. See [this tag's workflow runs]({{RUNS_URL}}) for results. These development builds are not a release-readiness claim. Publication is automatic only after uploaded asset metadata and downloaded bytes are verified.

[Packaging, licenses and prerequisites]({{SOURCE_URL}}/docs/release/packaging.md) · [JSON field contracts]({{SOURCE_URL}}/docs/reference/formats.md)
