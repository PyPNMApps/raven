#
# Copyright 2026 PNM Viewer contributors
# SPDX-License-Identifier: Apache-2.0
#
from os import PathLike

class ParseFailure(Exception):
    code: str
    field: str
    offset: int

class NativeCapture:
    @staticmethod
    def from_bytes(data: bytes) -> NativeCapture: ...
    @staticmethod
    def from_file(path: str | PathLike[str]) -> NativeCapture: ...
    def to_json(self) -> str: ...
    def statistics_json(self) -> str: ...
    def raw_values(self) -> bytes: ...
    def original_bytes(self) -> bytes: ...
