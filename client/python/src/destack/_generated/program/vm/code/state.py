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
class ResumeTable:
    """VM resume states keyed by execution frame state."""

    # resume states by dense frame state id
    states: destack._generated.core.section.SectionSlice
    # resume state ids sorted by lowered program point
    points: destack._generated.core.section.SectionSlice
    # flattened frame entry bindings
    bindings: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resume_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeTable:
        """Decode one ResumeTable."""
        return decode_resume_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resume_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ResumeTable:
        """Return one ResumeTable from one JSON value."""
        return from_json_resume_table(value)


def encode_resume_table(writer: BinaryWriter, value: ResumeTable) -> None:
    """Encode one ResumeTable."""
    destack._generated.core.section.encode_section_slice(writer, value.states)
    destack._generated.core.section.encode_section_slice(writer, value.points)
    destack._generated.core.section.encode_section_slice(writer, value.bindings)


def decode_resume_table(reader: BinaryReader) -> ResumeTable:
    """Decode one ResumeTable."""
    states = destack._generated.core.section.decode_section_slice(reader)
    points = destack._generated.core.section.decode_section_slice(reader)
    bindings = destack._generated.core.section.decode_section_slice(reader)

    return ResumeTable(
        states=states,
        points=points,
        bindings=bindings,
    )


def to_json_resume_table(value: ResumeTable) -> Json:
    """Return one JSON value for one ResumeTable."""
    return {
        "states": destack._generated.core.section.to_json_section_slice(value.states),
        "points": destack._generated.core.section.to_json_section_slice(value.points),
        "bindings": destack._generated.core.section.to_json_section_slice(
            value.bindings
        ),
    }


def from_json_resume_table(value: Json) -> ResumeTable:
    """Return one ResumeTable from one JSON value."""
    object_ = json_object(value)

    return ResumeTable(
        states=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "states")
        ),
        points=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "points")
        ),
        bindings=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "bindings")
        ),
    )


__all__ = [
    "ResumeTable",
    "encode_resume_table",
    "decode_resume_table",
    "to_json_resume_table",
    "from_json_resume_table",
]
