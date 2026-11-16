#
# Copyright 2026 PyPNMApps contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Product migration boundaries preserve package ownership and core identities."""
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts/release'))
import common
import linux


class ProductIdentityTests(unittest.TestCase):
    """Inspect real metadata without installing packages on the host."""

    def test_deb_replacement_is_owned_only_by_the_desktop_package(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ('raven', 'pnm-decode'):
                stage = Path(directory) / name
                stage.mkdir()
                linux.control(stage, name, '0.3.6', 'libc6', 'test')
                metadata = (stage / 'DEBIAN/control').read_text()
                self.assertIn(f'Package: {name}\n', metadata)
                if name == 'raven':
                    self.assertIn('Conflicts: pnm-viewer-toolkit\n', metadata)
                    self.assertIn('Replaces: pnm-viewer-toolkit\n', metadata)
                else:
                    self.assertNotIn('Conflicts:', metadata)
                    self.assertNotIn('Replaces:', metadata)

    def test_repository_cutover_is_explicit_and_public_core_names_are_preserved(self):
        self.assertEqual(common.PRODUCT['display_name'], 'RAVEN')
        self.assertEqual(common.PRODUCT['desktop_package'], 'raven-desktop')
        self.assertEqual(common.PRODUCT['desktop_binary'], 'raven')
        self.assertEqual(common.PRODUCT['repository'], 'PyPNMApps/raven')
        self.assertEqual(common.PRODUCT['target_repository'], 'PyPNMApps/raven')
        self.assertEqual(common.PROJECT_URL, 'https://github.com/' + common.PRODUCT['repository'])
        for file, name in [('crates/core/Cargo.toml', 'pnm'), ('crates/cli/Cargo.toml', 'pnm-decode'),
                           ('apps/viewer/Cargo.toml', 'viewer')]:
            self.assertIn(f'name = "{name}"', (ROOT / file).read_text())
        self.assertIn('uri: "Pnm.Viewer"', (ROOT / 'apps/qt/build.rs').read_text())
