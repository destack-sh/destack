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
class TraceTable:
    """Compact heap trace table stored in program sections."""

    # top-level trace roots indexed by TraceId
    roots: destack._generated.core.section.SectionSlice
    # flat trace entries
    entries: destack._generated.core.section.SectionSlice
    # flattened trace byte offsets
    offsets: destack._generated.core.section.SectionSlice
    # flattened child entry ids
    children: destack._generated.core.section.SectionSlice
    # flattened tagged variant entries
    variants: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_trace_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TraceTable:
        """Decode one TraceTable."""
        return decode_trace_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_trace_table(self)

    @classmethod
    def from_json(cls, value: Json) -> TraceTable:
        """Return one TraceTable from one JSON value."""
        return from_json_trace_table(value)


def encode_trace_table(writer: BinaryWriter, value: TraceTable) -> None:
    """Encode one TraceTable."""
    destack._generated.core.section.encode_section_slice(writer, value.roots)
    destack._generated.core.section.encode_section_slice(writer, value.entries)
    destack._generated.core.section.encode_section_slice(writer, value.offsets)
    destack._generated.core.section.encode_section_slice(writer, value.children)
    destack._generated.core.section.encode_section_slice(writer, value.variants)


def decode_trace_table(reader: BinaryReader) -> TraceTable:
    """Decode one TraceTable."""
    roots = destack._generated.core.section.decode_section_slice(reader)
    entries = destack._generated.core.section.decode_section_slice(reader)
    offsets = destack._generated.core.section.decode_section_slice(reader)
    children = destack._generated.core.section.decode_section_slice(reader)
    variants = destack._generated.core.section.decode_section_slice(reader)

    return TraceTable(
        roots=roots,
        entries=entries,
        offsets=offsets,
        children=children,
        variants=variants,
    )


def to_json_trace_table(value: TraceTable) -> Json:
    """Return one JSON value for one TraceTable."""
    return {
        "roots": destack._generated.core.section.to_json_section_slice(value.roots),
        "entries": destack._generated.core.section.to_json_section_slice(value.entries),
        "offsets": destack._generated.core.section.to_json_section_slice(value.offsets),
        "children": destack._generated.core.section.to_json_section_slice(
            value.children
        ),
        "variants": destack._generated.core.section.to_json_section_slice(
            value.variants
        ),
    }


def from_json_trace_table(value: Json) -> TraceTable:
    """Return one TraceTable from one JSON value."""
    object_ = json_object(value)

    return TraceTable(
        roots=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "roots")
        ),
        entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "entries")
        ),
        offsets=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "offsets")
        ),
        children=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "children")
        ),
        variants=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "variants")
        ),
    )


__all__ = [
    "TraceTable",
    "encode_trace_table",
    "decode_trace_table",
    "to_json_trace_table",
    "from_json_trace_table",
]
