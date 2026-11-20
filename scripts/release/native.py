#!/usr/bin/env python3
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Native Windows installer and macOS app/DMG deployment."""
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time

from common import PRODUCT, ROOT, binary_path, inventory, notices, qt, run
from samples import DEMO_DIRECTORY, bundle

DMG_CREATE_ATTEMPTS = 3
DMG_RETRY_INITIAL_DELAY_SECONDS = 2


def windows(stage, output, version):
    """Deploy app-local Qt/MSVC and build a native per-machine Windows MSI.

    Bundle the CLI/manual without changing PATH. Windows Installer owns upgrade,
    repair and uninstall; no elevated application/custom action is installed.
    """
    from windows_msi import build
    payload = stage / 'windows-payload'
    payload.mkdir()
    gui = payload / 'raven.exe'
    shutil.copy2(binary_path(gui=True), gui)
    shutil.copy2(binary_path(), payload / 'pnm-decode.exe')
    notices(payload / 'share')
    bundle(payload / DEMO_DIRECTORY)
    run([qt.sdk_path() / 'bin/windeployqt.exe', '--release', '--no-translations',
         '--no-compiler-runtime', '--qmldir', ROOT / 'apps/qt/qml', gui], env=qt.qt_environment())
    if not (payload / 'platforms/qwindows.dll').is_file():
        raise ValueError('Windows native platform/dialog plugin missing')
    redist = os.environ.get('VCToolsRedistDir')
    if not redist:
        raise ValueError('VCToolsRedistDir missing; initialize the MSVC environment')
    candidates = sorted((Path(redist) / 'x64').glob('Microsoft.VC*.CRT'))
    if not candidates:
        raise ValueError('MSVC app-local redistributable DLLs missing')
    for dll in candidates[-1].glob('*.dll'):
        shutil.copy2(dll, payload / dll.name)
    inventory(payload, output / 'windows-installed-files.json')
    build(payload, stage, output / f'RAVEN-{version}-windows-x64.msi', version)


def create_dmg(volume, destination):
    """Create a new DMG, retrying only the observed transient resource-busy error.

    Each of three attempts owns a fresh temporary image beside the destination.
    Preserve the staged app and any existing output; expose only a completed
    image using a no-overwrite hard link. Propagate other failures immediately,
    or the final busy failure after two bounded waits of two and four seconds.
    Native hdiutil errors retain their original stdout/stderr diagnostics.
    """
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    for attempt in range(DMG_CREATE_ATTEMPTS):
        with tempfile.TemporaryDirectory(prefix='.raven-dmg-', dir=destination.parent) as directory:
            image = Path(directory) / 'RAVEN.dmg'
            try:
                result = run(['hdiutil', 'create', '-volname', 'RAVEN', '-srcfolder', volume,
                              '-format', 'UDZO', image], capture_output=True, text=True)
            except subprocess.CalledProcessError as error:
                for diagnostic in (error.stdout, error.stderr):
                    if diagnostic:
                        print(diagnostic, file=sys.stderr, end='')
                if 'resource busy' not in (error.stderr or '').lower() or attempt + 1 == DMG_CREATE_ATTEMPTS:
                    raise
            else:
                if image.is_symlink() or not image.is_file() or image.stat().st_size == 0:
                    raise ValueError('hdiutil did not create a nonempty regular DMG')
                if result.stdout:
                    print(result.stdout, end='')
                if result.stderr:
                    print(result.stderr, file=sys.stderr, end='')
                os.link(image, destination)
                return
        delay_seconds = DMG_RETRY_INITIAL_DELAY_SECONDS * 2 ** attempt
        print(f'DMG resource busy; retry {attempt + 2}/{DMG_CREATE_ATTEMPTS} in {delay_seconds}s.', flush=True)
        time.sleep(delay_seconds)


def macos(stage, output, version, architecture):
    """Deploy/sign an app bundle with Qt's tool and create a compressed DMG.

    Ad-hoc signing is local integrity only, not Apple Developer signing or
    notarization. Include a private CLI/manual and keep separate CLI installs
    independent. Native mounted-image and copied-app checks run separately.
    """
    volume = stage / 'macos-volume'
    app = volume / 'RAVEN.app'
    contents = app / 'Contents'
    binaries = contents / 'MacOS'
    binaries.mkdir(parents=True)
    gui = binaries / 'raven'
    shutil.copy2(binary_path(gui=True), gui)
    resources = contents / 'Resources'
    resources.mkdir()
    shutil.copy2(ROOT / 'packaging/macos/raven.icns', resources / 'raven.icns')
    shutil.copy2(binary_path(), binaries / 'pnm-decode')
    notices(resources / 'share')
    bundle(resources / DEMO_DIRECTORY)
    info = {'CFBundleExecutable': 'raven', 'CFBundleIdentifier': PRODUCT['bundle_id'],
            'CFBundleName': 'RAVEN', 'CFBundleDisplayName': 'RAVEN',
            'CFBundlePackageType': 'APPL', 'CFBundleIconFile': 'raven.icns',
            'CFBundleShortVersionString': version,
            'CFBundleVersion': version, 'NSHighResolutionCapable': True,
            'LSMinimumSystemVersion': os.environ.get('MACOSX_DEPLOYMENT_TARGET', '13.0')}
    (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
    run([qt.sdk_path() / 'bin/macdeployqt', app, f'-qmldir={ROOT / "apps/qt/qml"}',
         '-always-overwrite'], env=qt.qt_environment())
    # The Qt SDK is universal; keep only the offered native architecture before
    # ad-hoc signing so each DMG is genuinely architecture-specific.
    slice_name = 'arm64' if architecture == 'arm64' else 'x86_64'
    for path in app.rglob('*'):
        if not path.is_file() or path.is_symlink():
            continue
        with path.open('rb') as binary:
            magic = binary.read(4)
        if magic not in (bytes.fromhex('cafebabe'), bytes.fromhex('cafebabf'),
                         bytes.fromhex('bebafeca'), bytes.fromhex('bfbafeca')):
            continue
        slices = run(['lipo', '-archs', path], capture_output=True, text=True).stdout.split()
        if slice_name not in slices:
            raise ValueError(f'Deployed runtime lacks {slice_name}: {path}')
        if len(slices) > 1:
            temporary = path.with_name(path.name + '.thin')
            run(['lipo', path, '-thin', slice_name, '-output', temporary])
            shutil.copystat(path, temporary)
            temporary.replace(path)
    run(['codesign', '--force', '--deep', '--sign', '-', app])
    run(['codesign', '--verify', '--deep', '--strict', app])
    inventory(app, output / f'macos-{architecture}-installed-files.json')
    (volume / 'Applications').symlink_to('/Applications')
    create_dmg(volume, output / f'RAVEN-{version}-macos-{architecture}.dmg')
