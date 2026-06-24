# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


@dataclass(frozen=True, slots=True)
class NameString(NameImpl):
    """A string identifier."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


@dataclass(frozen=True, slots=True)
class NameIndex(NameImpl):
    """A positional index."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


"""A name is a regular, string, or numeric identifier."""
Name: typing.TypeAlias = NameIdentifier | NameString | NameIndex


def encode_name(writer: BinaryWriter, value: Name) -> None:
    """Encode one Name."""
    if value.kind == "identifier":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.identifier)
    elif value.kind == "string":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "index":
        writer.write_unsigned(2)
        writer.write_unsigned(value.index)
    else:
        raise SerdeError("unknown enum variant")


def decode_name(reader: BinaryReader) -> Name:
    """Decode one Name."""
    variant = reader.read_number()

    if variant == 0:
        identifier = destack._generated.core.string.decode_string_id(reader)

        return NameIdentifier(identifier=identifier)
    elif variant == 1:
        string = destack._generated.core.string.decode_string_id(reader)

        return NameString(string=string)
    elif variant == 2:
        index = reader.read_number()

        return NameIndex(index=index)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_name(value: Name) -> Json:
    """Return one JSON value for one Name."""
    if value.kind == "identifier":
        return {
            "kind": "identifier",
            "identifier": destack._generated.core.string.to_json_string_id(
                value.identifier
            ),
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": value.index,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_name(value: Json) -> Name:
    """Return one Name from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "identifier":
        return NameIdentifier(
            identifier=destack._generated.core.string.from_json_string_id(
                json_field(object_, "identifier")
            )
        )
    elif kind == "string":
        return NameString(
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            )
        )
    elif kind == "index":
        return NameIndex(index=json_int(json_field(object_, "index")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class KeyName(KeyImpl):
    """A named key."""

    name: Name
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


@dataclass(frozen=True, slots=True)
class KeyPrivate(KeyImpl):
    """A private key."""

    private: destack._generated.core.string.StringId
    kind: typing.Literal["private"] = "private"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


@dataclass(frozen=True, slots=True)
class KeyExpression(KeyImpl):
    """A dynamic value-space key."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


"""A key in value or type property position."""
Key: typing.TypeAlias = KeyName | KeyPrivate | KeyExpression


def encode_key(writer: BinaryWriter, value: Key) -> None:
    """Encode one Key."""
    if value.kind == "name":
        writer.write_unsigned(0)
        encode_name(writer, value.name)
    elif value.kind == "private":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.private)
    elif value.kind == "expression":
        writer.write_unsigned(2)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    else:
        raise SerdeError("unknown enum variant")


def decode_key(reader: BinaryReader) -> Key:
    """Decode one Key."""
    variant = reader.read_number()

    if variant == 0:
        name = decode_name(reader)

        return KeyName(name=name)
    elif variant == 1:
        private = destack._generated.core.string.decode_string_id(reader)

        return KeyPrivate(private=private)
    elif variant == 2:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return KeyExpression(expression=expression)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_key(value: Key) -> Json:
    """Return one JSON value for one Key."""
    if value.kind == "name":
        return {
            "kind": "name",
            "name": to_json_name(value.name),
        }
    elif value.kind == "private":
        return {
            "kind": "private",
            "private": destack._generated.core.string.to_json_string_id(value.private),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_key(value: Json) -> Key:
    """Return one Key from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "name":
        return KeyName(name=from_json_name(json_field(object_, "name")))
    elif kind == "private":
        return KeyPrivate(
            private=destack._generated.core.string.from_json_string_id(
                json_field(object_, "private")
            )
        )
    elif kind == "expression":
        return KeyExpression(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
