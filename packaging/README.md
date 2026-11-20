# Native branding

These assets derive from `docs/images/raven-icon-source.png`, the transparent
monitor/spectrum/magnifier master. The surrounding desktop remains visible
outside the artwork; the navy screen interior is retained. PNG, ICO and ICNS
share the same silhouette. WiX BMPs composite it on the installer background.

- Windows: multi-size ICO (16–256 px), WiX dialog BMP (493×312), banner BMP (493×58).
- macOS: ICNS with standard icon representations through 1024 px.

The Windows executable embeds the ICO with the Windows SDK resource compiler.
WiX embeds it for Start/desktop shortcuts and Installed apps. macOS references
the ICNS from `CFBundleIconFile`. Installer artwork retains standard OS controls.

To regenerate into a **new** review directory using development prerequisites:

```bash
python3 tools/branding/generate.py --output local/branding-preview
```

Inspect the images before replacing the corresponding checked-in assets.
Pillow is a development-only conversion tool; packaged applications and native
release builders use the committed assets and do not need it at runtime.

RAVEN sources and regeneration instructions: [branding assets](../docs/images/README.md).
