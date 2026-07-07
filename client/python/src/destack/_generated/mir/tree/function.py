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
    # the return type
    return_type: destack._generated.mir.tree.node.LocalNodeId
    # the hidden environment type for this function when present
    environment: destack._generated.mir.tree.node.LocalNodeId | None
    # runtime binding name when this function has a binding identity
    binding: destack._generated.core.string.StringId | None
    # the executable function body when this function is defined
    body: FunctionBody | None
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
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.return_type)
    if value.environment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.environment)
    if value.binding is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.binding)
    if value.body is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_function_body(writer, value.body)
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
    return_type = destack._generated.mir.tree.node.decode_local_node_id(reader)
    environment = reader.read_option(
        lambda: destack._generated.mir.tree.node.decode_local_node_id(reader)
    )
    binding = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    body = reader.read_option(lambda: decode_function_body(reader))
    allocation = decode_allocation_mode(reader)
    suspension = reader.read_option(lambda: decode_suspension_kind(reader))

    return Function(
        name=name,
        symbol=symbol,
        linkage=linkage,
        parameters=parameters,
        lifetimes=lifetimes,
        parameter_names=parameter_names,
        return_type=return_type,
        environment=environment,
        binding=binding,
        body=body,
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
        **(
            {}
            if value.binding is None
            else {
                "binding": destack._generated.core.string.to_json_string_id(
                    value.binding
                )
            }
        ),
        **({} if value.body is None else {"body": to_json_function_body(value.body)}),
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
        binding=json_optional(
            object_,
            "binding",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        body=json_optional(
            object_, "body", lambda value: from_json_function_body(value)
        ),
        allocation=from_json_allocation_mode(json_field(object_, "allocation")),
        suspension=json_optional(
            object_, "suspension", lambda value: from_json_suspension_kind(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class FunctionBody:
    """The executable body of one MIR function."""

    # the entry block where execution starts
    entry: destack._generated.mir.tree.node.LocalNodeId
    # the function blocks in layout order
    blocks: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # the function locals in slot order
    locals: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # optional explicit SSA value names keyed by value id
    value_names: Sequence[destack._generated.core.string.StringId | None]
    # SSA value types keyed by value id
    value_types: Sequence[destack._generated.mir.tree.node.LocalNodeId | None]
    # counter for allocating unique SSA value ids
    next_value_id: int
    # instruction locations keyed by instruction id
    instruction_index: InstructionIndex

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_body(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBody:
        """Decode one FunctionBody."""
        return decode_function_body(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_body(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionBody:
        """Return one FunctionBody from one JSON value."""
        return from_json_function_body(value)


def encode_function_body(writer: BinaryWriter, value: FunctionBody) -> None:
    """Encode one FunctionBody."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.entry)
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_blocks_0
        )
    writer.write_unsigned(len(value.locals))
    for item_value_locals_0 in value.locals:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, item_value_locals_0
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
    encode_instruction_index(writer, value.instruction_index)


def decode_function_body(reader: BinaryReader) -> FunctionBody:
    """Decode one FunctionBody."""
    entry = destack._generated.mir.tree.node.decode_local_node_id(reader)
    blocks = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    locals = [
        destack._generated.mir.tree.node.decode_local_node_id(reader)
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
    instruction_index = decode_instruction_index(reader)

    return FunctionBody(
        entry=entry,
        blocks=blocks,
        locals=locals,
        value_names=value_names,
        value_types=value_types,
        next_value_id=next_value_id,
        instruction_index=instruction_index,
    )


def to_json_function_body(value: FunctionBody) -> Json:
    """Return one JSON value for one FunctionBody."""
    return {
        "entry": destack._generated.mir.tree.node.to_json_local_node_id(value.entry),
        "blocks": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.blocks
        ],
        "locals": [
            destack._generated.mir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.locals
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
        "instructionIndex": to_json_instruction_index(value.instruction_index),
    }


def from_json_function_body(value: Json) -> FunctionBody:
    """Return one FunctionBody from one JSON value."""
    object_ = json_object(value)

    return FunctionBody(
        entry=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "entry")
        ),
        blocks=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        locals=[
            destack._generated.mir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "locals"))
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
        instruction_index=from_json_instruction_index(
            json_field(object_, "instructionIndex")
        ),
    )


@dataclass(frozen=True, slots=True)
class InstructionIndex:
    """Function-local instruction location index."""

    # instruction locations keyed by instruction id
    locations: Sequence[InstructionLocation | None]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InstructionIndex:
        """Decode one InstructionIndex."""
        return decode_instruction_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction_index(self)

    @classmethod
    def from_json(cls, value: Json) -> InstructionIndex:
        """Return one InstructionIndex from one JSON value."""
        return from_json_instruction_index(value)


def encode_instruction_index(writer: BinaryWriter, value: InstructionIndex) -> None:
    """Encode one InstructionIndex."""
    writer.write_unsigned(len(value.locations))
    for item_value_locations_0 in value.locations:
        if item_value_locations_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_instruction_location(writer, item_value_locations_0)


def decode_instruction_index(reader: BinaryReader) -> InstructionIndex:
    """Decode one InstructionIndex."""
    locations = [
        reader.read_option(lambda: decode_instruction_location(reader))
        for _ in range(reader.read_number())
    ]

    return InstructionIndex(
        locations=locations,
    )


def to_json_instruction_index(value: InstructionIndex) -> Json:
    """Return one JSON value for one InstructionIndex."""
    return {
        "locations": [
            None if item_0 is None else to_json_instruction_location(item_0)
            for item_0 in value.locations
        ],
    }


def from_json_instruction_index(value: Json) -> InstructionIndex:
    """Return one InstructionIndex from one JSON value."""
    object_ = json_object(value)

    return InstructionIndex(
        locations=[
            None if item_0 is None else from_json_instruction_location(item_0)
            for item_0 in json_array(json_field(object_, "locations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InstructionLocation:
    """Location of one instruction in a MIR function body."""

    # the block that owns the instruction
    block: destack._generated.mir.tree.node.LocalNodeId
    # the instruction index in the block
    index: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction_location(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InstructionLocation:
        """Decode one InstructionLocation."""
        return decode_instruction_location(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction_location(self)

    @classmethod
    def from_json(cls, value: Json) -> InstructionLocation:
        """Return one InstructionLocation from one JSON value."""
        return from_json_instruction_location(value)


def encode_instruction_location(
    writer: BinaryWriter, value: InstructionLocation
) -> None:
    """Encode one InstructionLocation."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.block)
    writer.write_unsigned(value.index)


def decode_instruction_location(reader: BinaryReader) -> InstructionLocation:
    """Decode one InstructionLocation."""
    block = destack._generated.mir.tree.node.decode_local_node_id(reader)
    index = reader.read_number()

    return InstructionLocation(
        block=block,
        index=index,
    )


def to_json_instruction_location(value: InstructionLocation) -> Json:
    """Return one JSON value for one InstructionLocation."""
    return {
        "block": destack._generated.mir.tree.node.to_json_local_node_id(value.block),
        "index": value.index,
    }


def from_json_instruction_location(value: Json) -> InstructionLocation:
    """Return one InstructionLocation from one JSON value."""
    object_ = json_object(value)

    return InstructionLocation(
        block=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "block")
        ),
        index=json_int(json_field(object_, "index")),
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
    "FunctionBody",
    "encode_function_body",
    "decode_function_body",
    "to_json_function_body",
    "from_json_function_body",
    "InstructionIndex",
    "encode_instruction_index",
    "decode_instruction_index",
    "to_json_instruction_index",
    "from_json_instruction_index",
    "InstructionLocation",
    "encode_instruction_location",
    "decode_instruction_location",
    "to_json_instruction_location",
    "from_json_instruction_location",
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
