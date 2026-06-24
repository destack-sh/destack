# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.tree.attribute import (
    FloatValueImpl,
)

import destack._generated.core.string
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class Attribute:
    """A metadata attribute attached to a MIR node."""

    # the attribute name
    name: AttributeIdentifier
    # the attribute arguments
    args: AttributeArgs

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Attribute: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Attribute: ...

def encode_attribute(writer: BinaryWriter, value: Attribute) -> None: ...
def decode_attribute(reader: BinaryReader) -> Attribute: ...
def to_json_attribute(value: Attribute) -> Json: ...
def from_json_attribute(value: Json) -> Attribute: ...

@dataclass(frozen=True, slots=True)
class AttributeIdentifierIdentifier:
    """One concrete identifier."""

    identifier: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeIdentifierMissing:
    """One required identifier that was omitted."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeIdentifierError:
    """One malformed identifier fragment."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One identifier inside attribute syntax."""
AttributeIdentifier: typing.TypeAlias = (
    AttributeIdentifierIdentifier
    | AttributeIdentifierMissing
    | AttributeIdentifierError
)

def encode_attribute_identifier(
    writer: BinaryWriter, value: AttributeIdentifier
) -> None: ...
def decode_attribute_identifier(reader: BinaryReader) -> AttributeIdentifier: ...
def to_json_attribute_identifier(value: AttributeIdentifier) -> Json: ...
def from_json_attribute_identifier(value: Json) -> AttributeIdentifier: ...

@dataclass(frozen=True, slots=True)
class AttributeArgsNone:
    """No arguments were provided."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeArgsValue:
    """A single unnamed value was provided."""

    value: AttributeValue
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeArgsValues:
    """Multiple unnamed values were provided."""

    values: Sequence[AttributeValue]
    kind: typing.Literal["values"] = "values"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeArgsKeyValues:
    """Named arguments were provided as key-value pairs."""

    key_values: Sequence[AttributeKeyValue]
    kind: typing.Literal["keyValues"] = "keyValues"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Arguments for a MIR attribute."""
AttributeArgs: typing.TypeAlias = (
    AttributeArgsNone
    | AttributeArgsValue
    | AttributeArgsValues
    | AttributeArgsKeyValues
)

def encode_attribute_args(writer: BinaryWriter, value: AttributeArgs) -> None: ...
def decode_attribute_args(reader: BinaryReader) -> AttributeArgs: ...
def to_json_attribute_args(value: AttributeArgs) -> Json: ...
def from_json_attribute_args(value: Json) -> AttributeArgs: ...

@dataclass(frozen=True, slots=True)
class AttributeValueIdentifier:
    """An identifier value."""

    identifier: AttributeIdentifier
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueType:
    """A type value."""

    type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueInteger:
    """An integer literal."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueFloat:
    """A floating point literal."""

    float: FloatValue
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueBoolean:
    """A boolean literal."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueString:
    """A string literal."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueList:
    """A list of values."""

    list: Sequence[AttributeValue]
    kind: typing.Literal["list"] = "list"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueMissing:
    """One required value that was omitted."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AttributeValueError:
    """One malformed value fragment."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A value inside an attribute argument list."""
AttributeValue: typing.TypeAlias = (
    AttributeValueIdentifier
    | AttributeValueType
    | AttributeValueInteger
    | AttributeValueFloat
    | AttributeValueBoolean
    | AttributeValueString
    | AttributeValueList
    | AttributeValueMissing
    | AttributeValueError
)

def encode_attribute_value(writer: BinaryWriter, value: AttributeValue) -> None: ...
def decode_attribute_value(reader: BinaryReader) -> AttributeValue: ...
def to_json_attribute_value(value: AttributeValue) -> Json: ...
def from_json_attribute_value(value: Json) -> AttributeValue: ...

@dataclass(frozen=True, slots=True)
class FloatValue(FloatValueImpl):
    """A floating point literal stored by bit pattern."""

    # the IEEE-754 bits for the value
    bits: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FloatValue: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FloatValue: ...

def encode_float_value(writer: BinaryWriter, value: FloatValue) -> None: ...
def decode_float_value(reader: BinaryReader) -> FloatValue: ...
def to_json_float_value(value: FloatValue) -> Json: ...
def from_json_float_value(value: Json) -> FloatValue: ...

@dataclass(frozen=True, slots=True)
class AttributeKeyValue:
    """A key-value pair within an attribute argument list."""

    # the argument name
    key: AttributeIdentifier
    # the argument value
    value: AttributeValue

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AttributeKeyValue: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AttributeKeyValue: ...

def encode_attribute_key_value(
    writer: BinaryWriter, value: AttributeKeyValue
) -> None: ...
def decode_attribute_key_value(reader: BinaryReader) -> AttributeKeyValue: ...
def to_json_attribute_key_value(value: AttributeKeyValue) -> Json: ...
def from_json_attribute_key_value(value: Json) -> AttributeKeyValue: ...

__all__ = [
    "Attribute",
    "encode_attribute",
    "decode_attribute",
    "to_json_attribute",
    "from_json_attribute",
    "AttributeIdentifier",
    "encode_attribute_identifier",
    "decode_attribute_identifier",
    "to_json_attribute_identifier",
    "from_json_attribute_identifier",
    "AttributeIdentifierIdentifier",
    "AttributeIdentifierMissing",
    "AttributeIdentifierError",
    "AttributeArgs",
    "encode_attribute_args",
    "decode_attribute_args",
    "to_json_attribute_args",
    "from_json_attribute_args",
    "AttributeArgsNone",
    "AttributeArgsValue",
    "AttributeArgsValues",
    "AttributeArgsKeyValues",
    "AttributeValue",
    "encode_attribute_value",
    "decode_attribute_value",
    "to_json_attribute_value",
    "from_json_attribute_value",
    "AttributeValueIdentifier",
    "AttributeValueType",
    "AttributeValueInteger",
    "AttributeValueFloat",
    "AttributeValueBoolean",
    "AttributeValueString",
    "AttributeValueList",
    "AttributeValueMissing",
    "AttributeValueError",
    "FloatValue",
    "encode_float_value",
    "decode_float_value",
    "to_json_float_value",
    "from_json_float_value",
    "AttributeKeyValue",
    "encode_attribute_key_value",
    "decode_attribute_key_value",
    "to_json_attribute_key_value",
    "from_json_attribute_key_value",
]
