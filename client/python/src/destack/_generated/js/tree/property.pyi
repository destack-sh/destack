# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.js.tree.argument
import destack._generated.js.tree.function
import destack._generated.js.tree.key
import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class PropertyField:
    """Named field (like `x: int32`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    value: destack._generated.js.tree.node.LocalNodeId
    is_shorthand: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PropertyMethod:
    """Named member function (like `foo()` or `<T>(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key | None
    signature: destack._generated.js.tree.function.FunctionSignature
    body: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PropertySpread:
    """Spread property (like `...a`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Property is a property of an object literal (may be a field, method, or spread)."""
Property: typing.TypeAlias = PropertyField | PropertyMethod | PropertySpread

def encode_property(writer: BinaryWriter, value: Property) -> None: ...
def decode_property(reader: BinaryReader) -> Property: ...
def to_json_property(value: Property) -> Json: ...
def from_json_property(value: Json) -> Property: ...

@dataclass(frozen=True, slots=True)
class MemberField:
    """Named field (like `x: int32`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    value: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberMethod:
    """Named member function (like `foo()` or `<T>(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key | None
    signature: destack._generated.js.tree.function.FunctionSignature
    body: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberStaticBlock:
    """Static initialization block (like `static { ... }`)."""

    body: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["staticBlock"] = "staticBlock"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Member is a member of a object-like declaration."""
Member: typing.TypeAlias = MemberField | MemberMethod | MemberStaticBlock

def encode_member(writer: BinaryWriter, value: Member) -> None: ...
def decode_member(reader: BinaryReader) -> Member: ...
def to_json_member(value: Member) -> Json: ...
def from_json_member(value: Json) -> Member: ...

__all__ = [
    "Property",
    "encode_property",
    "decode_property",
    "to_json_property",
    "from_json_property",
    "PropertyField",
    "PropertyMethod",
    "PropertySpread",
    "Member",
    "encode_member",
    "decode_member",
    "to_json_member",
    "from_json_member",
    "MemberField",
    "MemberMethod",
    "MemberStaticBlock",
]
