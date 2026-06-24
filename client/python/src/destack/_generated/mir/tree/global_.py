# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.node
import destack._generated.mir.tree.symbol
import destack._generated.mir.tree.type

"""Symbol linkage (visibility and definition location)."""
Linkage: typing.TypeAlias = (
    typing.Literal["local"] | typing.Literal["export"] | typing.Literal["import"]
)


def encode_linkage(writer: BinaryWriter, value: Linkage) -> None:
    """Encode one Linkage."""
    if value == "local":
        writer.write_unsigned(0)
    elif value == "export":
        writer.write_unsigned(1)
    elif value == "import":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_linkage(reader: BinaryReader) -> Linkage:
    """Decode one Linkage."""
    variant = reader.read_number()

    if variant == 0:
        return "local"
    elif variant == 1:
        return "export"
    elif variant == 2:
        return "import"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_linkage(value: Linkage) -> Json:
    """Return one JSON value for one Linkage."""
    return value


def from_json_linkage(value: Json) -> Linkage:
    """Return one Linkage from one JSON value."""
    variant = json_string(value)

    if variant == "local":
        return "local"
    elif variant == "export":
        return "export"
    elif variant == "import":
        return "import"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class Global:
    """Global data definition (module-level variable or constant)."""

    # name for linking and debugging
    name: destack._generated.core.string.StringId
    # the global's persistent mangled symbol: its linkable identity
    symbol: destack._generated.mir.tree.symbol.Symbol
    # the type of the global
    ty: destack._generated.mir.tree.node.LocalNodeId
    # whether this global is mutable
    mutability: destack._generated.mir.tree.type.Mutability
    # the space that owns this global storage
    space: destack._generated.mir.tree.type.Space
    # linkage (local, export, or import)
    linkage: Linkage
    # initial value. None for imported globals
    initializer: GlobalInitializer | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Global:
        """Decode one Global."""
        return decode_global(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global(self)

    @classmethod
    def from_json(cls, value: Json) -> Global:
        """Return one Global from one JSON value."""
        return from_json_global(value)


def encode_global(writer: BinaryWriter, value: Global) -> None:
    """Encode one Global."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.mir.tree.symbol.encode_symbol(writer, value.symbol)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    destack._generated.mir.tree.type.encode_mutability(writer, value.mutability)
    destack._generated.mir.tree.type.encode_space(writer, value.space)
    encode_linkage(writer, value.linkage)
    if value.initializer is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_global_initializer(writer, value.initializer)


def decode_global(reader: BinaryReader) -> Global:
    """Decode one Global."""
    name = destack._generated.core.string.decode_string_id(reader)
    symbol = destack._generated.mir.tree.symbol.decode_symbol(reader)
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    mutability = destack._generated.mir.tree.type.decode_mutability(reader)
    space = destack._generated.mir.tree.type.decode_space(reader)
    linkage = decode_linkage(reader)
    initializer = reader.read_option(lambda: decode_global_initializer(reader))

    return Global(
        name=name,
        symbol=symbol,
        ty=ty,
        mutability=mutability,
        space=space,
        linkage=linkage,
        initializer=initializer,
    )


def to_json_global(value: Global) -> Json:
    """Return one JSON value for one Global."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "symbol": destack._generated.mir.tree.symbol.to_json_symbol(value.symbol),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "mutability": destack._generated.mir.tree.type.to_json_mutability(
            value.mutability
        ),
        "space": destack._generated.mir.tree.type.to_json_space(value.space),
        "linkage": to_json_linkage(value.linkage),
        **(
            {}
            if value.initializer is None
            else {"initializer": to_json_global_initializer(value.initializer)}
        ),
    }


