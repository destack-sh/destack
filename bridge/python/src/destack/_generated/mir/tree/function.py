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
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.mir.tree.global_
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter
import destack._generated.mir.tree.symbol


@dataclass(frozen=True, slots=True)
class Function:
    """A function in MIR."""

    # the function's name (for linking and debugging)
    name: destack._generated.core.string.StringId
    # the function's persistent mangled symbol: its linkable identity
    symbol: destack._generated.mir.tree.symbol.Symbol
    # linkage (local, export, or import)
    linkage: destack._generated.mir.tree.global_.Linkage
    # function parameters as typed SSA slots
    parameters: Sequence[destack._generated.mir.tree.parameter.FunctionParameter]
    # lifetime parameters in function-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # optional parameter names for diagnostics
    parameter_names: Sequence[destack._generated.core.string.StringId | None]
    # optional explicit SSA value names keyed by value id
    value_names: Sequence[destack._generated.core.string.StringId | None]
    # SSA value types keyed by value id
    value_types: Sequence[destack._generated.mir.tree.node.LocalNodeId | None]
    # counter for allocating unique SSA value IDs
    next_value_id: int
    # the return type
    return_type: destack._generated.mir.tree.node.LocalNodeId
    # the hidden environment type for this function when present
    environment: destack._generated.mir.tree.node.LocalNodeId | None
    # local variables (stack-allocated slots for mutable bindings)
    locals: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # all basic blocks in this function
    blocks: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # the entry block (execution starts here)
    entry: destack._generated.mir.tree.node.LocalNodeId | None
    # memory allocation restrictions for this function
    allocation: AllocationMode
    # the suspension kind when this function can suspend
    suspension: SuspensionKind | None

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
    destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.mir.tree.symbol.encode_symbol(writer, value.symbol)
    destack._generated.mir.tree.global_.encode_linkage(writer, value.linkage)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.mir.tree.parameter.encode_function_parameter(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.lifetimes))
    for item_value_lifetimes_0 in value.lifetimes:
        destack._generated.mir.tree.lifetime.encode_lifetime_parameter(
            writer, item_value_lifetimes_0
        )
    writer.write_unsigned(len(value.parameter_names))
    for item_value_parameter_names_0 in value.parameter_names:
        if item_value_parameter_names_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(
                writer, item_value_parameter_names_0
            )
    writer.write_unsigned(len(value.value_names))
    for item_value_value_names_0 in value.value_names:
        if item_value_value_names_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(
                writer, item_value_value_names_0
            )
    writer.write_unsigned(len(value.value_types))
    for item_value_value_types_0 in value.value_types:
        if item_value_value_types_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, item_value_value_types_0
            )
    writer.write_unsigned(value.next_value_id)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.return_type)
    if value.environment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.environment)
    writer.write_unsigned(len(value.locals))
    for item_value_locals_0 in value.locals:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_locals_0
        )
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_blocks_0
        )
    if value.entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.entry)
    encode_allocation_mode(writer, value.allocation)
    if value.suspension is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_suspension_kind(writer, value.suspension)


def decode_function(reader: BinaryReader) -> Function:
    """Decode one Function."""
    name = destack._generated.core.string.decode_string_id(reader)
    symbol = destack._generated.mir.tree.symbol.decode_symbol(reader)
    linkage = destack._generated.mir.tree.global_.decode_linkage(reader)
    parameters = [
        destack._generated.mir.tree.parameter.decode_function_parameter(reader)
        for _ in range(reader.read_number())
    ]
    lifetimes = [
        destack._generated.mir.tree.lifetime.decode_lifetime_parameter(reader)
        for _ in range(reader.read_number())
    ]
    parameter_names = [
        reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        for _ in range(reader.read_number())
    ]
    value_names = [
        reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        for _ in range(reader.read_number())
    ]
    value_types = [
        reader.read_option(
            lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
        )
        for _ in range(reader.read_number())
    ]
    next_value_id = reader.read_number()
    return_type = destack._generated.mir.tree.node.decode_local_node_id(reader)
    environment = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    locals = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    blocks = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    entry = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    allocation = decode_allocation_mode(reader)
    suspension = reader.read_option(lambda: decode_suspension_kind(reader))

    return Function(
        name=name,
        symbol=symbol,
        linkage=linkage,
        parameters=parameters,
        lifetimes=lifetimes,
        parameter_names=parameter_names,
        value_names=value_names,
        value_types=value_types,
        next_value_id=next_value_id,
        return_type=return_type,
        environment=environment,
        locals=locals,
        blocks=blocks,
        entry=entry,
        allocation=allocation,
        suspension=suspension,
    )


