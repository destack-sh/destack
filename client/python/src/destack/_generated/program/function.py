# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.program.type


@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Runtime function metadata carried by one durable program."""

    # dense function records keyed by program function id
    functions: Sequence[Function | None]
    # function ids keyed by exported source name
    function_by_name: Mapping[str, FunctionId]

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
    writer.write_unsigned(len(value.functions))
    for item_value_functions_0 in value.functions:
        if item_value_functions_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_function(writer, item_value_functions_0)
    entries_value_function_by_name_0 = []
    for (
        key_value_function_by_name_0,
        item_value_function_by_name_0,
    ) in value.function_by_name.items():

        def write_key_value_function_by_name_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_function_by_name_0)

        key_bytes = nested_bytes(write_key_value_function_by_name_0)
        entries_value_function_by_name_0.append(
            (key_value_function_by_name_0, item_value_function_by_name_0, key_bytes)
        )
    entries_value_function_by_name_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_function_by_name_0))
    for entry_value_function_by_name_0 in entries_value_function_by_name_0:
        writer.write_string(entry_value_function_by_name_0[0])
        encode_function_id(writer, entry_value_function_by_name_0[1])


def decode_function_table(reader: BinaryReader) -> FunctionTable:
    """Decode one FunctionTable."""
    functions = [
        reader.read_option(lambda: decode_function(reader))
        for _ in range(reader.read_number())
    ]
    function_by_name = {
        reader.read_string(): decode_function_id(reader)
        for _ in range(reader.read_number())
    }

    return FunctionTable(
        functions=functions,
        function_by_name=function_by_name,
    )


def to_json_function_table(value: FunctionTable) -> Json:
    """Return one JSON value for one FunctionTable."""
    return {
        "functions": [
            None if item_0 is None else to_json_function(item_0)
            for item_0 in value.functions
        ],
        "functionByName": {
            key_0: to_json_function_id(item_0)
            for key_0, item_0 in value.function_by_name.items()
        },
    }


def from_json_function_table(value: Json) -> FunctionTable:
    """Return one FunctionTable from one JSON value."""
    object_ = json_object(value)

    return FunctionTable(
        functions=[
            None if item_0 is None else from_json_function(item_0)
            for item_0 in json_array(json_field(object_, "functions"))
        ],
        function_by_name={
            key_0: from_json_function_id(item_0)
            for key_0, item_0 in json_object(
                json_field(object_, "functionByName")
            ).items()
        },
    )


@dataclass(frozen=True, slots=True)
class Function:
    """Runtime function metadata."""

    # the source-facing function name
    name: str
    # function parameter types
    parameters: Sequence[destack._generated.program.type.TypeId]
    # function return type
    return_type: destack._generated.program.type.TypeId
    # captured closure environment type when one exists
    environment: destack._generated.program.type.TypeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Function:
        """Decode one Function."""
        return decode_function(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function(self)

    @classmethod
    def from_json(cls, value: Json) -> Function:
        """Return one Function from one JSON value."""
        return from_json_function(value)


def encode_function(writer: BinaryWriter, value: Function) -> None:
    """Encode one Function."""
    writer.write_string(value.name)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.program.type.encode_type_id(writer, item_value_parameters_0)
    destack._generated.program.type.encode_type_id(writer, value.return_type)
    if value.environment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.program.type.encode_type_id(writer, value.environment)


def decode_function(reader: BinaryReader) -> Function:
    """Decode one Function."""
    name = reader.read_string()
    parameters = [
        destack._generated.program.type.decode_type_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = destack._generated.program.type.decode_type_id(reader)
    environment = reader.read_option(
        lambda: destack._generated.program.type.decode_type_id(reader)
    )

    return Function(
        name=name,
        parameters=parameters,
        return_type=return_type,
        environment=environment,
    )


def to_json_function(value: Function) -> Json:
    """Return one JSON value for one Function."""
    return {
        "name": value.name,
        "parameters": [
            destack._generated.program.type.to_json_type_id(item_0)
            for item_0 in value.parameters
        ],
        "returnType": destack._generated.program.type.to_json_type_id(
            value.return_type
        ),
        **(
            {}
            if value.environment is None
            else {
                "environment": destack._generated.program.type.to_json_type_id(
                    value.environment
                )
            }
        ),
    }


def from_json_function(value: Json) -> Function:
    """Return one Function from one JSON value."""
    object_ = json_object(value)

    return Function(
        name=json_string(json_field(object_, "name")),
        parameters=[
            destack._generated.program.type.from_json_type_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=destack._generated.program.type.from_json_type_id(
            json_field(object_, "returnType")
        ),
        environment=json_optional(
            object_,
            "environment",
            lambda value: destack._generated.program.type.from_json_type_id(value),
        ),
    )


"""Durable runtime function id inside one program."""
FunctionId: typing.TypeAlias = int


def encode_function_id(writer: BinaryWriter, value: FunctionId) -> None:
    """Encode one FunctionId."""
    writer.write_unsigned(value)


def decode_function_id(reader: BinaryReader) -> FunctionId:
    """Decode one FunctionId."""
    return reader.read_number()


def to_json_function_id(value: FunctionId) -> Json:
    """Return one JSON value for one FunctionId."""
    return value


def from_json_function_id(value: Json) -> FunctionId:
    """Return one FunctionId from one JSON value."""
    return json_int(value)


__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
    "Function",
    "encode_function",
    "decode_function",
    "to_json_function",
    "from_json_function",
    "FunctionId",
    "encode_function_id",
    "decode_function_id",
    "to_json_function_id",
    "from_json_function_id",
]
