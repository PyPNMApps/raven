#!/usr/bin/env python3
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Test native package installation, rendered JSON export, and removal in CI."""
import argparse
import hashlib
import json
import os
import plistlib
import uuid
from pathlib import Path
import shutil
import subprocess
import tempfile
import tarfile
import zipfile

from common import PRODUCT, PROJECT_URL, native_target, run
from smoke import smoke


def one(directory, pattern):
    """Require exactly one artifact matching pattern; reject missing/extra files."""
    paths = list(directory.glob(pattern))
    if len(paths) != 1:
        raise ValueError(f'Expected one {pattern}, found {len(paths)}')
    return paths[0].resolve()


def linux(packages):
    """Install both DEBs, exercise GUI/CLI, remove UI, then remove CLI.

    Intended only for disposable CI hosts with explicit --ci-disposable-host.
    Require same artifact checksums in the workflow before invoking this script.
    """
    gui = one(packages, 'raven_*.deb')
    cli = one(packages, 'pnm-decode_*.deb')
    installed_gui = Path('/opt/raven/raven')
    try:
        linux_legacy_migration(cli, gui)
        run(['sudo', 'apt-get', 'install', '-y', cli, gui])
        if Path('/opt/pnm-viewer-toolkit/pnm-viewer-qt').exists() or Path('/usr/bin/pnm-viewer').exists():
            raise ValueError('Legacy viewer files survived the RAVEN package transition')
        desktop_entry = Path('/usr/share/applications/raven.desktop')
        if not desktop_entry.is_file():
            raise ValueError('Desktop integration missing')
        launcher = desktop_entry.read_text(encoding='utf-8').splitlines()
        for field in ('Exec=raven', 'Icon=raven',
                      'StartupWMClass=raven'):
            if launcher.count(field) != 1:
                raise ValueError(f'Desktop integration missing or duplicated: {field}')
        smoke(installed_gui, Path('/usr/bin/pnm-decode'))
        linux_upgrade(cli, gui, installed_gui)
    finally:
        run(['sudo', 'apt-get', 'remove', '-y', 'raven'])
        if installed_gui.exists() or Path('/usr/bin/raven').exists():
            raise ValueError('Viewer removal left installed executable')
        # Separate CLI ownership must survive removing the viewer.
        run(['/usr/bin/pnm-decode', '--version'])
        run(['sudo', 'apt-get', 'remove', '-y', 'pnm-decode'])
        if Path('/usr/bin/pnm-decode').exists():
            raise ValueError('CLI removal left installed executable')


def linux_legacy_migration(cli, gui):
    """Install a synthetic legacy layout before testing the new DEB replacement.

    Use this build's bytes under the old package/executable/desktop identities,
    not a claim of historical application behavior. Run only through the guarded
    disposable-host entry point. APT must remove the conflicting legacy package
    when the caller installs RAVEN; no maintainer script deletes foreign files.
    """
    with tempfile.TemporaryDirectory(prefix='raven-legacy-deb-') as directory:
        workspace = Path(directory)
        stage = workspace / 'legacy'
        run(['dpkg-deb', '-R', gui, stage])
        control = stage / 'DEBIAN/control'
        lines = control.read_text().splitlines()
        lines = [line for line in lines if not line.startswith(('Conflicts:', 'Replaces:'))]
        lines = ['Package: pnm-viewer-toolkit' if line.startswith('Package:') else
                 line + '~legacy-test' if line.startswith('Version:') else line for line in lines]
        control.write_text('\n'.join(lines) + '\n')
        payload = stage / 'opt/raven'
        payload.rename(stage / 'opt/pnm-viewer-toolkit')
        (stage / 'opt/pnm-viewer-toolkit/raven').rename(stage / 'opt/pnm-viewer-toolkit/pnm-viewer-qt')
        (stage / 'usr/bin/raven').unlink()
        (stage / 'usr/bin/pnm-viewer').symlink_to('/opt/pnm-viewer-toolkit/pnm-viewer-qt')
        desktop = stage / 'usr/share/applications/raven.desktop'
        legacy_desktop = desktop.with_name('pnm-viewer-toolkit.desktop')
        legacy_desktop.write_text(desktop.read_text().replace('Name=RAVEN', 'Name=PNM Viewer Toolkit')
                                  .replace('Exec=raven', 'Exec=pnm-viewer')
                                  .replace('Icon=raven', 'Icon=pnm-viewer-toolkit')
                                  .replace('StartupWMClass=raven', 'StartupWMClass=pnm-viewer-toolkit'))
        desktop.unlink()
        icon = stage / 'usr/share/icons/hicolor/256x256/apps/raven.png'
        icon.rename(icon.with_name('pnm-viewer-toolkit.png'))
        output = workspace / 'legacy.deb'
        run(['dpkg-deb', '--root-owner-group', '-Znone', '--build', stage, output])
        run(['sudo', 'apt-get', 'install', '-y', cli, output])


def linux_upgrade(cli, gui, installed_gui):
    """Exercise the real root installer from a synthetic older control version.

    Runs only inside the existing disposable-host lifecycle gate. The baseline
    uses this build's payload with an older DEB control version, not a claimed
    real historical binary. Test both-package upgrade and same-version refusal.
    No desktop password prompt is automated; VM Polkit testing is separate.
    """
    version = json.loads((cli.parent / 'release-manifest.json').read_text())['version']
    with tempfile.TemporaryDirectory(prefix='pnm-upgrade-fixture-') as workspace:
        root = Path(workspace)
        older = []
        for package in (cli, gui):
            stage = root / package.stem
            run(['dpkg-deb', '-R', package, stage])
            control = stage / 'DEBIAN/control'
            control.write_text(control.read_text().replace(version, version + '~upgrade-test'))
            output = root / package.name
            run(['dpkg-deb', '--root-owner-group', '-Znone', '--build', stage, output])
            older.append(output)
        run(['sudo', 'apt-get', 'install', '-y', '--allow-downgrades', *older])
        with tempfile.TemporaryDirectory(prefix='pnm-viewer-update-') as directory:
            downloads = Path(directory)
            plan = {'version': version, 'packages': []}
            for package in (cli, gui):
                shutil.copy2(package, downloads / package.name)
                plan['packages'].append({
                    'name': package.name, 'size': package.stat().st_size,
                    'sha256': hashlib.sha256(package.read_bytes()).hexdigest(),
                    'url': f'{PROJECT_URL}/releases/download/v{version}/{package.name}'})
            (downloads / 'plan.json').write_text(json.dumps(plan))
            command = ['sudo', 'env', f'PKEXEC_UID={os.getuid()}', installed_gui,
                       '--install-update', downloads]
            # Reject tampered bytes through the real privileged entry point.
            first = downloads / cli.name
            original = first.read_bytes()
            first.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
            rejected = subprocess.run([str(item) for item in command], capture_output=True, text=True)
            if rejected.returncode == 0 or 'SHA-256 verification failed' not in rejected.stderr:
                raise ValueError('Updater did not reject a corrupted package by hash')
            first.write_bytes(original)
            run(command)
            for package in ('pnm-decode', 'raven'):
                installed = run(['dpkg-query', '-W', '-f=${Version}', package],
                                capture_output=True, text=True).stdout
                if installed != version:
                    raise ValueError('Paired upgrade did not install the expected version')
            rejected = subprocess.run([str(item) for item in command], capture_output=True, text=True)
            if rejected.returncode == 0 or 'newer than both installed packages' not in rejected.stderr:
                raise ValueError('Updater did not reject a same-version installation')
            smoke(installed_gui, Path('/usr/bin/pnm-decode'))


def windows(packages, payload):
    """Exercise MSI install/major-upgrade/downgrade refusal/removal on disposable CI.

    payload is the freshly packaged image used to author a synthetic 0.0.0
    baseline with the current binaries. This verifies installer transitions,
    not a historical application's behavior or interactive Windows 11 prompts.
    """
    import winreg
    from windows_msi import build
    registry_path = r'Software\Microsoft\Windows\CurrentVersion\Uninstall'

    def registrations():
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, registry_path, 0,
                            winreg.KEY_READ | winreg.KEY_WOW64_64KEY) as registry:
            found = []
            for index in range(winreg.QueryInfoKey(registry)[0]):
                name = winreg.EnumKey(registry, index)
                with winreg.OpenKey(registry, name) as product:
                    try:
                        title = winreg.QueryValueEx(product, 'DisplayName')[0]
                    except FileNotFoundError:
                        continue
                    if title in ('RAVEN', 'PNM Viewer Toolkit'):
                        uuid.UUID(name.strip('{}'))
                        found.append((name, winreg.QueryValueEx(product, 'DisplayVersion')[0]))
            return found

    def execute(arguments, logfile, success=True):
        result = subprocess.run(['msiexec.exe', *map(str, arguments), '/qn', '/norestart',
                                 '/L*V', str(logfile)], cwd=packages, capture_output=True,
                                text=True, timeout=180)
        accepted = result.returncode in (0, 3010)
        if not success:
            accepted = result.returncode == 1603
        if not accepted:
            if logfile.is_file():
                print(logfile.read_text(encoding='utf-16', errors='replace'), flush=True)
            raise ValueError(f'Windows Installer exit {result.returncode}; log: {logfile}')

    if registrations():
        raise ValueError('Existing viewer MSI registration; refusing lifecycle test')
    desktop_shortcut = Path(os.environ['PUBLIC']) / 'Desktop/RAVEN.lnk'
    start_shortcut = Path(os.environ['ProgramData']) / 'Microsoft/Windows/Start Menu/Programs/RAVEN/RAVEN.lnk'
    if desktop_shortcut.exists():
        raise ValueError('Existing viewer desktop shortcut; refusing lifecycle test')
    if payload is None or not payload.is_dir():
        raise ValueError('Windows lifecycle requires the fresh packaging payload')
    installer = one(packages, '*-windows-x64.msi')
    version = json.loads((packages / 'release-manifest.json').read_text())['version']
    logs = Path(__file__).resolve().parents[2] / 'local/windows-installer-logs'
    logs.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='pnm-msi-') as directory:
        work = Path(directory)
        prefix = work / 'viewer'
        baseline = work / 'baseline.msi'
        legacy_payload = work / 'legacy-payload'
        shutil.copytree(payload, legacy_payload)
        (legacy_payload / 'raven.exe').rename(legacy_payload / 'pnm-viewer.exe')
        legacy_stage = work / 'legacy-stage'
        legacy_stage.mkdir()
        build(legacy_payload, legacy_stage, baseline, '0.0.0', legacy=True)
        legacy_prefix = work / 'legacy-install'

        try:
            execute(['/i', baseline, f'INSTALLFOLDER={legacy_prefix}', 'CREATE_DESKTOP_SHORTCUT=0'], logs / 'baseline.log')
            if desktop_shortcut.exists():
                raise ValueError('Desktop shortcut created despite opt-out')
            if len(registrations()) != 1 or registrations()[0][1] != '0.0.0':
                raise ValueError('MSI baseline registration missing')
            execute(['/i', installer, f'INSTALLFOLDER={prefix}', 'CREATE_DESKTOP_SHORTCUT=1'], logs / 'upgrade.log')
            if (legacy_prefix / 'pnm-viewer.exe').exists():
                raise ValueError('Legacy viewer executable survived the upgrade')
            if not start_shortcut.is_file():
                raise ValueError('Start menu shortcut missing')
            if not desktop_shortcut.is_file():
                raise ValueError('Selected desktop shortcut missing')
            if len(registrations()) != 1 or registrations()[0][1] != version:
                raise ValueError('MSI major upgrade did not replace baseline registration')
            smoke(prefix / 'raven.exe', prefix / 'pnm-decode.exe')
            # Resource ID 1 is authored by apps/qt/build.rs from the branded ICO.
            import ctypes
            from ctypes import wintypes
            library = ctypes.WinDLL('kernel32', use_last_error=True)
            library.LoadLibraryExW.argtypes = (wintypes.LPCWSTR, wintypes.HANDLE, wintypes.DWORD)
            library.LoadLibraryExW.restype = wintypes.HANDLE
            library.FindResourceW.argtypes = (wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p)
            library.FindResourceW.restype = wintypes.HANDLE
            library.FreeLibrary.argtypes = (wintypes.HANDLE,)
            library.FreeLibrary.restype = wintypes.BOOL
            load_as_data_file = 0x02
            load_as_image_resource = 0x20
            main_icon_id = 1
            group_icon_resource_type = 14
            module = library.LoadLibraryExW(str(prefix / 'raven.exe'), None,
                                           load_as_data_file | load_as_image_resource)
            if not module:
                raise ValueError('Cannot inspect installed Windows icon resources')
            try:
                if not library.FindResourceW(module, ctypes.c_void_p(main_icon_id),
                                             ctypes.c_void_p(group_icon_resource_type)):
                    raise ValueError('Installed executable lacks the branded icon resource')
            finally:
                library.FreeLibrary(module)
            if not (prefix / 'share/pnm-decode.1').is_file():
                raise ValueError('Bundled CLI manual missing')
            execute(['/fa', installer], logs / 'repair.log')
            if not desktop_shortcut.is_file() or not start_shortcut.is_file():
                raise ValueError('Repair did not preserve requested shortcuts')
            smoke(prefix / 'raven.exe', prefix / 'pnm-decode.exe')
            execute(['/i', baseline, f'INSTALLFOLDER={legacy_prefix}'], logs / 'downgrade.log', success=False)
            if len(registrations()) != 1 or registrations()[0][1] != version:
                raise ValueError('Rejected downgrade changed installed version')
        finally:
            for product_code, _version in registrations():
                execute(['/x', product_code], logs / 'uninstall.log')
        leftovers = [str(path) for path in (prefix / 'raven.exe', prefix / 'pnm-decode.exe',
                                           desktop_shortcut, start_shortcut) if path.exists()]
        remaining_products = registrations()
        if remaining_products or leftovers:
            raise ValueError(f'MSI uninstall left files {leftovers} or products {remaining_products}')


def macos(packages, architecture):
    """Mount a DMG read-only and test a copied app without development Qt paths.

    Copy into a fresh temporary Applications directory, then remove it. No
    developer signing, notarization, global installation or LaunchServices change.
    """
    dmg = one(packages, f'*-macos-{architecture}.dmg')
    with tempfile.TemporaryDirectory(prefix='pnm-dmg-') as directory:
        mount = Path(directory) / 'mounted'
        mount.mkdir()
        run(['hdiutil', 'attach', '-readonly', '-nobrowse', '-mountpoint', mount, dmg])
        app = Path(directory) / 'Applications/RAVEN.app'
        try:
            run(['ditto', mount / 'RAVEN.app', app])
        finally:
            run(['hdiutil', 'detach', mount])
        run(['codesign', '--verify', '--deep', '--strict', app])
        with (app / 'Contents/Info.plist').open('rb') as stream:
            info = plistlib.load(stream)
        if info.get('CFBundleIdentifier') != PRODUCT['bundle_id'] or info.get('CFBundleExecutable') != 'raven' or info.get('CFBundleName') != 'RAVEN' or info.get('CFBundleDisplayName') != 'RAVEN':
            raise ValueError('macOS product identity mismatch')
        icon = app / 'Contents/Resources' / info.get('CFBundleIconFile', '')
        if not icon.is_file() or icon.read_bytes()[:4] != b'icns':
            raise ValueError('Bundled macOS application icon missing')
        smoke(app / 'Contents/MacOS/raven', app / 'Contents/MacOS/pnm-decode')
        if not (app / 'Contents/Resources/share/pnm-decode.1').is_file():
            raise ValueError('Bundled CLI manual missing')
        shutil.rmtree(app)
        if app.exists():
            raise ValueError('App removal failed')


def standalone_cli(packages):
    """Extract the standalone archive safely and test its decoder/manual.

    Reject traversal and links even though CI produced the artifact. Extraction
    uses a temporary prefix and never modifies a globally installed CLI.
    """
    archives = list(packages.glob('pnm-decode-*.zip')) + list(packages.glob('pnm-decode-*.tar.gz'))
    if len(archives) != 1:
        raise ValueError('Expected one standalone CLI archive')
    with tempfile.TemporaryDirectory(prefix='pnm-cli-archive-') as directory:
        root = Path(directory)
        archive = archives[0]
        if archive.suffix == '.zip':
            with zipfile.ZipFile(archive) as source:
                members = source.infolist()
                if sum(member.file_size for member in members) > 200_000_000:
                    raise ValueError('CLI archive expansion exceeds limit')
                entries = [(member.filename, source.read(member)) for member in members if not member.is_dir()]
        else:
            with tarfile.open(archive) as source:
                entries = []
                expanded_bytes = 0
                for member in source:
                    if member.isdir():
                        continue
                    if not member.isfile():
                        raise ValueError('Unexpected link in CLI archive')
                    expanded_bytes += member.size
                    if expanded_bytes > 200_000_000:
                        raise ValueError('CLI archive expansion exceeds limit')
                    entries.append((member.name, source.extractfile(member).read()))
        for name, data in entries:
            relative = Path(name)
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError('Unsafe CLI archive path')
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            if target.name == 'pnm-decode':
                target.chmod(0o755)
        suffix = '.exe' if os.name == 'nt' else ''
        binary = one(root, '*/pnm-decode' + suffix)
        if not (binary.parent / 'share/pnm-decode.1').is_file():
            raise ValueError('Standalone CLI manual missing')
        output = root / 'rxmer.json'
        from common import ROOT
        from smoke import clean_environment
        environment = clean_environment()
        run([binary, '--version'], env=environment)
        run([binary, ROOT / 'tests/fixtures/pypnm/tests/files/rxmer.bin', '--output', output], env=environment)
        import json
        if not json.loads(output.read_text(encoding="utf-8")):
            raise ValueError('Standalone CLI produced empty JSON')


def main():
    """Require a disposable GitHub runner before system-changing package tests."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--packages', required=True, type=Path)
    parser.add_argument('--windows-payload', type=Path, help='Fresh image for synthetic MSI upgrade tests')
    parser.add_argument('--ci-disposable-host', action='store_true')
    args = parser.parse_args()
    if not args.ci_disposable_host or os.environ.get('GITHUB_ACTIONS') != 'true':
        parser.error('Installation requires a disposable GitHub Actions runner')
    standalone_cli(args.packages)
    system, architecture = native_target()
    if system == 'Linux':
        linux(args.packages)
    elif system == 'Windows':
        windows(args.packages.resolve(), args.windows_payload.resolve() if args.windows_payload else None)
    else:
        macos(args.packages, architecture)


if __name__ == '__main__':
    main()
