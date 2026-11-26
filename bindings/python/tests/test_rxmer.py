#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Test an installed native wheel against literal anchors, manifests and the real Rust CLI."""
import json
import os
from pathlib import Path
import subprocess
import unittest
from raven_pnm import CmtsUsOfdmaRxMer, ParseFailure

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = Path(os.environ.get('RAVEN_FIXTURE_DIR', ROOT / 'tests/fixtures/synthetic/cmts-upstream-rxmer'))

class RxMerTests(unittest.TestCase):
    def test_bytes_file_and_rust_cli_are_equivalent(self):
        """Compare independent processes and exact literal values, retaining nulls and raw bytes."""
        path = FIXTURES / 'golden.bin'
        capture = CmtsUsOfdmaRxMer.from_file(path)
        self.assertEqual(capture.to_json(), CmtsUsOfdmaRxMer.from_bytes(path.read_bytes()).to_json())
        self.assertEqual(capture.header['capture_time'], 1700000000)
        self.assertEqual(capture.metadata['if_index'], 16909060)
        self.assertEqual([sample.rxmer_db for sample in capture.samples], [0.0, 0.25, 23.75, 63.5, None])
        self.assertEqual(capture.frequency_axis, (20400000, 20425000, 20450000, 20475000, 20500000))
        self.assertEqual(capture.raw_values, bytes([0, 1, 95, 254, 255]))
        self.assertEqual(capture.original_bytes(), path.read_bytes())
        self.assertEqual(capture.statistics['arithmetic_mean_db'], 21.875)
        cli = Path(os.environ.get('RAVEN_TEST_CLI', ROOT / 'target/debug/pnm-decode'))
        result = subprocess.run([str(cli), str(path), '--stdout'], check=True, capture_output=True, text=True)
        self.assertEqual(json.loads(capture.to_json()), json.loads(result.stdout))

    def test_every_manifest_and_typed_errors(self):
        """Verify every synthetic scenario through the actual installed extension."""
        for path in sorted(FIXTURES.glob('*.bin')):
            with self.subTest(path=path.name):
                manifest = json.loads(path.with_suffix('.json').read_text())
                if manifest['expected_result'] == 'success':
                    capture = CmtsUsOfdmaRxMer.from_file(path)
                    self.assertEqual(list(capture.raw_values), manifest['raw_values'])
                    self.assertEqual([sample.rxmer_db for sample in capture.samples], manifest['expected_values_db'])
                else:
                    with self.assertRaises(ParseFailure) as failure:
                        CmtsUsOfdmaRxMer.from_file(path)
                    self.assertEqual(failure.exception.code, manifest['expected_result'])
                    self.assertIsInstance(failure.exception.offset, int)
                    self.assertIsInstance(failure.exception.field, str)

    def test_limits_and_missing_input(self):
        """Keep filesystem and bounded parser failures distinct."""
        with self.assertRaises(OSError):
            CmtsUsOfdmaRxMer.from_file(FIXTURES / 'missing.bin')
        with self.assertRaises(ParseFailure) as failure:
            CmtsUsOfdmaRxMer.from_bytes(b'x' * 70000)
        self.assertEqual(failure.exception.code, 'LimitExceeded')
        self.assertEqual(CmtsUsOfdmaRxMer.from_file(FIXTURES / 'all-unavailable.bin').statistics['valid_count'], 0)

if __name__ == '__main__':
    unittest.main()
