# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_object


@dataclass(frozen=True, slots=True)
class LinterSuspiciousOptions:
    """Suspicious-category linter options."""

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_suspicious_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterSuspiciousOptions:
        """Decode one LinterSuspiciousOptions."""
        return decode_linter_suspicious_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_suspicious_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterSuspiciousOptions:
        """Return one LinterSuspiciousOptions from one JSON value."""
        return from_json_linter_suspicious_options(value)


def encode_linter_suspicious_options(
    writer: BinaryWriter, value: LinterSuspiciousOptions
) -> None:
    """Encode one LinterSuspiciousOptions."""
    pass


def decode_linter_suspicious_options(reader: BinaryReader) -> LinterSuspiciousOptions:
    """Decode one LinterSuspiciousOptions."""
    return LinterSuspiciousOptions()


def to_json_linter_suspicious_options(value: LinterSuspiciousOptions) -> Json:
    """Return one JSON value for one LinterSuspiciousOptions."""
    return {}


def from_json_linter_suspicious_options(value: Json) -> LinterSuspiciousOptions:
    """Return one LinterSuspiciousOptions from one JSON value."""
    object_ = json_object(value)
    return LinterSuspiciousOptions()


__all__ = [
    "LinterSuspiciousOptions",
    "encode_linter_suspicious_options",
    "decode_linter_suspicious_options",
    "to_json_linter_suspicious_options",
    "from_json_linter_suspicious_options",
]
