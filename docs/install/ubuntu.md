# Install on Ubuntu

[Installation overview](README.md) · [Windows](windows.md) · [macOS](macos.md)

RAVEN is **under development**. Release validation targets Ubuntu
22.04, 24.04 and 26.04 on x64, using the same DEBs built on 22.04. Check the
release's assets and workflow results; older releases may cover only 22.04.

## Download and install

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

### Manual download

1. Open [GitHub Releases](https://github.com/PyPNMApps/raven/releases).
2. Download `raven_<version>_amd64.deb` and `SHA256SUMS` into Downloads.
   From 0.4.2, the desktop DEB includes its own decoder and manual. The separate
   `pnm-decode` DEB is only needed for a standalone CLI installation.
   Releases through 0.4.1 require both same-version DEBs.
3. Quit the viewer. Open a terminal in that download folder, verify the checksums,
   then install the desktop DEB using its actual filename:

```bash
# Replace VERSION with the version you downloaded (0.4.2 or newer).
sha256sum --ignore-missing --check SHA256SUMS &&
sudo apt install ./raven_VERSION_amd64.deb &&
/opt/raven/pnm-decode --version
```

Launch **RAVEN** from the application menu, or run `raven`
from a graphical desktop terminal as your normal user. No repository checkout,
Rust, Python, or separate Qt SDK is needed to run packaged binaries. APT may
download system libraries. An `_apt` sandbox permission notice by itself is
nonfatal; confirm installation succeeded.

Capture, archive and folder selection starts in the bundled
[demo files](../ui/examples.md) at `/opt/raven/files/demo`.
Export JSON to your own writable folder.

## CLI only

The desktop includes `/opt/raven/pnm-decode` and its manual at
`/opt/raven/share/pnm-decode.1`. For a global `pnm-decode` command and
`man pnm-decode`, install the optional standalone CLI DEB below. Both packages
can coexist and be removed independently.

Download the release's `pnm-decode_<version>_amd64.deb` and `SHA256SUMS`.
Verify its hash and install that single DEB with `sudo apt install ./<filename>`.
Run `pnm-decode --help` or `man pnm-decode`. The viewer package is not required.

## Update

No uninstall is needed: quit the viewer and install the new desktop DEB using the same steps. Installed
Ubuntu builds with the updater can also use About → Check for updates, then
Install update and restart. Export work before starting and approve the OS
authentication dialog. After success, the viewer restarts automatically once any
active capture/export work finishes. If launching fails, the existing window
remains open with a Restart viewer retry button. Older builds with the manual
restart flow require that button after installation. Versions 0.1.8/0.1.9 need one manual upgrade to fix their download
directory permissions before testing future in-app updates.

## Uninstall

**About → Uninstall…** at the bottom of the dialog opens these platform-specific
instructions. Follow the removal steps below; the viewer does not remove itself.

Remove both the viewer and CLI:

```bash
sudo apt remove raven pnm-decode
```

To keep the CLI, remove only `raven`. Your capture files and source
checkout are separate from the installed packages and remain intact.

## Add to Favorites

Open Ubuntu's application grid, search for **RAVEN**, right-click
its launcher and select **Add to Favorites** (or **Pin to Dash**, depending on
your GNOME version). Dragging an icon from a terminal is not required.

The installed desktop entry is `raven.desktop`. The viewer sets the
same Qt desktop ID and X11 window class so GNOME can associate a running window
with its launcher. After installing a release with this fix, close all older
viewer windows and reopen it from the application grid before pinning. Remove
any old duplicate favorite through its context menu first. A source-only build
has no installed desktop entry unless you install the desktop package.

The first RAVEN installation replaces the former `pnm-viewer-toolkit` package
through APT package metadata; `pnm-decode` remains separately owned. Use manual
installation for this first transition because the older updater expects the
previous names. From 0.4.2, RAVEN updates its complete desktop DEB without
changing any independently installed CLI package.

## Optional saved AI connections

Remembering AI credentials requires an unlocked Secret Service keyring, normally
GNOME Keyring on Ubuntu Desktop. A headless installation without that service can
still use session-only credentials. RAVEN never falls back to a plaintext key
file. See [AI Assistance](../ui/ai-assistance.md) for Remember, Connect and Forget.
