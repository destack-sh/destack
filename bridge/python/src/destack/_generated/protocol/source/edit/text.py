# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class TextChange:
    """One textual edit in an open text buffer."""

    """Optional range to replace, absent for full replacement."""
    range: TextRange | None
    """Replacement text."""
    text: str


def encode_text_change(writer: Writer, value: TextChange) -> None:
    if value.range is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_text_range(writer, value.range)
    writer.write_string(value.text)


def decode_text_change(reader: Reader) -> TextChange:
    field_0 = reader.read_option(lambda: decode_text_range(reader))
    field_1 = reader.read_string()

    return TextChange(
        range=field_0,
        text=field_1,
    )


@dataclass(frozen=True, slots=True)
class TextRange:
    """One text range expressed in UTF-16 positions."""

    """Start position."""
    start: TextPosition
    """End position."""
    end: TextPosition


def encode_text_range(writer: Writer, value: TextRange) -> None:
    encode_text_position(writer, value.start)
    encode_text_position(writer, value.end)


def decode_text_range(reader: Reader) -> TextRange:
    field_0 = decode_text_position(reader)
    field_1 = decode_text_position(reader)

    return TextRange(
        start=field_0,
        end=field_1,
    )


@dataclass(frozen=True, slots=True)
class TextPosition:
    """One zero-based UTF-16 text position."""

    """Zero-based line number."""
    line: int
    """Zero-based UTF-16 column."""
    character: int


def encode_text_position(writer: Writer, value: TextPosition) -> None:
    writer.write_unsigned(value.line)
    writer.write_unsigned(value.character)


def decode_text_position(reader: Reader) -> TextPosition:
    field_0 = reader.read_number()
    field_1 = reader.read_number()

    return TextPosition(
        line=field_0,
        character=field_1,
    )


__all__ = [
    "TextChange",
    "encode_text_change",
    "decode_text_change",
    "TextRange",
    "encode_text_range",
    "decode_text_range",
    "TextPosition",
    "encode_text_position",
    "decode_text_position",
]