def from_json_global(value: Json) -> Global:
    """Return one Global from one JSON value."""
    object_ = json_object(value)

    return Global(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        symbol=destack._generated.mir.tree.symbol.from_json_symbol(
            json_field(object_, "symbol")
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        mutability=destack._generated.mir.tree.type.from_json_mutability(
            json_field(object_, "mutability")
        ),
        space=destack._generated.mir.tree.type.from_json_space(
            json_field(object_, "space")
        ),
        linkage=from_json_linkage(json_field(object_, "linkage")),
        initializer=json_optional(
            object_, "initializer", lambda value: from_json_global_initializer(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class GlobalInitializerZero:
    """Zero-initialized (all bytes zero)."""

    kind: typing.Literal["zero"] = "zero"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_initializer(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_initializer(self)


@dataclass(frozen=True, slots=True)
class GlobalInitializerScalar:
    """Scalar constant (bool, int, float)."""

    scalar: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_initializer(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_initializer(self)


@dataclass(frozen=True, slots=True)
class GlobalInitializerFunctionAddress:
    """Address of one function inside the program."""

    function_address: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionAddress"] = "functionAddress"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_initializer(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_initializer(self)


@dataclass(frozen=True, slots=True)
class GlobalInitializerBytes:
    """Raw bytes (blobs)."""

    bytes: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["bytes"] = "bytes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_initializer(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_initializer(self)


@dataclass(frozen=True, slots=True)
class GlobalInitializerAggregate:
    """Aggregate (array/struct fields)."""

    aggregate: Sequence[GlobalInitializer]
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_initializer(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_initializer(self)


"""Initializer for global data."""
GlobalInitializer: typing.TypeAlias = (
    GlobalInitializerZero
    | GlobalInitializerScalar
    | GlobalInitializerFunctionAddress
    | GlobalInitializerBytes
    | GlobalInitializerAggregate
)


def encode_global_initializer(writer: BinaryWriter, value: GlobalInitializer) -> None:
    """Encode one GlobalInitializer."""
    if value.kind == "zero":
        writer.write_unsigned(0)
    elif value.kind == "scalar":
        writer.write_unsigned(1)
        destack._generated.mir.tree.constant.encode_constant(writer, value.scalar)
    elif value.kind == "functionAddress":
        writer.write_unsigned(2)
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, value.function_address
        )
    elif value.kind == "bytes":
        writer.write_unsigned(3)
        writer.write_byte_slice(value.bytes)
    elif value.kind == "aggregate":
        writer.write_unsigned(4)
        writer.write_unsigned(len(value.aggregate))
        for item_value_aggregate_0 in value.aggregate:
            encode_global_initializer(writer, item_value_aggregate_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_global_initializer(reader: BinaryReader) -> GlobalInitializer:
    """Decode one GlobalInitializer."""
    variant = reader.read_number()

    if variant == 0:
        return GlobalInitializerZero()
    elif variant == 1:
        scalar = destack._generated.mir.tree.constant.decode_constant(reader)

        return GlobalInitializerScalar(scalar=scalar)
    elif variant == 2:
        function_address = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return GlobalInitializerFunctionAddress(function_address=function_address)
    elif variant == 3:
        bytes = reader.read_byte_slice()

        return GlobalInitializerBytes(bytes=bytes)
    elif variant == 4:
        aggregate = [
            decode_global_initializer(reader) for _ in range(reader.read_number())
        ]

        return GlobalInitializerAggregate(aggregate=aggregate)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_global_initializer(value: GlobalInitializer) -> Json:
    """Return one JSON value for one GlobalInitializer."""
    if value.kind == "zero":
        return {
            "kind": "zero",
        }
    elif value.kind == "scalar":
        return {
            "kind": "scalar",
            "scalar": destack._generated.mir.tree.constant.to_json_constant(
                value.scalar
            ),
        }
    elif value.kind == "functionAddress":
        return {
            "kind": "functionAddress",
            "function_address": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function_address
            ),
        }
    elif value.kind == "bytes":
        return {
            "kind": "bytes",
            "bytes": bytes_to_json(value.bytes),
        }
    elif value.kind == "aggregate":
        return {
            "kind": "aggregate",
            "aggregate": [
                to_json_global_initializer(item_0) for item_0 in value.aggregate
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_global_initializer(value: Json) -> GlobalInitializer:
    """Return one GlobalInitializer from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "zero":
        return GlobalInitializerZero()
    elif kind == "scalar":
        return GlobalInitializerScalar(
            scalar=destack._generated.mir.tree.constant.from_json_constant(
                json_field(object_, "scalar")
            )
        )
    elif kind == "functionAddress":
        return GlobalInitializerFunctionAddress(
            function_address=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function_address")
            )
        )
    elif kind == "bytes":
        return GlobalInitializerBytes(
            bytes=bytes_from_json(json_field(object_, "bytes"))
        )
    elif kind == "aggregate":
        return GlobalInitializerAggregate(
            aggregate=[
                from_json_global_initializer(item_0)
                for item_0 in json_array(json_field(object_, "aggregate"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Linkage",
    "encode_linkage",
    "decode_linkage",
    "to_json_linkage",
    "from_json_linkage",
    "Global",
    "encode_global",
    "decode_global",
    "to_json_global",
    "from_json_global",
    "GlobalInitializer",
    "encode_global_initializer",
    "decode_global_initializer",
    "to_json_global_initializer",
    "from_json_global_initializer",
    "GlobalInitializerZero",
    "GlobalInitializerScalar",
    "GlobalInitializerFunctionAddress",
    "GlobalInitializerBytes",
    "GlobalInitializerAggregate",
]
