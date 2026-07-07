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
class DispatchTable:
    """Dispatch table carried by one durable program."""

    # virtual tables keyed by dense virtual table id
    virtual_tables: destack._generated.core.section.SectionSlice
    # flattened virtual method function ids
    virtual_methods: destack._generated.core.section.SectionSlice
    # dynamic tables keyed by dense dynamic table id
    dynamic_tables: destack._generated.core.section.SectionSlice
    # flattened dynamic dispatch entries
    dynamic_entries: destack._generated.core.section.SectionSlice
    # dynamic table shapes keyed by constraint type
    dynamic_shapes: destack._generated.core.section.SectionSlice
    # flattened dynamic slots
    dynamic_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dispatch_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DispatchTable:
        """Decode one DispatchTable."""
        return decode_dispatch_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dispatch_table(self)

    @classmethod
    def from_json(cls, value: Json) -> DispatchTable:
        """Return one DispatchTable from one JSON value."""
        return from_json_dispatch_table(value)


def encode_dispatch_table(writer: BinaryWriter, value: DispatchTable) -> None:
    """Encode one DispatchTable."""
    destack._generated.core.section.encode_section_slice(writer, value.virtual_tables)
    destack._generated.core.section.encode_section_slice(writer, value.virtual_methods)
    destack._generated.core.section.encode_section_slice(writer, value.dynamic_tables)
    destack._generated.core.section.encode_section_slice(writer, value.dynamic_entries)
    destack._generated.core.section.encode_section_slice(writer, value.dynamic_shapes)
    destack._generated.core.section.encode_section_slice(writer, value.dynamic_slots)


def decode_dispatch_table(reader: BinaryReader) -> DispatchTable:
    """Decode one DispatchTable."""
    virtual_tables = destack._generated.core.section.decode_section_slice(reader)
    virtual_methods = destack._generated.core.section.decode_section_slice(reader)
    dynamic_tables = destack._generated.core.section.decode_section_slice(reader)
    dynamic_entries = destack._generated.core.section.decode_section_slice(reader)
    dynamic_shapes = destack._generated.core.section.decode_section_slice(reader)
    dynamic_slots = destack._generated.core.section.decode_section_slice(reader)

    return DispatchTable(
        virtual_tables=virtual_tables,
        virtual_methods=virtual_methods,
        dynamic_tables=dynamic_tables,
        dynamic_entries=dynamic_entries,
        dynamic_shapes=dynamic_shapes,
        dynamic_slots=dynamic_slots,
    )


def to_json_dispatch_table(value: DispatchTable) -> Json:
    """Return one JSON value for one DispatchTable."""
    return {
        "virtualTables": destack._generated.core.section.to_json_section_slice(
            value.virtual_tables
        ),
        "virtualMethods": destack._generated.core.section.to_json_section_slice(
            value.virtual_methods
        ),
        "dynamicTables": destack._generated.core.section.to_json_section_slice(
            value.dynamic_tables
        ),
        "dynamicEntries": destack._generated.core.section.to_json_section_slice(
            value.dynamic_entries
        ),
        "dynamicShapes": destack._generated.core.section.to_json_section_slice(
            value.dynamic_shapes
        ),
        "dynamicSlots": destack._generated.core.section.to_json_section_slice(
            value.dynamic_slots
        ),
    }


def from_json_dispatch_table(value: Json) -> DispatchTable:
    """Return one DispatchTable from one JSON value."""
    object_ = json_object(value)

    return DispatchTable(
        virtual_tables=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "virtualTables")
        ),
        virtual_methods=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "virtualMethods")
        ),
        dynamic_tables=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "dynamicTables")
        ),
        dynamic_entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "dynamicEntries")
        ),
        dynamic_shapes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "dynamicShapes")
        ),
        dynamic_slots=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "dynamicSlots")
        ),
    )


__all__ = [
    "DispatchTable",
    "encode_dispatch_table",
    "decode_dispatch_table",
    "to_json_dispatch_table",
    "from_json_dispatch_table",
]
