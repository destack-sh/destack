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
class TypeTable:
    """Runtime type table carried by one durable program."""

    # dense runtime type descriptors keyed by program type id
    descriptors: destack._generated.core.section.SectionSlice
    # flattened runtime supertype ids
    supertypes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeTable:
        """Decode one TypeTable."""
        return decode_type_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_table(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeTable:
        """Return one TypeTable from one JSON value."""
        return from_json_type_table(value)


def encode_type_table(writer: BinaryWriter, value: TypeTable) -> None:
    """Encode one TypeTable."""
    destack._generated.core.section.encode_section_slice(writer, value.descriptors)
    destack._generated.core.section.encode_section_slice(writer, value.supertypes)


def decode_type_table(reader: BinaryReader) -> TypeTable:
    """Decode one TypeTable."""
    descriptors = destack._generated.core.section.decode_section_slice(reader)
    supertypes = destack._generated.core.section.decode_section_slice(reader)

    return TypeTable(
        descriptors=descriptors,
        supertypes=supertypes,
    )


def to_json_type_table(value: TypeTable) -> Json:
    """Return one JSON value for one TypeTable."""
    return {
        "descriptors": destack._generated.core.section.to_json_section_slice(
            value.descriptors
        ),
        "supertypes": destack._generated.core.section.to_json_section_slice(
            value.supertypes
        ),
    }


def from_json_type_table(value: Json) -> TypeTable:
    """Return one TypeTable from one JSON value."""
    object_ = json_object(value)

    return TypeTable(
        descriptors=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "descriptors")
        ),
        supertypes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "supertypes")
        ),
    )


__all__ = [
    "TypeTable",
    "encode_type_table",
    "decode_type_table",
    "to_json_type_table",
    "from_json_type_table",
]