def to_json_function(value: Function) -> Json:
    """Return one JSON value for one Function."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "symbol": destack._generated.mir.tree.symbol.to_json_symbol(value.symbol),
        "linkage": destack._generated.mir.tree.global_.to_json_linkage(value.linkage),
        "parameters": [
            destack._generated.mir.tree.parameter.to_json_function_parameter(item_0)
            for item_0 in value.parameters
        ],
        "lifetimes": [
            destack._generated.mir.tree.lifetime.to_json_lifetime_parameter(item_0)
            for item_0 in value.lifetimes
        ],
        "parameterNames": [
            None
            if item_0 is None
            else destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.parameter_names
        ],
        "valueNames": [
            None
            if item_0 is None
            else destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.value_names
        ],
        "valueTypes": [
            None
            if item_0 is None
            else destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.value_types
        ],
        "nextValueId": value.next_value_id,
        "returnType": destack._generated.mir.tree.node.to_json_local_node_id(
            value.return_type
        ),
        **(
            {}
            if value.environment is None
            else {
                "environment": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.environment
                )
            }
        ),
        "locals": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.locals
        ],
        "blocks": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.blocks
        ],
        **(
            {}
            if value.entry is None
            else {
                "entry": destack._generated.mir.tree.node.to_json_local_node_id(
                    value.entry
                )
            }
        ),
        "allocation": to_json_allocation_mode(value.allocation),
        **(
            {}
            if value.suspension is None
            else {"suspension": to_json_suspension_kind(value.suspension)}
        ),
    }


def from_json_function(value: Json) -> Function:
    """Return one Function from one JSON value."""
    object_ = json_object(value)

    return Function(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        symbol=destack._generated.mir.tree.symbol.from_json_symbol(
            json_field(object_, "symbol")
        ),
        linkage=destack._generated.mir.tree.global_.from_json_linkage(
            json_field(object_, "linkage")
        ),
        parameters=[
            destack._generated.mir.tree.parameter.from_json_function_parameter(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        lifetimes=[
            destack._generated.mir.tree.lifetime.from_json_lifetime_parameter(item_0)
            for item_0 in json_array(json_field(object_, "lifetimes"))
        ],
        parameter_names=[
            None
            if item_0 is None
            else destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "parameterNames"))
        ],
        value_names=[
            None
            if item_0 is None
            else destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "valueNames"))
        ],
        value_types=[
            None
            if item_0 is None
            else destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "valueTypes"))
        ],
        next_value_id=json_int(json_field(object_, "nextValueId")),
        return_type=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "returnType")
        ),
        environment=json_optional(
            object_,
            "environment",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        locals=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "locals"))
        ],
        blocks=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        entry=json_optional(
            object_,
            "entry",
            lambda value: destack._generated.mir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        allocation=from_json_allocation_mode(json_field(object_, "allocation")),
        suspension=json_optional(
            object_, "suspension", lambda value: from_json_suspension_kind(value)
        ),
    )


"""Memory allocation restrictions for a function."""
AllocationMode: typing.TypeAlias = (
    typing.Literal["any"] | typing.Literal["noManaged"] | typing.Literal["noHeap"]
)


def encode_allocation_mode(writer: BinaryWriter, value: AllocationMode) -> None:
    """Encode one AllocationMode."""
    if value == "any":
        writer.write_unsigned(0)
    elif value == "noManaged":
        writer.write_unsigned(1)
    elif value == "noHeap":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_allocation_mode(reader: BinaryReader) -> AllocationMode:
    """Decode one AllocationMode."""
    variant = reader.read_number()

    if variant == 0:
        return "any"
    elif variant == 1:
        return "noManaged"
    elif variant == 2:
        return "noHeap"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_allocation_mode(value: AllocationMode) -> Json:
    """Return one JSON value for one AllocationMode."""
    return value


def from_json_allocation_mode(value: Json) -> AllocationMode:
    """Return one AllocationMode from one JSON value."""
    variant = json_string(value)

    if variant == "any":
        return "any"
    elif variant == "noManaged":
        return "noManaged"
    elif variant == "noHeap":
        return "noHeap"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The suspension kind for one function."""
SuspensionKind: typing.TypeAlias = (
    typing.Literal["generator"]
    | typing.Literal["async"]
    | typing.Literal["asyncGenerator"]
)


def encode_suspension_kind(writer: BinaryWriter, value: SuspensionKind) -> None:
    """Encode one SuspensionKind."""
    if value == "generator":
        writer.write_unsigned(0)
    elif value == "async":
        writer.write_unsigned(1)
    elif value == "asyncGenerator":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_suspension_kind(reader: BinaryReader) -> SuspensionKind:
    """Decode one SuspensionKind."""
    variant = reader.read_number()

    if variant == 0:
        return "generator"
    elif variant == 1:
        return "async"
    elif variant == 2:
        return "asyncGenerator"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_suspension_kind(value: SuspensionKind) -> Json:
    """Return one JSON value for one SuspensionKind."""
    return value


def from_json_suspension_kind(value: Json) -> SuspensionKind:
    """Return one SuspensionKind from one JSON value."""
    variant = json_string(value)

    if variant == "generator":
        return "generator"
    elif variant == "async":
        return "async"
    elif variant == "asyncGenerator":
        return "asyncGenerator"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Function",
    "encode_function",
    "decode_function",
    "to_json_function",
    "from_json_function",
    "AllocationMode",
    "encode_allocation_mode",
    "decode_allocation_mode",
    "to_json_allocation_mode",
    "from_json_allocation_mode",
    "SuspensionKind",
    "encode_suspension_kind",
    "decode_suspension_kind",
    "to_json_suspension_kind",
    "from_json_suspension_kind",
]
