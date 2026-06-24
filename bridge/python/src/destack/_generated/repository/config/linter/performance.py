# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.repository.config.linter.core


@dataclass(frozen=True, slots=True)
class LinterPerformanceOptions:
    """Performance-category linter options."""

    # required Unicode regex flag for `require-unicode-regexp`
    require_unicode_regexp_require_flag: (
        destack._generated.repository.config.linter.core.UnicodeRegexpRequireFlag
    )

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_performance_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterPerformanceOptions:
        """Decode one LinterPerformanceOptions."""
        return decode_linter_performance_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_performance_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterPerformanceOptions:
        """Return one LinterPerformanceOptions from one JSON value."""
        return from_json_linter_performance_options(value)


def encode_linter_performance_options(
    writer: BinaryWriter, value: LinterPerformanceOptions
) -> None:
    """Encode one LinterPerformanceOptions."""
    destack._generated.repository.config.linter.core.encode_unicode_regexp_require_flag(
        writer, value.require_unicode_regexp_require_flag
    )


def decode_linter_performance_options(reader: BinaryReader) -> LinterPerformanceOptions:
    """Decode one LinterPerformanceOptions."""
    require_unicode_regexp_require_flag = destack._generated.repository.config.linter.core.decode_unicode_regexp_require_flag(
        reader
    )

    return LinterPerformanceOptions(
        require_unicode_regexp_require_flag=require_unicode_regexp_require_flag,
    )


def to_json_linter_performance_options(value: LinterPerformanceOptions) -> Json:
    """Return one JSON value for one LinterPerformanceOptions."""
    return {
        "requireUnicodeRegexpRequireFlag": destack._generated.repository.config.linter.core.to_json_unicode_regexp_require_flag(
            value.require_unicode_regexp_require_flag
        ),
    }


def from_json_linter_performance_options(value: Json) -> LinterPerformanceOptions:
    """Return one LinterPerformanceOptions from one JSON value."""
    object_ = json_object(value)

    return LinterPerformanceOptions(
        require_unicode_regexp_require_flag=destack._generated.repository.config.linter.core.from_json_unicode_regexp_require_flag(
            json_field(object_, "requireUnicodeRegexpRequireFlag")
        ),
    )


__all__ = [
    "LinterPerformanceOptions",
    "encode_linter_performance_options",
    "decode_linter_performance_options",
    "to_json_linter_performance_options",
    "from_json_linter_performance_options",
]
