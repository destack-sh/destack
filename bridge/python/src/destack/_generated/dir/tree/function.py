# generated bridge target, do not edit

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

import destack._generated.dir.tree.node
import destack._generated.dir.tree.property

"""The source form used to spell a receiver."""
ThisForm: typing.TypeAlias = typing.Literal["implicit"] | typing.Literal["explicit"]


def encode_this_form(writer: BinaryWriter, value: ThisForm) -> None:
    """Encode one ThisForm."""
    if value == "implicit":
        writer.write_unsigned(0)
    elif value == "explicit":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_this_form(reader: BinaryReader) -> ThisForm:
    """Decode one ThisForm."""
    variant = reader.read_number()

    if variant == 0:
        return "implicit"
    elif variant == 1:
        return "explicit"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_this_form(value: ThisForm) -> Json:
    """Return one JSON value for one ThisForm."""
    return value


def from_json_this_form(value: Json) -> ThisForm:
    """Return one ThisForm from one JSON value."""
    variant = json_string(value)

    if variant == "implicit":
        return "implicit"
    elif variant == "explicit":
        return "explicit"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class FunctionSignature:
    """The signature of a function."""

    # the asynchrony of the function
    asynchrony: destack._generated.dir.tree.node.Asynchrony
    # the special role of the function
    role: destack._generated.dir.tree.property.FunctionRole | None
    # the source form of the function
    form: FunctionForm
    # when the function may be called
    phase: FunctionPhase
    # the generic parameters of the function
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the function
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the source form used for the receiver
    this_form: ThisForm | None
    # the optional `this` parameter
    this_parameter: destack._generated.dir.tree.node.LocalNodeId | None
    # the dynamic parameters of the function
    parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the return type of the function
    return_type: destack._generated.dir.tree.node.LocalNodeId | None
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
    destack._generated.dir.tree.node.encode_asynchrony(writer, value.asynchrony)
    if value.role is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.property.encode_function_role(writer, value.role)
    encode_function_form(writer, value.form)
    encode_function_phase(writer, value.phase)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    if value.this_form is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_this_form(writer, value.this_form)
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.this_parameter
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.return_type)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_override)
    writer.write_bool(value.is_generator)


def decode_function_signature(reader: BinaryReader) -> FunctionSignature:
    """Decode one FunctionSignature."""
    asynchrony = destack._generated.dir.tree.node.decode_asynchrony(reader)
    role = reader.read_option(
        lambda: destack._generated.dir.tree.property.decode_function_role(reader)
    )
    form = decode_function_form(reader)
    phase = decode_function_phase(reader)
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    this_form = reader.read_option(lambda: decode_this_form(reader))
    this_parameter = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    is_abstract = reader.read_bool()
    is_override = reader.read_bool()
    is_generator = reader.read_bool()

    return FunctionSignature(
        asynchrony=asynchrony,
        role=role,
        form=form,
        phase=phase,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        this_form=this_form,
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
        "asynchrony": destack._generated.dir.tree.node.to_json_asynchrony(
            value.asynchrony
        ),
        **(
            {}
            if value.role is None
            else {
                "role": destack._generated.dir.tree.property.to_json_function_role(
                    value.role
                )
            }
        ),
        "form": to_json_function_form(value.form),
        "phase": to_json_function_phase(value.phase),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        **(
            {}
            if value.this_form is None
            else {"thisForm": to_json_this_form(value.this_form)}
        ),
        **(
            {}
            if value.this_parameter is None
            else {
                "thisParameter": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.this_parameter
                )
            }
        ),
        "parameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.dir.tree.node.to_json_local_node_id(
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
        asynchrony=destack._generated.dir.tree.node.from_json_asynchrony(
            json_field(object_, "asynchrony")
        ),
        role=json_optional(
            object_,
            "role",
            lambda value: destack._generated.dir.tree.property.from_json_function_role(
                value
            ),
        ),
        form=from_json_function_form(json_field(object_, "form")),
        phase=from_json_function_phase(json_field(object_, "phase")),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        this_form=json_optional(
            object_, "thisForm", lambda value: from_json_this_form(value)
        ),
        this_parameter=json_optional(
            object_,
            "thisParameter",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_override=json_bool(json_field(object_, "isOverride")),
        is_generator=json_bool(json_field(object_, "isGenerator")),
    )


"""The source form of a function."""
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


"""When a function may be called."""
FunctionPhase: typing.TypeAlias = typing.Literal["normal"] | typing.Literal["comptime"]


def encode_function_phase(writer: BinaryWriter, value: FunctionPhase) -> None:
    """Encode one FunctionPhase."""
    if value == "normal":
        writer.write_unsigned(0)
    elif value == "comptime":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_function_phase(reader: BinaryReader) -> FunctionPhase:
    """Decode one FunctionPhase."""
    variant = reader.read_number()

    if variant == 0:
        return "normal"
    elif variant == 1:
        return "comptime"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_function_phase(value: FunctionPhase) -> Json:
    """Return one JSON value for one FunctionPhase."""
    return value


def from_json_function_phase(value: Json) -> FunctionPhase:
    """Return one FunctionPhase from one JSON value."""
    variant = json_string(value)

    if variant == "normal":
        return "normal"
    elif variant == "comptime":
        return "comptime"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "ThisForm",
    "encode_this_form",
    "decode_this_form",
    "to_json_this_form",
    "from_json_this_form",
    "FunctionSignature",
    "encode_function_signature",
    "decode_function_signature",
    "to_json_function_signature",
    "from_json_function_signature",
    "FunctionForm",
    "encode_function_form",
    "decode_function_form",
    "to_json_function_form",
    "from_json_function_form",
    "FunctionPhase",
    "encode_function_phase",
    "decode_function_phase",
    "to_json_function_phase",
    "from_json_function_phase",
]
