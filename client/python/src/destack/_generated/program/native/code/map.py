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
class CodeMap:
    """Native code map for entries, safepoints, roots, and deoptimization."""

    # native function code ranges
    function: destack._generated.core.section.SectionSlice
    # native continuation resume code ranges
    resume: destack._generated.core.section.SectionSlice
    # native safepoints keyed by safepoint id
    safepoint: destack._generated.core.section.SectionSlice
    # native roots referenced by safepoints
    root: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code_map(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CodeMap:
        """Decode one CodeMap."""
        return decode_code_map(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code_map(self)

    @classmethod
    def from_json(cls, value: Json) -> CodeMap:
        """Return one CodeMap from one JSON value."""
        return from_json_code_map(value)


def encode_code_map(writer: BinaryWriter, value: CodeMap) -> None:
    """Encode one CodeMap."""
    destack._generated.core.section.encode_section_slice(writer, value.function)
    destack._generated.core.section.encode_section_slice(writer, value.resume)
    destack._generated.core.section.encode_section_slice(writer, value.safepoint)
    destack._generated.core.section.encode_section_slice(writer, value.root)


def decode_code_map(reader: BinaryReader) -> CodeMap:
    """Decode one CodeMap."""
    function = destack._generated.core.section.decode_section_slice(reader)
    resume = destack._generated.core.section.decode_section_slice(reader)
    safepoint = destack._generated.core.section.decode_section_slice(reader)
    root = destack._generated.core.section.decode_section_slice(reader)

    return CodeMap(
        function=function,
        resume=resume,
        safepoint=safepoint,
        root=root,
    )


def to_json_code_map(value: CodeMap) -> Json:
    """Return one JSON value for one CodeMap."""
    return {
        "function": destack._generated.core.section.to_json_section_slice(
            value.function
        ),
        "resume": destack._generated.core.section.to_json_section_slice(value.resume),
        "safepoint": destack._generated.core.section.to_json_section_slice(
            value.safepoint
        ),
        "root": destack._generated.core.section.to_json_section_slice(value.root),
    }


def from_json_code_map(value: Json) -> CodeMap:
    """Return one CodeMap from one JSON value."""
    object_ = json_object(value)

    return CodeMap(
        function=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "function")
        ),
        resume=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "resume")
        ),
        safepoint=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "safepoint")
        ),
        root=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "root")
        ),
    )


__all__ = [
    "CodeMap",
    "encode_code_map",
    "decode_code_map",
    "to_json_code_map",
    "from_json_code_map",
]
