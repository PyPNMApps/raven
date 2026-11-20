#!/usr/bin/env python3
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Generate RAVEN shell icons and WiX artwork from the transparent icon master."""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]


def generate(output):
    """Create fresh Linux, Windows and macOS assets from the icon master.

    output must not exist; source images and checked-in assets are never changed.
    Use Pillow from the development prerequisites. No network access is required.
    Return the generated directory; filesystem/image errors propagate.
    """
    output.mkdir(parents=True, exist_ok=False)
    windows = output / 'windows'
    macos = output / 'macos'
    windows.mkdir()
    macos.mkdir()
    sampling = getattr(Image, 'Resampling', Image).LANCZOS
    images = output / 'images'
    images.mkdir()
    with Image.open(ROOT / 'docs/images/raven-icon-source.png') as source:
        if source.width != source.height or source.mode != 'RGBA':
            raise ValueError('Icon master must be square RGBA artwork')
        if any(source.getpixel(corner)[3] for corner in (
                (0, 0), (source.width - 1, 0),
                (0, source.height - 1), (source.width - 1, source.height - 1))):
            raise ValueError('Icon master must have transparent exterior corners')
        icon = source.resize((546, 546), sampling)
    for theme in ('dark', 'light'):
        icon.save(images / f'raven-icon-{theme}.png')
        icon.save(images / f'raven-favicon-{theme}.ico', sizes=[(size, size) for size in (16, 24, 32, 48, 64, 128, 256)])
    with Image.open(images / 'raven-icon-dark.png') as source:
        icon = source.convert('RGBA')
    icon.save(windows / 'raven.ico', sizes=[(size, size) for size in (16, 24, 32, 48, 64, 128, 256)])
    icon.resize((1024, 1024), sampling).save(macos / 'raven.icns')
    dialog = Image.new('RGB', (493, 312), 'white')
    drawing = ImageDraw.Draw(dialog)
    drawing.rectangle((0, 0, 163, 311), fill='#0b1423')
    drawing.rectangle((160, 0, 163, 311), fill='#31d5cb')
    small = icon.resize((138, 138), sampling)
    dialog.paste(small, (11, 75), small)
    dialog.save(windows / 'dialog.bmp')
    banner = Image.new('RGB', (493, 58), 'white')
    ImageDraw.Draw(banner).rectangle((0, 55, 492, 57), fill='#31d5cb')
    small = icon.resize((48, 48), sampling)
    banner.paste(small, (435, 3), small)
    banner.save(windows / 'banner.bmp')
    return output


def main():
    """Generate reviewable assets into --output without replacing existing files."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    arguments = parser.parse_args()
    generate(arguments.output)


if __name__ == '__main__':
    main()
