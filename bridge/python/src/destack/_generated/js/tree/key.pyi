# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class NameIdentifier:
    """A regular identifier (regular `x` or `someThing`)."""

    identifier: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NameString:
    """A string identifier (like `["Content-Type"]`, only in certain contexts)."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Name is a regular or string identifier."""
Name: typing.TypeAlias = NameIdentifier | NameString

def encode_name(writer: BinaryWriter, value: Name) -> None: ...
def decode_name(reader: BinaryReader) -> Name: ...
def to_json_name(value: Name) -> Json: ...
def from_json_name(value: Json) -> Name: ...

@dataclass(frozen=True, slots=True)
class KeyName:
    """Name (like `x` or `someThing`)."""

    name: Name
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class KeyPrivate:
    """Private name (like `#x`)."""

    private: destack._generated.core.string.StringId
    kind: typing.Literal["private"] = "private"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class KeyExpression:
    """Dynamic key (like `["Content-Type"]`)."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class KeyNamedExpression:
    """Named dynamic key (like `[x: string]: any`)."""

    name: Name
    key: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["namedExpression"] = "namedExpression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A Key is a name or a dynamic key."""
Key: typing.TypeAlias = KeyName | KeyPrivate | KeyExpression | KeyNamedExpression

def encode_key(writer: BinaryWriter, value: Key) -> None: ...
def decode_key(reader: BinaryReader) -> Key: ...
def to_json_key(value: Key) -> Json: ...
def from_json_key(value: Json) -> Key: ...

__all__ = [
    "Name",
    "encode_name",
    "decode_name",
    "to_json_name",
    "from_json_name",
    "NameIdentifier",
    "NameString",
    "Key",
    "encode_key",
    "decode_key",
    "to_json_key",
    "from_json_key",
    "KeyName",
    "KeyPrivate",
    "KeyExpression",
    "KeyNamedExpression",
]
