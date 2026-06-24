# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class LinterSuspiciousOptions:
    """Suspicious-category linter options."""
    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterSuspiciousOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterSuspiciousOptions: ...

def encode_linter_suspicious_options(
    writer: BinaryWriter, value: LinterSuspiciousOptions
) -> None: ...
def decode_linter_suspicious_options(
    reader: BinaryReader,
) -> LinterSuspiciousOptions: ...
def to_json_linter_suspicious_options(value: LinterSuspiciousOptions) -> Json: ...
def from_json_linter_suspicious_options(value: Json) -> LinterSuspiciousOptions: ...

__all__ = [
    "LinterSuspiciousOptions",
    "encode_linter_suspicious_options",
    "decode_linter_suspicious_options",
    "to_json_linter_suspicious_options",
    "from_json_linter_suspicious_options",
]
