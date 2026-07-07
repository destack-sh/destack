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
class FunctionTable:
    """Lowered VM function registry owned by one program."""

    # lowered functions by dense local index
    functions: destack._generated.core.section.SectionSlice
    # flattened instruction entries
    code: destack._generated.core.section.SectionSlice
    # flattened block entries
    blocks: destack._generated.core.section.SectionSlice
    # flattened argument move slots
    argument_pool: destack._generated.core.section.SectionSlice
    # flattened move pairs
    move_pool: destack._generated.core.section.SectionSlice
    # dense call target entries keyed by program function id
    call_targets: destack._generated.core.section.SectionSlice

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTable:
        """Decode one FunctionTable."""
        return decode_function_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_table(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionTable:
        """Return one FunctionTable from one JSON value."""
        return from_json_function_table(value)


def encode_function_table(writer: BinaryWriter, value: FunctionTable) -> None:
    """Encode one FunctionTable."""
    destack._generated.core.section.encode_section_slice(writer, value.functions)
    destack._generated.core.section.encode_section_slice(writer, value.code)
    destack._generated.core.section.encode_section_slice(writer, value.blocks)
    destack._generated.core.section.encode_section_slice(writer, value.argument_pool)
    destack._generated.core.section.encode_section_slice(writer, value.move_pool)
    destack._generated.core.section.encode_section_slice(writer, value.call_targets)


def decode_function_table(reader: BinaryReader) -> FunctionTable:
    """Decode one FunctionTable."""
    functions = destack._generated.core.section.decode_section_slice(reader)
    code = destack._generated.core.section.decode_section_slice(reader)
    blocks = destack._generated.core.section.decode_section_slice(reader)
    argument_pool = destack._generated.core.section.decode_section_slice(reader)
    move_pool = destack._generated.core.section.decode_section_slice(reader)
    call_targets = destack._generated.core.section.decode_section_slice(reader)

    return FunctionTable(
        functions=functions,
        code=code,
        blocks=blocks,
        argument_pool=argument_pool,
        move_pool=move_pool,
        call_targets=call_targets,
    )


def to_json_function_table(value: FunctionTable) -> Json:
    """Return one JSON value for one FunctionTable."""
    return {
        "functions": destack._generated.core.section.to_json_section_slice(
            value.functions
        ),
        "code": destack._generated.core.section.to_json_section_slice(value.code),
        "blocks": destack._generated.core.section.to_json_section_slice(value.blocks),
        "argumentPool": destack._generated.core.section.to_json_section_slice(
            value.argument_pool
        ),
        "movePool": destack._generated.core.section.to_json_section_slice(
            value.move_pool
        ),
        "callTargets": destack._generated.core.section.to_json_section_slice(
            value.call_targets
        ),
    }


def from_json_function_table(value: Json) -> FunctionTable:
    """Return one FunctionTable from one JSON value."""
    object_ = json_object(value)

    return FunctionTable(
        functions=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "functions")
        ),
        code=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "code")
        ),
        blocks=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "blocks")
        ),
        argument_pool=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "argumentPool")
        ),
        move_pool=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "movePool")
        ),
        call_targets=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callTargets")
        ),
    )


__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
]
