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
class LayoutTable:
    """Shared layout table for runtime values."""

    # layout entries indexed by LayoutId
    layouts: destack._generated.core.section.SectionSlice
    # flattened field layout entries
    fields: destack._generated.core.section.SectionSlice
    # flattened variant case layout entries
    variants: destack._generated.core.section.SectionSlice
    # flattened signature parameter type entries
    parameters: destack._generated.core.section.SectionSlice
    # flattened tensor sharding axis entries
    tensor_axes: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_layout_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LayoutTable:
        """Decode one LayoutTable."""
        return decode_layout_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_layout_table(self)

    @classmethod
    def from_json(cls, value: Json) -> LayoutTable:
        """Return one LayoutTable from one JSON value."""
        return from_json_layout_table(value)


def encode_layout_table(writer: BinaryWriter, value: LayoutTable) -> None:
    """Encode one LayoutTable."""
    destack._generated.core.section.encode_section_slice(writer, value.layouts)
    destack._generated.core.section.encode_section_slice(writer, value.fields)
    destack._generated.core.section.encode_section_slice(writer, value.variants)
    destack._generated.core.section.encode_section_slice(writer, value.parameters)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_axes)


def decode_layout_table(reader: BinaryReader) -> LayoutTable:
    """Decode one LayoutTable."""
    layouts = destack._generated.core.section.decode_section_slice(reader)
    fields = destack._generated.core.section.decode_section_slice(reader)
    variants = destack._generated.core.section.decode_section_slice(reader)
    parameters = destack._generated.core.section.decode_section_slice(reader)
    tensor_axes = destack._generated.core.section.decode_section_slice(reader)

    return LayoutTable(
        layouts=layouts,
        fields=fields,
        variants=variants,
        parameters=parameters,
        tensor_axes=tensor_axes,
    )


def to_json_layout_table(value: LayoutTable) -> Json:
    """Return one JSON value for one LayoutTable."""
    return {
        "layouts": destack._generated.core.section.to_json_section_slice(value.layouts),
        "fields": destack._generated.core.section.to_json_section_slice(value.fields),
        "variants": destack._generated.core.section.to_json_section_slice(
            value.variants
        ),
        "parameters": destack._generated.core.section.to_json_section_slice(
            value.parameters
        ),
        "tensorAxes": destack._generated.core.section.to_json_section_slice(
            value.tensor_axes
        ),
    }


def from_json_layout_table(value: Json) -> LayoutTable:
    """Return one LayoutTable from one JSON value."""
    object_ = json_object(value)

    return LayoutTable(
        layouts=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "layouts")
        ),
        fields=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "fields")
        ),
        variants=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "variants")
        ),
        parameters=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "parameters")
        ),
        tensor_axes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorAxes")
        ),
    )


__all__ = [
    "LayoutTable",
    "encode_layout_table",
    "decode_layout_table",
    "to_json_layout_table",
    "from_json_layout_table",
]
