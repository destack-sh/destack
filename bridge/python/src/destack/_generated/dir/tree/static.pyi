# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.tree.function
import destack._generated.dir.tree.literal
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GlobalStaticId:
    """Global static id across modules."""

    # the module id of the global static value
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global static value
    local_id: LocalStaticId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalStaticId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalStaticId: ...

def encode_global_static_id(writer: BinaryWriter, value: GlobalStaticId) -> None: ...
def decode_global_static_id(reader: BinaryReader) -> GlobalStaticId: ...
def to_json_global_static_id(value: GlobalStaticId) -> Json: ...
def from_json_global_static_id(value: Json) -> GlobalStaticId: ...

"""Unique identifier for a local static value."""
LocalStaticId: typing.TypeAlias = int

def encode_local_static_id(writer: BinaryWriter, value: LocalStaticId) -> None: ...
def decode_local_static_id(reader: BinaryReader) -> LocalStaticId: ...
def to_json_local_static_id(value: LocalStaticId) -> Json: ...
def from_json_local_static_id(value: Json) -> LocalStaticId: ...

@dataclass(frozen=True, slots=True)
class StaticTermScalarLiteral:
    """Scalar literal."""

    value: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermType:
    """Type value."""

    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermArray:
    """Array value."""

    elements: Sequence[StaticTerm]
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermFixedArray:
    """Fixed array value."""

    # the repeated value
    value: StaticTerm
    # the fixed array length
    length: int
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermTuple:
    """Tuple value."""

    elements: Sequence[StaticTerm]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermObject:
    """Structural object value."""

    # the object properties
    properties: Sequence[StaticProperty]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticTermStruct:
    """Nominal struct value."""

    # the struct type selected for this value
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the struct properties
    properties: Sequence[StaticProperty]
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Concrete static value produced by checked static evaluation."""
StaticTerm: typing.TypeAlias = (
    StaticTermScalarLiteral
    | StaticTermType
    | StaticTermArray
    | StaticTermFixedArray
    | StaticTermTuple
    | StaticTermObject
    | StaticTermStruct
)

def encode_static_term(writer: BinaryWriter, value: StaticTerm) -> None: ...
def decode_static_term(reader: BinaryReader) -> StaticTerm: ...
def to_json_static_term(value: StaticTerm) -> Json: ...
def from_json_static_term(value: Json) -> StaticTerm: ...

@dataclass(frozen=True, slots=True)
class StaticPropertyField:
    """Static field."""

    # the property key
    key: destack._generated.dir.symbol.key.StaticKey
    # the property value
    value: StaticTerm
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticPropertyMethod:
    """Static member function."""

    # the optional method key
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the method signature
    signature: destack._generated.dir.tree.function.FunctionSignature
    # the method body
    body: StaticTerm
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StaticPropertySpread:
    """Static spread."""

    # the spread value
    value: StaticTerm
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Static object property in a checked static context."""
StaticProperty: typing.TypeAlias = (
    StaticPropertyField | StaticPropertyMethod | StaticPropertySpread
)

def encode_static_property(writer: BinaryWriter, value: StaticProperty) -> None: ...
def decode_static_property(reader: BinaryReader) -> StaticProperty: ...
def to_json_static_property(value: StaticProperty) -> Json: ...
def from_json_static_property(value: Json) -> StaticProperty: ...

__all__ = [
    "GlobalStaticId",
    "encode_global_static_id",
    "decode_global_static_id",
    "to_json_global_static_id",
    "from_json_global_static_id",
    "LocalStaticId",
    "encode_local_static_id",
    "decode_local_static_id",
    "to_json_local_static_id",
    "from_json_local_static_id",
    "StaticTerm",
    "encode_static_term",
    "decode_static_term",
    "to_json_static_term",
    "from_json_static_term",
    "StaticTermScalarLiteral",
    "StaticTermType",
    "StaticTermArray",
    "StaticTermFixedArray",
    "StaticTermTuple",
    "StaticTermObject",
    "StaticTermStruct",
    "StaticProperty",
    "encode_static_property",
    "decode_static_property",
    "to_json_static_property",
    "from_json_static_property",
    "StaticPropertyField",
    "StaticPropertyMethod",
    "StaticPropertySpread",
]
