#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
"""Optional Python presentation for the authoritative native Rust decoder."""
from dataclasses import dataclass
from functools import cached_property
import json
from os import PathLike
from typing import Any
from ._native import NativeCapture, ParseFailure

@dataclass(frozen=True)
class RxMerSample:
    """One Rust-decoded physical carrier; unavailable readings retain their grid position."""
    subcarrier_index: int
    frequency_hz: int
    raw: int
    rxmer_db: float | None
    valid: bool
    status: str

class CmtsUsOfdmaRxMer:
    """Own a native snapshot; Python formats results and performs no decoding or DSP."""
    def __init__(self, native: NativeCapture):
        """Wrap an already validated immutable Rust snapshot without copying its byte buffer."""
        self._native = native
        self._decoded = json.loads(native.to_json())

    @classmethod
    def from_bytes(cls, data: bytes) -> 'CmtsUsOfdmaRxMer':
        """Decode complete bytes through Rust; raise ParseFailure with field/code/offset."""
        return cls(NativeCapture.from_bytes(data))

    @classmethod
    def from_file(cls, path: str | PathLike[str]) -> 'CmtsUsOfdmaRxMer':
        """Read a bounded regular file through Rust; raise OSError or ParseFailure."""
        return cls(NativeCapture.from_file(path))

    @property
    def header(self) -> dict[str, Any]:
        """Return canonical common-header metadata, with file and protocol versions distinct."""
        return dict(self._decoded['pnm_header'])

    @property
    def metadata(self) -> dict[str, Any]:
        """Return canonical measurement metadata without creating another sample array."""
        return {key: value for key, value in self._decoded.items() if key != 'samples'}

    @cached_property
    def samples(self) -> tuple[RxMerSample, ...]:
        """Present Rust-decoded records as typed values; no frequency or MER math occurs here."""
        return tuple(RxMerSample(**sample) for sample in self._decoded['samples'])

    @property
    def raw_values(self) -> bytes:
        """Return original encoded sample octets, preserving unavailable markers."""
        return self._native.raw_values()

    @property
    def frequency_axis(self) -> tuple[int, ...]:
        """Return Rust-calculated Hz centers, including unavailable carrier positions."""
        return tuple(sample.frequency_hz for sample in self.samples)

    @property
    def statistics(self) -> dict[str, Any]:
        """Return Rust descriptive statistics; no Python averaging is performed."""
        return json.loads(self._native.statistics_json())

    def to_json(self) -> str:
        """Export the exact canonical Rust representation shared with CLI output."""
        return self._native.to_json()

    def original_bytes(self) -> bytes:
        """Return a new bytes object containing the unmodified original capture."""
        return self._native.original_bytes()

__all__ = ['CmtsUsOfdmaRxMer', 'RxMerSample', 'ParseFailure']
