# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
)

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class Path:
    """A Path is a sequence of segments."""

    # the segments of the path
    segments: Sequence[destack._generated.core.string.StringId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_path(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Path:
        """Decode one Path."""
        return decode_path(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_path(self)

    @classmethod
    def from_json(cls, value: Json) -> Path:
        """Return one Path from one JSON value."""
        return from_json_path(value)


def encode_path(writer: BinaryWriter, value: Path) -> None:
    """Encode one Path."""
    writer.write_unsigned(len(value.segments))
    for item_value_segments_0 in value.segments:
        destack._generated.core.string.encode_string_id(writer, item_value_segments_0)


def decode_path(reader: BinaryReader) -> Path:
    """Decode one Path."""
    segments = [
        destack._generated.core.string.decode_string_id(reader)
        for _ in range(reader.read_number())
    ]

    return Path(
        segments=segments,
    )


def to_json_path(value: Path) -> Json:
    """Return one JSON value for one Path."""
    return {
        "segments": [
            destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.segments
        ],
    }


def from_json_path(value: Json) -> Path:
    """Return one Path from one JSON value."""
    object_ = json_object(value)

    return Path(
        segments=[
            destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "segments"))
        ],
    )


__all__ = [
    "Path",
    "encode_path",
    "decode_path",
    "to_json_path",
    "from_json_path",
]
