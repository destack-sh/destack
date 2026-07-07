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
class GlobalTable:
    """Global table carried by one program."""

    # global records keyed by dense global id
    globals: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTable:
        """Decode one GlobalTable."""
        return decode_global_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_table(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalTable:
        """Return one GlobalTable from one JSON value."""
        return from_json_global_table(value)


def encode_global_table(writer: BinaryWriter, value: GlobalTable) -> None:
    """Encode one GlobalTable."""
    destack._generated.core.section.encode_section_slice(writer, value.globals)


def decode_global_table(reader: BinaryReader) -> GlobalTable:
    """Decode one GlobalTable."""
    globals = destack._generated.core.section.decode_section_slice(reader)

    return GlobalTable(
        globals=globals,
    )


def to_json_global_table(value: GlobalTable) -> Json:
    """Return one JSON value for one GlobalTable."""
    return {
        "globals": destack._generated.core.section.to_json_section_slice(value.globals),
    }


def from_json_global_table(value: Json) -> GlobalTable:
    """Return one GlobalTable from one JSON value."""
    object_ = json_object(value)

    return GlobalTable(
        globals=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "globals")
        ),
    )


__all__ = [
    "GlobalTable",
    "encode_global_table",
    "decode_global_table",
    "to_json_global_table",
    "from_json_global_table",
]
