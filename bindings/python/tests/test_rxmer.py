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
from raven_pnm._native import NativeCapture

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

    def test_native_ownership_and_structured_errors_match_rust_contract(self):
        """Keep owned bytes after temporary input expires and inspect error attributes directly."""
        path = FIXTURES / 'golden.bin'
        native = NativeCapture.from_bytes(path.read_bytes())
        capture = CmtsUsOfdmaRxMer.from_bytes(path.read_bytes())
        cli = Path(os.environ.get('RAVEN_TEST_CLI', ROOT / 'target/debug/pnm-decode'))
        result = subprocess.run([str(cli), str(path), '--stdout'], check=True, capture_output=True, text=True)
        expected = json.loads(result.stdout)
        self.assertEqual(json.loads(native.to_json()), expected)
        self.assertEqual(capture.metadata, {key: value for key, value in expected.items() if key != 'samples'})
        self.assertEqual(native.original_bytes(), path.read_bytes())
        self.assertEqual(capture.original_bytes(), path.read_bytes())
        self.assertEqual(native.raw_values(), bytes([0, 1, 95, 254, 255]))
        self.assertEqual(capture.samples[-1].rxmer_db, None)
        self.assertEqual(json.loads(native.statistics_json())['arithmetic_mean_db'], 21.875)

        for prefix in (b'\xff\0', b'c\n\0', b'\xc2\x85\0'):
            malformed = bytearray(path.read_bytes())
            malformed[14:14 + len(prefix)] = prefix
            for decoder in (NativeCapture.from_bytes, CmtsUsOfdmaRxMer.from_bytes):
                with self.subTest(prefix=prefix, decoder=decoder):
                    with self.assertRaises(ParseFailure) as failure:
                        decoder(bytes(malformed))
                    self.assertEqual(failure.exception.code, 'InvalidField')
                    self.assertEqual(failure.exception.field, 'ccap_id_encoding')
                    self.assertEqual(failure.exception.offset, 14)

    def test_limits_and_missing_input(self):
        """Keep filesystem and bounded parser failures distinct."""
        with self.assertRaises(OSError):
            CmtsUsOfdmaRxMer.from_file(FIXTURES / 'missing.bin')
        with self.assertRaises(ParseFailure) as failure:
            CmtsUsOfdmaRxMer.from_bytes(b'x' * 70000)
        self.assertEqual(failure.exception.code, 'LimitExceeded')
        self.assertEqual(CmtsUsOfdmaRxMer.from_file(FIXTURES / 'all-unavailable.bin').statistics['valid_count'], 0)
        for name in ('all-unavailable', 'empty'):
            summary = CmtsUsOfdmaRxMer.from_file(FIXTURES / (name + '.bin')).statistics
            self.assertIsNone(summary['minimum_db'])
            self.assertIsNone(summary['arithmetic_mean_db'])
            self.assertIsNone(summary['maximum_db'])

if __name__ == '__main__':
    unittest.main()
