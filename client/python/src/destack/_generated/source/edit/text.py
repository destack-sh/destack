# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)


@dataclass(frozen=True, slots=True)
class TextChange:
    """One textual edit in an open text buffer."""

    # optional range to replace, absent for full replacement
    range: TextRange | None
    # replacement text
    text: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_text_change(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TextChange:
        """Decode one TextChange."""
        return decode_text_change(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_text_change(self)

    @classmethod
    def from_json(cls, value: Json) -> TextChange:
        """Return one TextChange from one JSON value."""
        return from_json_text_change(value)


def encode_text_change(writer: BinaryWriter, value: TextChange) -> None:
    """Encode one TextChange."""
    if value.range is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_text_range(writer, value.range)
    writer.write_string(value.text)


def decode_text_change(reader: BinaryReader) -> TextChange:
    """Decode one TextChange."""
    range_ = reader.read_option(lambda: decode_text_range(reader))
    text = reader.read_string()

    return TextChange(
        range=range_,
        text=text,
    )


def to_json_text_change(value: TextChange) -> Json:
    """Return one JSON value for one TextChange."""
    return {
        **({} if value.range is None else {"range": to_json_text_range(value.range)}),
        "text": value.text,
    }


def from_json_text_change(value: Json) -> TextChange:
    """Return one TextChange from one JSON value."""
    object_ = json_object(value)

    return TextChange(
        range=json_optional(
            object_, "range", lambda value: from_json_text_range(value)
        ),
        text=json_string(json_field(object_, "text")),
    )


@dataclass(frozen=True, slots=True)
class TextRange:
    """One text range expressed in UTF-16 positions."""

    # start position
    start: TextPosition
    # end position
    end: TextPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_text_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TextRange:
        """Decode one TextRange."""
        return decode_text_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_text_range(self)

    @classmethod
    def from_json(cls, value: Json) -> TextRange:
        """Return one TextRange from one JSON value."""
        return from_json_text_range(value)


def encode_text_range(writer: BinaryWriter, value: TextRange) -> None:
    """Encode one TextRange."""
    encode_text_position(writer, value.start)
    encode_text_position(writer, value.end)


def decode_text_range(reader: BinaryReader) -> TextRange:
    """Decode one TextRange."""
    start = decode_text_position(reader)
    end = decode_text_position(reader)

    return TextRange(
        start=start,
        end=end,
    )


def to_json_text_range(value: TextRange) -> Json:
    """Return one JSON value for one TextRange."""
    return {
        "start": to_json_text_position(value.start),
        "end": to_json_text_position(value.end),
    }


def from_json_text_range(value: Json) -> TextRange:
    """Return one TextRange from one JSON value."""
    object_ = json_object(value)

    return TextRange(
        start=from_json_text_position(json_field(object_, "start")),
        end=from_json_text_position(json_field(object_, "end")),
    )


@dataclass(frozen=True, slots=True)
class TextPosition:
    """One zero-based UTF-16 text position."""

    # zero-based line number
    line: int
    # zero-based UTF-16 column
    character: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_text_position(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TextPosition:
        """Decode one TextPosition."""
        return decode_text_position(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_text_position(self)

    @classmethod
    def from_json(cls, value: Json) -> TextPosition:
        """Return one TextPosition from one JSON value."""
        return from_json_text_position(value)


def encode_text_position(writer: BinaryWriter, value: TextPosition) -> None:
    """Encode one TextPosition."""
    writer.write_unsigned(value.line)
    writer.write_unsigned(value.character)


def decode_text_position(reader: BinaryReader) -> TextPosition:
    """Decode one TextPosition."""
    line = reader.read_number()
    character = reader.read_number()

    return TextPosition(
        line=line,
        character=character,
    )


def to_json_text_position(value: TextPosition) -> Json:
    """Return one JSON value for one TextPosition."""
    return {
        "line": value.line,
        "character": value.character,
    }


def from_json_text_position(value: Json) -> TextPosition:
    """Return one TextPosition from one JSON value."""
    object_ = json_object(value)

    return TextPosition(
        line=json_int(json_field(object_, "line")),
        character=json_int(json_field(object_, "character")),
    )


__all__ = [
    "TextChange",
    "encode_text_change",
    "decode_text_change",
    "to_json_text_change",
    "from_json_text_change",
    "TextRange",
    "encode_text_range",
    "decode_text_range",
    "to_json_text_range",
    "from_json_text_range",
    "TextPosition",
    "encode_text_position",
    "decode_text_position",
    "to_json_text_position",
    "from_json_text_position",
]
