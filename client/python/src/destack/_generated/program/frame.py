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
class FrameTable:
    """Physical execution frame layout and materialization tables."""

    # frame materializations by frame state id
    materializations: destack._generated.core.section.SectionSlice
    # frame layouts by id
    layouts: destack._generated.core.section.SectionSlice
    # flattened frame slots
    slots: destack._generated.core.section.SectionSlice
    # flattened copied materialization slots
    copied_slots: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameTable:
        """Decode one FrameTable."""
        return decode_frame_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameTable:
        """Return one FrameTable from one JSON value."""
        return from_json_frame_table(value)


def encode_frame_table(writer: BinaryWriter, value: FrameTable) -> None:
    """Encode one FrameTable."""
    destack._generated.core.section.encode_section_slice(writer, value.materializations)
    destack._generated.core.section.encode_section_slice(writer, value.layouts)
    destack._generated.core.section.encode_section_slice(writer, value.slots)
    destack._generated.core.section.encode_section_slice(writer, value.copied_slots)


def decode_frame_table(reader: BinaryReader) -> FrameTable:
    """Decode one FrameTable."""
    materializations = destack._generated.core.section.decode_section_slice(reader)
    layouts = destack._generated.core.section.decode_section_slice(reader)
    slots = destack._generated.core.section.decode_section_slice(reader)
    copied_slots = destack._generated.core.section.decode_section_slice(reader)

    return FrameTable(
        materializations=materializations,
        layouts=layouts,
        slots=slots,
        copied_slots=copied_slots,
    )


def to_json_frame_table(value: FrameTable) -> Json:
    """Return one JSON value for one FrameTable."""
    return {
        "materializations": destack._generated.core.section.to_json_section_slice(
            value.materializations
        ),
        "layouts": destack._generated.core.section.to_json_section_slice(value.layouts),
        "slots": destack._generated.core.section.to_json_section_slice(value.slots),
        "copiedSlots": destack._generated.core.section.to_json_section_slice(
            value.copied_slots
        ),
    }


def from_json_frame_table(value: Json) -> FrameTable:
    """Return one FrameTable from one JSON value."""
    object_ = json_object(value)

    return FrameTable(
        materializations=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "materializations")
        ),
        layouts=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "layouts")
        ),
        slots=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "slots")
        ),
        copied_slots=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "copiedSlots")
        ),
    )


__all__ = [
    "FrameTable",
    "encode_frame_table",
    "decode_frame_table",
    "to_json_frame_table",
    "from_json_frame_table",
]
