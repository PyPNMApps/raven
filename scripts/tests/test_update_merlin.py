#!/usr/bin/env python3
#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Verify release selection and transactional dependency updates without network."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('update_merlin', ROOT / 'tools/development/update-merlin.py')
updater = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(updater)


class MerlinUpdateTests(unittest.TestCase):
    """Check annotated tags, exact edits and failure restoration."""

    def test_latest_uses_semantic_order_and_peeled_commit(self):
        """Exclude prereleases and prefer annotated-tag commit over tag object."""
        listing = ('old refs/tags/v0.9.0\nobject refs/tags/v0.10.0\n'
                   'commit refs/tags/v0.10.0^{}\nfuture refs/tags/v9.0.0-rc1\n')
        self.assertEqual(updater.select_tag(listing, 'latest'), ('v0.10.0', 'commit'))
        self.assertEqual(updater.select_tag(listing, 'v0.9.0'), ('v0.9.0', 'old'))
        with self.assertRaises(ValueError):
            updater.select_tag(listing, 'v9.0.0-rc1')
        with self.assertRaises(ValueError):
            updater.select_tag('', 'latest')

    def test_dependency_edit_is_scoped(self):
        """Change the selector without losing features or unrelated dependencies."""
        original = ('[dependencies]\nmerlin = { git = "' + updater.REMOTE
                    + '", tag = "v0.4.0", optional = true }\nother = "1"\n')
        expected = original.replace('tag = "v0.4.0"', 'rev = "new"')
        self.assertEqual(updater.update_dependency(original, 'new'), expected)
        with self.assertRaises(ValueError):
            updater.update_dependency(original.replace(updater.REMOTE, 'wrong'), 'new')

    def prepare(self, root):
        """Create disposable dependency inputs and a prior legal snapshot."""
        for relative in ('apps/viewer/Cargo.toml', 'apps/viewer/src/simulation/fec/mod.rs'):
            target = root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((ROOT / relative).read_bytes())
        (root / 'Cargo.lock').write_text('original lock\n')
        legal_root = root / 'LICENSES/merlin'
        legal_root.mkdir(parents=True)
        (legal_root / 'NOTICE').write_text('old notice\n')

    def test_failure_restores_all_original_bytes(self):
        """A failing Cargo operation must not leave half an upgrade in the tree."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.prepare(root)
            before = {path.relative_to(root): path.read_bytes() for path in root.rglob('*') if path.is_file()}

            def failing_cargo(*arguments, **keywords):
                (root / 'Cargo.lock').write_text('partial lock')
                raise subprocess.CalledProcessError(37, arguments[0])

            with patch.object(updater.subprocess, 'run', side_effect=failing_cargo):
                with self.assertRaises(subprocess.CalledProcessError):
                    updater.apply_update(root, 'v0.5.0', 'new', {'NOTICE': b'new', 'LICENSE': b'license'})
            after = {path.relative_to(root): path.read_bytes() for path in root.rglob('*') if path.is_file()}
            self.assertEqual(before, after)

    def test_success_records_selection_and_updates_runtime(self):
        """Successful resolution retains precise pin and exact upstream notices."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.prepare(root)
            with patch.object(updater.subprocess, 'run') as cargo:
                updater.apply_update(root, 'v0.5.0', 'new', {'NOTICE': b'upstream notice'})
            self.assertIn('rev = "new"', (root / 'apps/viewer/Cargo.toml').read_text())
            self.assertIn('MERLIN_REVISION: &str = "new"', (root / 'apps/viewer/src/simulation/fec/mod.rs').read_text())
            self.assertEqual((root / 'LICENSES/merlin/NOTICE').read_bytes(), b'upstream notice')
            self.assertIn('Tag: v0.5.0', (root / 'LICENSES/merlin/SOURCE.txt').read_text())
            self.assertEqual(cargo.call_args.args[0][-4:], ['-p', 'docsis-ofdm-fec', '--precise', 'new'])


if __name__ == '__main__':
    unittest.main()
