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

import destack._generated.program.vm.function
import destack._generated.program.vm.state
import destack._generated.program.vm.table


@dataclass(frozen=True, slots=True)
class Code:
    """Durable VM code body."""

    # lowered function bodies for the VM backend
    functions: destack._generated.program.vm.function.FunctionTable
    # side table referenced by compact side records
    side_table: destack._generated.program.vm.table.SideTable
    # resume states keyed by lowered VM program point
    resume: destack._generated.program.vm.state.ResumeTable

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_code(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Code:
        """Decode one Code."""
        return decode_code(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_code(self)

    @classmethod
    def from_json(cls, value: Json) -> Code:
        """Return one Code from one JSON value."""
        return from_json_code(value)


def encode_code(writer: BinaryWriter, value: Code) -> None:
    """Encode one Code."""
    destack._generated.program.vm.function.encode_function_table(
        writer, value.functions
    )
    destack._generated.program.vm.table.encode_side_table(writer, value.side_table)
    destack._generated.program.vm.state.encode_resume_table(writer, value.resume)


def decode_code(reader: BinaryReader) -> Code:
    """Decode one Code."""
    functions = destack._generated.program.vm.function.decode_function_table(reader)
    side_table = destack._generated.program.vm.table.decode_side_table(reader)
    resume = destack._generated.program.vm.state.decode_resume_table(reader)

    return Code(
        functions=functions,
        side_table=side_table,
        resume=resume,
    )


def to_json_code(value: Code) -> Json:
    """Return one JSON value for one Code."""
    return {
        "functions": destack._generated.program.vm.function.to_json_function_table(
            value.functions
        ),
        "sideTable": destack._generated.program.vm.table.to_json_side_table(
            value.side_table
        ),
        "resume": destack._generated.program.vm.state.to_json_resume_table(
            value.resume
        ),
    }


def from_json_code(value: Json) -> Code:
    """Return one Code from one JSON value."""
    object_ = json_object(value)

    return Code(
        functions=destack._generated.program.vm.function.from_json_function_table(
            json_field(object_, "functions")
        ),
        side_table=destack._generated.program.vm.table.from_json_side_table(
            json_field(object_, "sideTable")
        ),
        resume=destack._generated.program.vm.state.from_json_resume_table(
            json_field(object_, "resume")
        ),
    )


__all__ = [
    "Code",
    "encode_code",
    "decode_code",
    "to_json_code",
    "from_json_code",
]
