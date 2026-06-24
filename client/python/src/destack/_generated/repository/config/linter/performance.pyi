# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.linter.core

@dataclass(frozen=True, slots=True)
class LinterPerformanceOptions:
    """Performance-category linter options."""

    # required Unicode regex flag for `require-unicode-regexp`
    require_unicode_regexp_require_flag: (
        destack._generated.repository.config.linter.core.UnicodeRegexpRequireFlag
    )

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterPerformanceOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterPerformanceOptions: ...

def encode_linter_performance_options(
    writer: BinaryWriter, value: LinterPerformanceOptions
) -> None: ...
def decode_linter_performance_options(
    reader: BinaryReader,
) -> LinterPerformanceOptions: ...
def to_json_linter_performance_options(value: LinterPerformanceOptions) -> Json: ...
def from_json_linter_performance_options(value: Json) -> LinterPerformanceOptions: ...

__all__ = [
    "LinterPerformanceOptions",
    "encode_linter_performance_options",
    "decode_linter_performance_options",
    "to_json_linter_performance_options",
    "from_json_linter_performance_options",
]
