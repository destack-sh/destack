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
class ImportTable:
    """Native imports required by one native code payload."""

    # native imports in linker order
    import_: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportTable:
        """Decode one ImportTable."""
        return decode_import_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportTable:
        """Return one ImportTable from one JSON value."""
        return from_json_import_table(value)


def encode_import_table(writer: BinaryWriter, value: ImportTable) -> None:
    """Encode one ImportTable."""
    destack._generated.core.section.encode_section_slice(writer, value.import_)


def decode_import_table(reader: BinaryReader) -> ImportTable:
    """Decode one ImportTable."""
    import_ = destack._generated.core.section.decode_section_slice(reader)

    return ImportTable(
        import_=import_,
    )


def to_json_import_table(value: ImportTable) -> Json:
    """Return one JSON value for one ImportTable."""
    return {
        "import": destack._generated.core.section.to_json_section_slice(value.import_),
    }


def from_json_import_table(value: Json) -> ImportTable:
    """Return one ImportTable from one JSON value."""
    object_ = json_object(value)

    return ImportTable(
        import_=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "import")
        ),
    )


__all__ = [
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
]
