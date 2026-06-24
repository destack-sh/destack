# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.property import (
    PropertyImpl,
)
from destack._impl.dir.tree.property import (
    MemberImpl,
)

import destack._generated.core.string
import destack._generated.dir.symbol.key
import destack._generated.dir.tree.function
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node

"""The special role of a function."""
FunctionRole: typing.TypeAlias = (
    typing.Literal["getter"]
    | typing.Literal["setter"]
    | typing.Literal["constructor"]
    | typing.Literal["new"]
    | typing.Literal["call"]
)

def encode_function_role(writer: BinaryWriter, value: FunctionRole) -> None: ...
def decode_function_role(reader: BinaryReader) -> FunctionRole: ...
def to_json_function_role(value: FunctionRole) -> Json: ...
def from_json_function_role(value: Json) -> FunctionRole: ...

@dataclass(frozen=True, slots=True)
class PropertyField(PropertyImpl):
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    value: destack._generated.dir.tree.node.LocalNodeId
    is_shorthand: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PropertyMethod(PropertyImpl):
    """Object-like member function."""

    key: destack._generated.dir.tree.key.Key | None
    signature: destack._generated.dir.tree.function.FunctionSignature
    body: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PropertySpread(PropertyImpl):
    """Spread property."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PropertyError(PropertyImpl):
    """Malformed property slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A property of an object-like literal."""
Property: typing.TypeAlias = (
    PropertyField | PropertyMethod | PropertySpread | PropertyError
)

def encode_property(writer: BinaryWriter, value: Property) -> None: ...
def decode_property(reader: BinaryReader) -> Property: ...
def to_json_property(value: Property) -> Json: ...
def from_json_property(value: Json) -> Property: ...

@dataclass(frozen=True, slots=True)
class MemberAssociatedType(MemberImpl):
    """Associated type alias."""

    name: destack._generated.core.string.StringId
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberAssociatedConst(MemberImpl):
    """Associated compile-time constant."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberField(MemberImpl):
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    mutability: destack._generated.dir.tree.node.Mutability | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_optional: bool
    is_definite: bool
    is_readonly: bool
    is_ambient: bool
    is_abstract: bool
    is_override: bool
    is_static: bool
    is_accessor: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberMethod(MemberImpl):
    """Named member function."""

    key: destack._generated.dir.tree.key.Key | None
    signature: destack._generated.dir.tree.function.FunctionSignature
    abstraction: MethodAbstraction
    body: destack._generated.dir.tree.node.LocalNodeId | None
    visibility: destack._generated.dir.tree.node.Visibility | None
    is_optional: bool
    is_ambient: bool
    is_override: bool
    is_static: bool
    is_accessor: bool
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberStaticBlock(MemberImpl):
    """Static initialization block."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["staticBlock"] = "staticBlock"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberComptimeBlock(MemberImpl):
    """Comptime block."""

    body: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["comptimeBlock"] = "comptimeBlock"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberError(MemberImpl):
    """Malformed member slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A member of a declaration body."""
Member: typing.TypeAlias = (
    MemberAssociatedType
    | MemberAssociatedConst
    | MemberField
    | MemberMethod
    | MemberStaticBlock
    | MemberComptimeBlock
    | MemberError
)

def encode_member(writer: BinaryWriter, value: Member) -> None: ...
def decode_member(reader: BinaryReader) -> Member: ...
def to_json_member(value: Member) -> Json: ...
def from_json_member(value: Json) -> Member: ...

"""The abstraction mode of a class method."""
MethodAbstraction: typing.TypeAlias = (
    typing.Literal["concrete"] | typing.Literal["virtual"] | typing.Literal["abstract"]
)

def encode_method_abstraction(
    writer: BinaryWriter, value: MethodAbstraction
) -> None: ...
def decode_method_abstraction(reader: BinaryReader) -> MethodAbstraction: ...
def to_json_method_abstraction(value: MethodAbstraction) -> Json: ...
def from_json_method_abstraction(value: Json) -> MethodAbstraction: ...

"""Variance annotation for generic parameters."""
VarianceModifier: typing.TypeAlias = (
    typing.Literal["in"] | typing.Literal["out"] | typing.Literal["inOut"]
)

def encode_variance_modifier(writer: BinaryWriter, value: VarianceModifier) -> None: ...
def decode_variance_modifier(reader: BinaryReader) -> VarianceModifier: ...
def to_json_variance_modifier(value: VarianceModifier) -> Json: ...
def from_json_variance_modifier(value: Json) -> VarianceModifier: ...

@dataclass(frozen=True, slots=True)
class MemberSlotKey:
    """Property keyed by a static key."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberSlotConstructor:
    """Constructor role member."""

    kind: typing.Literal["constructor"] = "constructor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberSlotNew:
    """New role member."""

    kind: typing.Literal["new"] = "new"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberSlotCall:
    """Callable role member."""

    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A nominal member slot."""
MemberSlot: typing.TypeAlias = (
    MemberSlotKey | MemberSlotConstructor | MemberSlotNew | MemberSlotCall
)

def encode_member_slot(writer: BinaryWriter, value: MemberSlot) -> None: ...
def decode_member_slot(reader: BinaryReader) -> MemberSlot: ...
def to_json_member_slot(value: MemberSlot) -> Json: ...
def from_json_member_slot(value: Json) -> MemberSlot: ...

__all__ = [
    "FunctionRole",
    "encode_function_role",
    "decode_function_role",
    "to_json_function_role",
    "from_json_function_role",
    "Property",
    "encode_property",
    "decode_property",
    "to_json_property",
    "from_json_property",
    "PropertyField",
    "PropertyMethod",
    "PropertySpread",
    "PropertyError",
    "Member",
    "encode_member",
    "decode_member",
    "to_json_member",
    "from_json_member",
    "MemberAssociatedType",
    "MemberAssociatedConst",
    "MemberField",
    "MemberMethod",
    "MemberStaticBlock",
    "MemberComptimeBlock",
    "MemberError",
    "MethodAbstraction",
    "encode_method_abstraction",
    "decode_method_abstraction",
    "to_json_method_abstraction",
    "from_json_method_abstraction",
    "VarianceModifier",
    "encode_variance_modifier",
    "decode_variance_modifier",
    "to_json_variance_modifier",
    "from_json_variance_modifier",
    "MemberSlot",
    "encode_member_slot",
    "decode_member_slot",
    "to_json_member_slot",
    "from_json_member_slot",
    "MemberSlotKey",
    "MemberSlotConstructor",
    "MemberSlotNew",
    "MemberSlotCall",
]
