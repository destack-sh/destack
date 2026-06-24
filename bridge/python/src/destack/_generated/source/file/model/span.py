# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class Span:
    """A source range in bytes (in some File)."""

    # the file that the Span belongs to
    file: destack._generated.source.file.model.file.FileId
    # the start position of the Span in bytes (absolute, inclusive)
    start: int
    # the end position of the Span in bytes (absolute, exclusive)
    end: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_span(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Span:
        """Decode one Span."""
        return decode_span(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_span(self)

    @classmethod
    def from_json(cls, value: Json) -> Span:
        """Return one Span from one JSON value."""
        return from_json_span(value)


def encode_span(writer: BinaryWriter, value: Span) -> None:
    """Encode one Span."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.end)


def decode_span(reader: BinaryReader) -> Span:
    """Decode one Span."""
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    start = reader.read_number()
    end = reader.read_number()

    return Span(
        file=file,
        start=start,
        end=end,
    )


def to_json_span(value: Span) -> Json:
    """Return one JSON value for one Span."""
    return {
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "start": value.start,
        "end": value.end,
    }


def from_json_span(value: Json) -> Span:
    """Return one Span from one JSON value."""
    object_ = json_object(value)

    return Span(
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        start=json_int(json_field(object_, "start")),
        end=json_int(json_field(object_, "end")),
    )


__all__ = [
    "Span",
    "encode_span",
    "decode_span",
    "to_json_span",
    "from_json_span",
]
