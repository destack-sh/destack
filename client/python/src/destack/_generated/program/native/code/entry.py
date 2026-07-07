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
class EntryTable:
    """Native entry table keyed by program ids."""

    # native function entries keyed by program function id
    function: destack._generated.core.section.SectionSlice
    # native resume entries keyed by frame state id
    resume: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entry_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryTable:
        """Decode one EntryTable."""
        return decode_entry_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entry_table(self)

    @classmethod
    def from_json(cls, value: Json) -> EntryTable:
        """Return one EntryTable from one JSON value."""
        return from_json_entry_table(value)


def encode_entry_table(writer: BinaryWriter, value: EntryTable) -> None:
    """Encode one EntryTable."""
    destack._generated.core.section.encode_section_slice(writer, value.function)
    destack._generated.core.section.encode_section_slice(writer, value.resume)


def decode_entry_table(reader: BinaryReader) -> EntryTable:
    """Decode one EntryTable."""
    function = destack._generated.core.section.decode_section_slice(reader)
    resume = destack._generated.core.section.decode_section_slice(reader)

    return EntryTable(
        function=function,
        resume=resume,
    )


def to_json_entry_table(value: EntryTable) -> Json:
    """Return one JSON value for one EntryTable."""
    return {
        "function": destack._generated.core.section.to_json_section_slice(
            value.function
        ),
        "resume": destack._generated.core.section.to_json_section_slice(value.resume),
    }


def from_json_entry_table(value: Json) -> EntryTable:
    """Return one EntryTable from one JSON value."""
    object_ = json_object(value)

    return EntryTable(
        function=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "function")
        ),
        resume=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "resume")
        ),
    )


__all__ = [
    "EntryTable",
    "encode_entry_table",
    "decode_entry_table",
    "to_json_entry_table",
    "from_json_entry_table",
]
