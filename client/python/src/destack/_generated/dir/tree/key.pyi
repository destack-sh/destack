# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.key import (
    NameImpl,
)
from destack._impl.dir.tree.key import (
    KeyImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.node

@dataclass(frozen=True, slots=True)
class NameIdentifier(NameImpl):
    """A regular identifier."""

    identifier: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NameString(NameImpl):
    """A string identifier."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NameIndex(NameImpl):
    """A positional index."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A name is a regular, string, or numeric identifier."""
Name: typing.TypeAlias = NameIdentifier | NameString | NameIndex

def encode_name(writer: BinaryWriter, value: Name) -> None: ...
def decode_name(reader: BinaryReader) -> Name: ...
def to_json_name(value: Name) -> Json: ...
def from_json_name(value: Json) -> Name: ...

@dataclass(frozen=True, slots=True)
class KeyName(KeyImpl):
    """A named key."""

    name: Name
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class KeyPrivate(KeyImpl):
    """A private key."""

    private: destack._generated.core.string.StringId
    kind: typing.Literal["private"] = "private"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class KeyExpression(KeyImpl):
    """A dynamic value-space key."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A key in value or type property position."""
Key: typing.TypeAlias = KeyName | KeyPrivate | KeyExpression

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
    "NameIndex",
    "Key",
    "encode_key",
    "decode_key",
    "to_json_key",
    "from_json_key",
    "KeyName",
    "KeyPrivate",
    "KeyExpression",
]
