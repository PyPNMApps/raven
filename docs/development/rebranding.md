# RAVEN product migration

RAVEN means **RF Analysis, Visualization & Engineering Navigator**.
The tagline is *See the signal. Reveal the impairment.* PyPNMApps maintains the
DOCSIS 3.1/4.0 PNM visualization, analysis and RF diagnostics toolkit.

The application is in development. This migration retains the existing Rust and
Qt/QML architecture, parser behavior, JSON schemas and supported platforms.
The `pnm` core, `viewer` services, `logging`, `pnm-decode` command/manual and
`Pnm.Viewer` QML URI remain. The current directory layout is retained.

## Identity and ownership

| Consumer | RAVEN identity |
| --- | --- |
| Product/title | `RAVEN` |
| Desktop Cargo package / executable | `raven-desktop` / `raven` (`raven.exe`) |
| Ubuntu GUI package | `raven`; separate same-version `pnm-decode` dependency |
| Linux paths | `/opt/raven/raven`, `/usr/bin/raven`, `raven.desktop` |
| Linux icon / WM_CLASS | `raven` |
| Windows installer | `RAVEN-{version}-windows-x64.msi` |
| Windows install / shortcuts | `RAVEN`, optional desktop shortcut |
| macOS app / bundle ID | `RAVEN.app` / `org.pypnmapps.raven` |
| macOS disk images | `RAVEN-{version}-macos-arm64.dmg`, `RAVEN-{version}-macos-x64.dmg` |
| Organization | `PyPNMApps` |

Root `product.json` owns the active repository and product metadata. The desktop
service build generates Rust constants from it; release tooling reads the same
file. The updater, report link, project link, download User-Agent and package
notices use this identity. Public parser/measurement data contracts are unchanged.
VERSION still owns every local Cargo package version and both lockfiles.

## First migration from PNM Viewer Toolkit

- Ubuntu: install the same-version `pnm-decode` and `raven` DEBs together. The new
  desktop package declares `Conflicts` and `Replaces` for `pnm-viewer-toolkit`,
  allowing APT to replace it. Removing RAVEN must preserve the separately owned
  CLI. No maintainer script deletes another package's files.
- Windows: run the new MSI as a normal installer. Preserve UpgradeCode
  `54ACD1D9-4027-47BA-9A41-B9A04109C7CE`. Relocated file components use new stable
  GUIDs because changing the installation directory changes their key paths.
  Native tests install a synthetic legacy identity/layout, upgrade, repair,
  reject a downgrade, verify shortcuts/icons and uninstall. This fixture uses
  current binaries and does not establish historical binary behavior.
- macOS: quit and remove the old `PNM Viewer Toolkit.app`, then install
  `RAVEN.app`. Changing the bundle ID creates a distinct LaunchServices identity;
  installing the new bundle alone does not remove the old app. No persistent
  application preference store exists in this checkout to migrate. Keep the
  existing macOS 13.0 minimum, architecture checks and ad-hoc signing; this is
  not Developer ID signing or notarization.

Existing PNM Viewer Toolkit binaries accept only the old asset names, package
identities and fixed URLs, with redirects disabled. Use a manual installer for
the first move to RAVEN. RAVEN's subsequent Ubuntu updates retain paired-package
checks, exact trusted URLs, installed-path eligibility, SHA256, bounded I/O,
authenticated installation and restart. Windows/macOS retain manual installers.

## Repository cutover

The new public repository is [PyPNMApps/raven](https://github.com/PyPNMApps/raven).
It is a separate repository, not a rename of the old project. `product.json`,
maintained download links, README badges and updater test URLs now use the exact
lowercase `PyPNMApps/raven` identity. The existing local checkout keeps its Git
history; its origin points to `https://github.com/PyPNMApps/raven.git`. Cloning
an empty repository and copying files would unnecessarily lose that history.
The local directory remains `PNM-Viewer-Toolkit`.

Source migration preserves the existing master history in the new repository.
Hosted CI and release publication require separate verification. Local build
outputs created before this URL cutover must be rebuilt before distribution.
Historical CI investigation links intentionally point to the old repository.

Before making the old repository private:

1. Commit the reviewed refactor and push master to the new origin, retaining
   the existing commit history. Do not mirror or blanket-push historical tags.
2. Run native CI, rebuild packages against the new identity and verify update,
   asset, checksum and installed lifecycle tests before the first RAVEN release.
3. Publish the new release and document the first manual installation for users
   of the old product. Old binaries cannot follow the new updater identity.
4. Update external links. Making the old repository private breaks anonymous
   access to its old downloads and historical CI links; it is a separate
   user-managed action, not part of local source migration.

Do not mutate old releases, rewrite history or weaken redirect/URL checks.

## Artwork and verification

The supplied PNGs are the original raster artwork. Dark and light-blue are the
primary theme pair; light-white is retained as an alternate. The browser, RF
spectrum, magnifier and raven artwork remain in the banners. Native icons use a
reviewed square extraction of the browser/spectrum/magnifier mark, not squeezed
banner text. See [branding assets](../images/README.md).

The pre-change full local gate passed, including all 153 QML tests and four Qt
adapter tests. Baseline exports were captured for all nine single-capture types.
Post-change evidence is recorded in the local implementation report; hosted
Windows/macOS lifecycle behavior requires native runners. The existing six-host
matrix, CodeQL, installed smoke tests, partial-release rules, immutable public
assets and 50,000,000-byte maximum per installer remain required.

The full file-by-file assessment and local working plan live at
`development/raven-rebrand/assessment.md`, which is ignored and absent from fresh
clones. Historical reports, original bootstrap references and contributor
copyright notices intentionally retain their original names. Home dark/light screenshots were refreshed from the actual RAVEN window. Older
measurement gallery images may retain historic branding until recaptured;
never edit screenshots to invent UI evidence.

## Local validation result

On Ubuntu 22.04 x64, workspace tests, Clippy, 153 QML tests, four Qt adapter
tests, 56 release tests, native-cache/Git/script tests, workflow lint, shell checks,
version synchronization and fixture hygiene pass. The nine CLI exports match
the pre-change JSON baseline. Relocated packaged Qt binaries opened/rendered/
exported all nine installed demo types and matched the CLI output.

The local `raven_0.3.6_amd64.deb` is 32,919,504 bytes; the separate CLI DEB is
733,236 bytes. The GUI payload inventory is 125,619,819 bytes and the CLI
archive payload is 3,756,604 bytes; external system prerequisites remain declared
in the DEB control metadata. Artifact versions, SHA256 and size gates pass. These packages
were extracted for relocation checks; no system installation was performed.
Ubuntu install/upgrade/removal, Windows MSI lifecycle, macOS DMG lifecycle and
hosted CI remain pending native execution. Generated reports and package bytes
are in ignored `development/raven-rebrand/`, not release assets.
