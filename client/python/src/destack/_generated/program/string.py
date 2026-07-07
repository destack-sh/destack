# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.core.section


@dataclass(frozen=True, slots=True)
class StringTable:
    """Section-backed program string table."""

    # string entries sorted by stable string id
    entries: destack._generated.core.section.SectionSlice
    # concatenated UTF-8 string bytes
    bytes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_string_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StringTable:
        """Decode one StringTable."""
        return decode_string_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_string_table(self)

    @classmethod
    def from_json(cls, value: Json) -> StringTable:
        """Return one StringTable from one JSON value."""
        return from_json_string_table(value)


def encode_string_table(writer: BinaryWriter, value: StringTable) -> None:
    """Encode one StringTable."""
    destack._generated.core.section.encode_section_slice(writer, value.entries)
    destack._generated.core.section.encode_section_slice(writer, value.bytes)


def decode_string_table(reader: BinaryReader) -> StringTable:
    """Decode one StringTable."""
    entries = destack._generated.core.section.decode_section_slice(reader)
    bytes = destack._generated.core.section.decode_section_slice(reader)

    return StringTable(
        entries=entries,
        bytes=bytes,
    )


def to_json_string_table(value: StringTable) -> Json:
    """Return one JSON value for one StringTable."""
    return {
        "entries": destack._generated.core.section.to_json_section_slice(value.entries),
        "bytes": destack._generated.core.section.to_json_section_slice(value.bytes),
    }


def from_json_string_table(value: Json) -> StringTable:
    """Return one StringTable from one JSON value."""
    object_ = json_object(value)

    return StringTable(
        entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "entries")
        ),
        bytes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "bytes")
        ),
    )


__all__ = [
    "StringTable",
    "encode_string_table",
    "decode_string_table",
    "to_json_string_table",
    "from_json_string_table",
]
