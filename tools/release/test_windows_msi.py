#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""MSI identity and security contracts independent of native Windows tooling."""
from pathlib import Path
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts/release'))
import windows_msi


class WindowsMsiTests(unittest.TestCase):
    """Inspect actual installer authoring with synthetic, non-executable files."""

    def setUp(self):
        """Create a nested image with spaces/ampersands to exercise XML escaping."""
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.payload = self.root / 'payload & spaces'
        self.payload.mkdir()
        for name in ('raven.exe', 'pnm-decode.exe', 'platforms/qwindows.dll'):
            path = self.payload / name
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(b'synthetic fixture')
        self.names = {'w': windows_msi.NAMESPACE}

    def test_machine_scope_upgrade_identity_and_no_executable_actions(self):
        """Keep stable components across versions without app-launch/security overrides."""
        products = []
        for version in ('0.1.0', '0.1.1'):
            source = windows_msi.author(self.payload, version, self.root / (version + '.wxs'))
            products.append(ET.parse(source).find('w:Product', self.names))
        first, second = products
        self.assertEqual(first.get('UpgradeCode'), second.get('UpgradeCode'))
        self.assertEqual(first.find('w:Package', self.names).get('InstallScope'), 'perMachine')
        self.assertEqual(first.find('w:MajorUpgrade', self.names).get('Schedule'), 'afterInstallInitialize')
        self.assertEqual([item.attrib for item in first.findall('.//w:Component', self.names)],
                         [item.attrib for item in second.findall('.//w:Component', self.names)])
        self.assertIsNone(first.find('.//w:CustomAction', self.names))
        self.assertIsNone(first.find('.//w:Environment', self.names))
        shortcut = first.find('.//w:Shortcut[@Id="StartMenuViewer"]', self.names)
        self.assertEqual(shortcut.get('Advertise'), 'yes')
        for installed in first.findall('.//w:File', self.names):
            self.assertTrue(Path(installed.get('Source')).is_file())
        self.assertEqual(len(first.findall('.//w:File', self.names)), 3)

    def test_branded_installer_offers_an_unchecked_desktop_choice(self):
        """Use real branding and a conditioned shortcut without launching the app."""
        source = windows_msi.author(self.payload, '0.2.2', self.root / 'branded.wxs')
        product = ET.parse(source).find('w:Product', self.names)
        icon = product.find(f'w:Icon[@Id="{windows_msi.APPLICATION_ICON_ID}"]', self.names)
        self.assertTrue(Path(icon.get('SourceFile')).is_file())
        self.assertEqual(product.find('w:Property[@Id="ARPPRODUCTICON"]', self.names).get('Value'), windows_msi.APPLICATION_ICON_ID)
        for identifier in ('WixUIDialogBmp', 'WixUIBannerBmp'):
            artwork = product.find(f'w:WixVariable[@Id="{identifier}"]', self.names)
            self.assertTrue(Path(artwork.get('Value')).is_file())
        choice = product.find('w:Property[@Id="CREATE_DESKTOP_SHORTCUT"]', self.names)
        self.assertIsNone(choice.get('Value'))
        checkbox = product.find('.//w:Control[@Id="DesktopChoice"]', self.names)
        self.assertEqual(checkbox.get('Property'), 'CREATE_DESKTOP_SHORTCUT')
        self.assertEqual(checkbox.get('CheckBoxValue'), '1')
        component = product.find('.//w:Component[@Id="DesktopShortcutComponent"]', self.names)
        self.assertEqual(component.find('w:Condition', self.names).text, 'CREATE_DESKTOP_SHORTCUT = "1"')
        self.assertEqual(component.find('w:RegistryValue', self.names).get('KeyPath'), 'yes')
        self.assertEqual(component.find('w:Shortcut', self.names).get('Icon'), windows_msi.APPLICATION_ICON_ID)
        self.assertIsNone(product.find('.//w:CustomAction', self.names))

    def test_advertised_shortcut_uses_embedded_executable_icon(self):
        """Avoid ICE50 while leaving the PE icon as the advertised shortcut source."""
        source = windows_msi.author(self.payload, '0.2.3', self.root / 'icons.wxs')
        product = ET.parse(source).find('w:Product', self.names)
        icons = {icon.get('Id'): icon for icon in product.findall('w:Icon', self.names)}
        for shortcut in product.findall('.//w:Shortcut', self.names):
            if shortcut.get('Advertise') == 'yes':
                self.assertIsNone(shortcut.get('Icon'))
                self.assertIsNone(shortcut.get('IconIndex'))
            else:
                icon = icons[shortcut.get('Icon')]
                self.assertEqual(Path(icon.get('Id')).suffix, '.ico')
                self.assertEqual(Path(icon.get('SourceFile')).suffix, '.ico')

    def test_version_bounds_case_collisions_and_existing_source_rejected(self):
        """MSI must reject invalid identity before compiling or modifying installers."""
        for version in ('256.1.0', '1.256.0', '1.0.65536', '1.2.3.4'):
            with self.assertRaises(ValueError):
                windows_msi.author(self.payload, version, self.root / 'invalid.wxs')
        windows_msi.author(self.payload, '255.255.65535', self.root / 'maximum.wxs')
        with self.assertRaises(FileExistsError):
            windows_msi.author(self.payload, '1.0.0', self.root / 'maximum.wxs')
        # Distinct case spellings can coexist only on a case-sensitive host.
        duplicate = self.payload / 'RAVEN.exe'
        if not duplicate.exists():
            duplicate.write_bytes(b'collision')
            with self.assertRaisesRegex(ValueError, 'collision'):
                windows_msi.author(self.payload, '1.0.0', self.root / 'collision.wxs')

    def test_raven_replaces_legacy_family_without_reusing_moved_component_guids(self):
        """Retain the upgrade family while giving relocated key paths new owners."""
        import shutil
        legacy_payload = self.root / 'legacy-payload'
        shutil.copytree(self.payload, legacy_payload)
        (legacy_payload / 'raven.exe').rename(legacy_payload / 'pnm-viewer.exe')
        old = ET.parse(windows_msi.author(
            legacy_payload, '0.0.0', self.root / 'legacy.wxs', legacy=True)).find('w:Product', self.names)
        new = ET.parse(windows_msi.author(
            self.payload, '0.3.6', self.root / 'raven.wxs')).find('w:Product', self.names)
        self.assertEqual(old.get('Name'), 'PNM Viewer Toolkit')
        self.assertEqual(new.get('Name'), 'RAVEN')
        self.assertEqual(new.get('UpgradeCode'), '54ACD1D9-4027-47BA-9A41-B9A04109C7CE')
        self.assertEqual(old.get('UpgradeCode'), new.get('UpgradeCode'))
        old_guids = {item.get('Guid') for item in old.findall('.//w:Component', self.names)}
        new_guids = {item.get('Guid') for item in new.findall('.//w:Component', self.names)}
        self.assertTrue(old_guids.isdisjoint(new_guids))
        self.assertEqual(new.find('.//w:Directory[@Id="INSTALLFOLDER"]', self.names).get('Name'), 'RAVEN')


if __name__ == '__main__':
    unittest.main()
