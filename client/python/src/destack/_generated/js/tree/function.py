# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class FunctionSignature:
    """The signature of a function."""

    # the asynchrony of the function
    asynchrony: destack._generated.js.tree.node.Asynchrony
    # the role of the function
    role: FunctionRole | None
    # the form of the function
    form: FunctionForm
    # the generic parameters of the function
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the optional `this` parameter of the function
    this_parameter: destack._generated.js.tree.node.LocalNodeId | None
    # the runtime parameters of the function
    parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the return type of the function
    return_type: destack._generated.js.tree.node.LocalNodeId | None
    # whether the function is abstract
    is_abstract: bool
    # whether the function is an override
    is_override: bool
    # whether the function is a generator
    is_generator: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_signature(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignature:
        """Decode one FunctionSignature."""
        return decode_function_signature(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_signature(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionSignature:
        """Return one FunctionSignature from one JSON value."""
        return from_json_function_signature(value)


def encode_function_signature(writer: BinaryWriter, value: FunctionSignature) -> None:
    """Encode one FunctionSignature."""
    destack._generated.js.tree.node.encode_asynchrony(writer, value.asynchrony)
    if value.role is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_function_role(writer, value.role)
    encode_function_form(writer, value.form)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(
            writer, value.this_parameter
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.return_type)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_override)
    writer.write_bool(value.is_generator)


def decode_function_signature(reader: BinaryReader) -> FunctionSignature:
    """Decode one FunctionSignature."""
    asynchrony = destack._generated.js.tree.node.decode_asynchrony(reader)
    role = reader.read_option(lambda: decode_function_role(reader))
    form = decode_function_form(reader)
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    this_parameter = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    is_abstract = reader.read_bool()
    is_override = reader.read_bool()
    is_generator = reader.read_bool()

    return FunctionSignature(
        asynchrony=asynchrony,
        role=role,
        form=form,
        generic_parameters=generic_parameters,
        this_parameter=this_parameter,
        parameters=parameters,
        return_type=return_type,
        is_abstract=is_abstract,
        is_override=is_override,
        is_generator=is_generator,
    )


def to_json_function_signature(value: FunctionSignature) -> Json:
    """Return one JSON value for one FunctionSignature."""
    return {
        "asynchrony": destack._generated.js.tree.node.to_json_asynchrony(
            value.asynchrony
        ),
        **({} if value.role is None else {"role": to_json_function_role(value.role)}),
        "form": to_json_function_form(value.form),
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        **(
            {}
            if value.this_parameter is None
            else {
                "thisParameter": destack._generated.js.tree.node.to_json_local_node_id(
                    value.this_parameter
                )
            }
        ),
        "parameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.js.tree.node.to_json_local_node_id(
                    value.return_type
                )
            }
        ),
        "isAbstract": value.is_abstract,
        "isOverride": value.is_override,
        "isGenerator": value.is_generator,
    }


def from_json_function_signature(value: Json) -> FunctionSignature:
    """Return one FunctionSignature from one JSON value."""
    object_ = json_object(value)

    return FunctionSignature(
        asynchrony=destack._generated.js.tree.node.from_json_asynchrony(
            json_field(object_, "asynchrony")
        ),
        role=json_optional(
            object_, "role", lambda value: from_json_function_role(value)
        ),
        form=from_json_function_form(json_field(object_, "form")),
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        this_parameter=json_optional(
            object_,
            "thisParameter",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_override=json_bool(json_field(object_, "isOverride")),
        is_generator=json_bool(json_field(object_, "isGenerator")),
    )


"""The role of a function."""
FunctionRole: typing.TypeAlias = (
    typing.Literal["getter"] | typing.Literal["setter"] | typing.Literal["constructor"]
)


def encode_function_role(writer: BinaryWriter, value: FunctionRole) -> None:
    """Encode one FunctionRole."""
    if value == "getter":
        writer.write_unsigned(0)
    elif value == "setter":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_function_role(reader: BinaryReader) -> FunctionRole:
    """Decode one FunctionRole."""
    variant = reader.read_number()

    if variant == 0:
        return "getter"
    elif variant == 1:
        return "setter"
    elif variant == 2:
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_function_role(value: FunctionRole) -> Json:
    """Return one JSON value for one FunctionRole."""
    return value


def from_json_function_role(value: Json) -> FunctionRole:
    """Return one FunctionRole from one JSON value."""
    variant = json_string(value)

    if variant == "getter":
        return "getter"
    elif variant == "setter":
        return "setter"
    elif variant == "constructor":
        return "constructor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The style of a function."""
FunctionForm: typing.TypeAlias = typing.Literal["function"] | typing.Literal["lambda"]


def encode_function_form(writer: BinaryWriter, value: FunctionForm) -> None:
    """Encode one FunctionForm."""
    if value == "function":
        writer.write_unsigned(0)
    elif value == "lambda":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_function_form(reader: BinaryReader) -> FunctionForm:
    """Decode one FunctionForm."""
    variant = reader.read_number()

    if variant == 0:
        return "function"
    elif variant == 1:
        return "lambda"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_function_form(value: FunctionForm) -> Json:
    """Return one JSON value for one FunctionForm."""
    return value


def from_json_function_form(value: Json) -> FunctionForm:
    """Return one FunctionForm from one JSON value."""
    variant = json_string(value)

    if variant == "function":
        return "function"
    elif variant == "lambda":
        return "lambda"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "FunctionSignature",
    "encode_function_signature",
    "decode_function_signature",
    "to_json_function_signature",
    "from_json_function_signature",
    "FunctionRole",
    "encode_function_role",
    "decode_function_role",
    "to_json_function_role",
    "from_json_function_role",
    "FunctionForm",
    "encode_function_form",
    "decode_function_form",
    "to_json_function_form",
    "from_json_function_form",
]
