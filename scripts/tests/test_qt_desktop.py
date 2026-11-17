#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Exercise the Linux VM launcher without Qt, Cargo, or a real desktop."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parents[1] / 'qt-desktop.sh'


class QtDesktopLauncherTests(unittest.TestCase):
    """Run copied launchers against isolated executables and SDK directories."""

    def setUp(self):
        """Create a fake checkout whose Cargo executable must never run."""
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.launcher = self.root / 'scripts/qt-desktop.sh'
        self.launcher.parent.mkdir()
        shutil.copyfile(SOURCE, self.launcher)
        self.sdk = self.root / 'Qt SDK'
        qmake = self.sdk / 'bin/qmake'
        qmake.parent.mkdir(parents=True)
        qmake.write_text('#!/bin/sh\nexit 0\n')
        qmake.chmod(0o755)
        cargo = self.sdk / 'bin/cargo'
        cargo.write_text('#!/bin/sh\necho CARGO_MUST_NOT_RUN >&2\nexit 91\n')
        cargo.chmod(0o755)
        self.binary = self.root / 'apps/qt/target/debug/raven'
        self.environment = dict(os.environ, QT_SDK_DIR=str(self.sdk),
                                QT_QPA_PLATFORM='offscreen')

    def launch(self, *arguments):
        """Execute the launcher from outside the fake project and retain output."""
        return subprocess.run(['bash', str(self.launcher), *arguments], cwd='/tmp',
                              env=self.environment, text=True, capture_output=True)

    def test_skip_build_executes_existing_binary_and_preserves_arguments(self):
        """Both accepted forms bypass Cargo and retain paths containing spaces."""
        self.binary.parent.mkdir(parents=True)
        self.binary.write_text('#!' + sys.executable + '\nimport os,sys,json\n'
                               'print(json.dumps([sys.argv[1:], os.environ["LD_LIBRARY_PATH"]]))\n')
        self.binary.chmod(0o755)
        for prefix in (('--skip-build',), ('run', '--skip-build')):
            with self.subTest(prefix=prefix):
                result = self.launch(*prefix, '--ofdm-smoke', 'file with spaces.bin')
                self.assertEqual(result.returncode, 0, result.stderr)
                arguments, libraries = json.loads(result.stdout)
                self.assertEqual(arguments, ['--ofdm-smoke', 'file with spaces.bin'])
                self.assertEqual(libraries.split(os.pathsep)[0], str(self.sdk / 'lib'))

    def test_missing_binary_fails_without_falling_back_to_build(self):
        """Missing outputs give a build instruction rather than launching Cargo."""
        result = self.launch('--skip-build')
        self.assertEqual(result.returncode, 1)
        self.assertIn('qt-desktop.sh build first', result.stderr)
        self.assertNotIn('CARGO_MUST_NOT_RUN', result.stderr)

    def test_build_and_check_reject_skip_build(self):
        """The run-only flag cannot silently skip validation or build commands."""
        for action in ('build', 'check'):
            with self.subTest(action=action):
                result = self.launch(action, '--skip-build')
                self.assertEqual(result.returncode, 2)
                self.assertIn('only when running', result.stderr)


if __name__ == '__main__':
    unittest.main()
