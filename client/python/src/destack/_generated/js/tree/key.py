# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

import destack._generated.core.string
import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class NameIdentifier:
    """A regular identifier (regular `x` or `someThing`)."""

    identifier: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


@dataclass(frozen=True, slots=True)
class NameString:
    """A string identifier (like `["Content-Type"]`, only in certain contexts)."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name(self)


"""A Name is a regular or string identifier."""
Name: typing.TypeAlias = NameIdentifier | NameString


def encode_name(writer: BinaryWriter, value: Name) -> None:
    """Encode one Name."""
    if value.kind == "identifier":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.identifier)
    elif value.kind == "string":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.string)
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
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class KeyName:
    """Name (like `x` or `someThing`)."""

    name: Name
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


@dataclass(frozen=True, slots=True)
class KeyPrivate:
    """Private name (like `#x`)."""

    private: destack._generated.core.string.StringId
    kind: typing.Literal["private"] = "private"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


@dataclass(frozen=True, slots=True)
class KeyExpression:
    """Dynamic key (like `["Content-Type"]`)."""

    expression: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


@dataclass(frozen=True, slots=True)
class KeyNamedExpression:
    """Named dynamic key (like `[x: string]: any`)."""

    name: Name
    key: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["namedExpression"] = "namedExpression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_key(self)


"""A Key is a name or a dynamic key."""
Key: typing.TypeAlias = KeyName | KeyPrivate | KeyExpression | KeyNamedExpression


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
        destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "namedExpression":
        writer.write_unsigned(3)
        encode_name(writer, value.name)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.key)
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
        expression = destack._generated.js.tree.node.decode_local_node_id(reader)

        return KeyExpression(expression=expression)
    elif variant == 3:
        name = decode_name(reader)
        key = destack._generated.js.tree.node.decode_local_node_id(reader)

        return KeyNamedExpression(
            name=name,
            key=key,
        )
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
            "expression": destack._generated.js.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "namedExpression":
        return {
            "kind": "namedExpression",
            "name": to_json_name(value.name),
            "key": destack._generated.js.tree.node.to_json_local_node_id(value.key),
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
            expression=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            )
        )
    elif kind == "namedExpression":
        return KeyNamedExpression(
            name=from_json_name(json_field(object_, "name")),
            key=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "key")
            ),
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
