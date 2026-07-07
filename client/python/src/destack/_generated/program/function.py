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
    """Function table carried by one durable program."""

    # dense function entries keyed by program function id
    functions: destack._generated.core.section.SectionSlice
    # flattened function parameter types
    parameters: destack._generated.core.section.SectionSlice
    # exported function names
    exports: destack._generated.core.section.SectionSlice

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
    destack._generated.core.section.encode_section_slice(writer, value.parameters)
    destack._generated.core.section.encode_section_slice(writer, value.exports)


def decode_function_table(reader: BinaryReader) -> FunctionTable:
    """Decode one FunctionTable."""
    functions = destack._generated.core.section.decode_section_slice(reader)
    parameters = destack._generated.core.section.decode_section_slice(reader)
    exports = destack._generated.core.section.decode_section_slice(reader)

    return FunctionTable(
        functions=functions,
        parameters=parameters,
        exports=exports,
    )


def to_json_function_table(value: FunctionTable) -> Json:
    """Return one JSON value for one FunctionTable."""
    return {
        "functions": destack._generated.core.section.to_json_section_slice(
            value.functions
        ),
        "parameters": destack._generated.core.section.to_json_section_slice(
            value.parameters
        ),
        "exports": destack._generated.core.section.to_json_section_slice(value.exports),
    }


def from_json_function_table(value: Json) -> FunctionTable:
    """Return one FunctionTable from one JSON value."""
    object_ = json_object(value)

    return FunctionTable(
        functions=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "functions")
        ),
        parameters=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "parameters")
        ),
        exports=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "exports")
        ),
    )


__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
]
