# RAVEN branding assets

| Role | Dark | Light |
| --- | --- | --- |
| Primary banner | `raven-logo-dark.png` | `raven-logo-light.png` |
| Application icon | `raven-icon-dark.png` | `raven-icon-light.png` |
| Favicon | `raven-favicon-dark.ico` | `raven-favicon-light.ico` |

`raven-logo-light-white.png` preserves the supplied white alternate. The primary
light banner is the supplied light-blue variant. These are PNG originals, not
vector artwork. README and embedded Qt resources choose light/dark automatically.

`tools/branding/generate.py --output NEW_DIRECTORY` regenerates theme icons,
Windows ICO (16/24/32/48/64/128/256), macOS ICNS and WiX BMPs into a new directory.
It extracts the original browser/spectrum/magnifier region at `(64,280,610,780)`
from the approved 1536×1024 primary banners. Review icons at small sizes before
copying generated `images/`, `windows/` and `macos/` files into their checked-in
locations. Packaging uses `packaging/windows/raven.ico` and
`packaging/macos/raven.icns`; banners and native icons have separate purposes.

The source ZIP is retained locally in ignored `development/raven-rebrand/`.
Application screenshots are captured from the actual Qt window; see
[the screenshot guide](../development/screenshots.md).

The sidebar blends the displayed logo edges into the active theme in QML. Fade
bounds follow `Image.paintedWidth` and `paintedHeight` so aspect-fit padding cannot
leave a rectangular seam. Verify both dark and light themes after changing logo
layout; preserve the original README PNGs and artwork lettering.
