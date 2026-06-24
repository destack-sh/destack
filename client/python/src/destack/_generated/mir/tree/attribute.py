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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Attribute:
        """Decode one Attribute."""
        return decode_attribute(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute(self)

    @classmethod
    def from_json(cls, value: Json) -> Attribute:
        """Return one Attribute from one JSON value."""
        return from_json_attribute(value)


def encode_attribute(writer: BinaryWriter, value: Attribute) -> None:
    """Encode one Attribute."""
    encode_attribute_identifier(writer, value.name)
    encode_attribute_args(writer, value.args)


def decode_attribute(reader: BinaryReader) -> Attribute:
    """Decode one Attribute."""
    name = decode_attribute_identifier(reader)
    args = decode_attribute_args(reader)

    return Attribute(
        name=name,
        args=args,
    )


def to_json_attribute(value: Attribute) -> Json:
    """Return one JSON value for one Attribute."""
    return {
        "name": to_json_attribute_identifier(value.name),
        "args": to_json_attribute_args(value.args),
    }


def from_json_attribute(value: Json) -> Attribute:
    """Return one Attribute from one JSON value."""
    object_ = json_object(value)

    return Attribute(
        name=from_json_attribute_identifier(json_field(object_, "name")),
        args=from_json_attribute_args(json_field(object_, "args")),
    )


@dataclass(frozen=True, slots=True)
class AttributeIdentifierIdentifier:
    """One concrete identifier."""

    identifier: destack._generated.core.string.StringId
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_identifier(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_identifier(self)


@dataclass(frozen=True, slots=True)
class AttributeIdentifierMissing:
    """One required identifier that was omitted."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_identifier(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_identifier(self)


@dataclass(frozen=True, slots=True)
class AttributeIdentifierError:
    """One malformed identifier fragment."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_identifier(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_identifier(self)


"""One identifier inside attribute syntax."""
AttributeIdentifier: typing.TypeAlias = (
    AttributeIdentifierIdentifier
    | AttributeIdentifierMissing
    | AttributeIdentifierError
)


def encode_attribute_identifier(
    writer: BinaryWriter, value: AttributeIdentifier
) -> None:
    """Encode one AttributeIdentifier."""
    if value.kind == "identifier":
        writer.write_unsigned(0)
        destack._generated.core.string.encode_string_id(writer, value.identifier)
    elif value.kind == "missing":
        writer.write_unsigned(1)
    elif value.kind == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_attribute_identifier(reader: BinaryReader) -> AttributeIdentifier:
    """Decode one AttributeIdentifier."""
    variant = reader.read_number()

    if variant == 0:
        identifier = destack._generated.core.string.decode_string_id(reader)

        return AttributeIdentifierIdentifier(identifier=identifier)
    elif variant == 1:
        return AttributeIdentifierMissing()
    elif variant == 2:
        return AttributeIdentifierError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_attribute_identifier(value: AttributeIdentifier) -> Json:
    """Return one JSON value for one AttributeIdentifier."""
    if value.kind == "identifier":
        return {
            "kind": "identifier",
            "identifier": destack._generated.core.string.to_json_string_id(
                value.identifier
            ),
        }
    elif value.kind == "missing":
        return {
            "kind": "missing",
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_attribute_identifier(value: Json) -> AttributeIdentifier:
    """Return one AttributeIdentifier from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "identifier":
        return AttributeIdentifierIdentifier(
            identifier=destack._generated.core.string.from_json_string_id(
                json_field(object_, "identifier")
            )
        )
    elif kind == "missing":
        return AttributeIdentifierMissing()
    elif kind == "error":
        return AttributeIdentifierError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AttributeArgsNone:
    """No arguments were provided."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_args(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_args(self)


@dataclass(frozen=True, slots=True)
class AttributeArgsValue:
    """A single unnamed value was provided."""

    value: AttributeValue
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_args(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_args(self)


@dataclass(frozen=True, slots=True)
class AttributeArgsValues:
    """Multiple unnamed values were provided."""

    values: Sequence[AttributeValue]
    kind: typing.Literal["values"] = "values"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_args(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_args(self)


@dataclass(frozen=True, slots=True)
class AttributeArgsKeyValues:
    """Named arguments were provided as key-value pairs."""

    key_values: Sequence[AttributeKeyValue]
    kind: typing.Literal["keyValues"] = "keyValues"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_args(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_args(self)


"""Arguments for a MIR attribute."""
AttributeArgs: typing.TypeAlias = (
    AttributeArgsNone
    | AttributeArgsValue
    | AttributeArgsValues
    | AttributeArgsKeyValues
)


def encode_attribute_args(writer: BinaryWriter, value: AttributeArgs) -> None:
    """Encode one AttributeArgs."""
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "value":
        writer.write_unsigned(1)
        encode_attribute_value(writer, value.value)
    elif value.kind == "values":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.values))
        for item_value_values_0 in value.values:
            encode_attribute_value(writer, item_value_values_0)
    elif value.kind == "keyValues":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.key_values))
        for item_value_key_values_0 in value.key_values:
            encode_attribute_key_value(writer, item_value_key_values_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_attribute_args(reader: BinaryReader) -> AttributeArgs:
    """Decode one AttributeArgs."""
    variant = reader.read_number()

    if variant == 0:
        return AttributeArgsNone()
    elif variant == 1:
        value_ = decode_attribute_value(reader)

        return AttributeArgsValue(value=value_)
    elif variant == 2:
        values = [decode_attribute_value(reader) for _ in range(reader.read_number())]

        return AttributeArgsValues(values=values)
    elif variant == 3:
        key_values = [
            decode_attribute_key_value(reader) for _ in range(reader.read_number())
        ]

        return AttributeArgsKeyValues(key_values=key_values)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_attribute_args(value: AttributeArgs) -> Json:
    """Return one JSON value for one AttributeArgs."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "value":
        return {
            "kind": "value",
            "value": to_json_attribute_value(value.value),
        }
    elif value.kind == "values":
        return {
            "kind": "values",
            "values": [to_json_attribute_value(item_0) for item_0 in value.values],
        }
    elif value.kind == "keyValues":
        return {
            "kind": "keyValues",
            "key_values": [
                to_json_attribute_key_value(item_0) for item_0 in value.key_values
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_attribute_args(value: Json) -> AttributeArgs:
    """Return one AttributeArgs from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return AttributeArgsNone()
    elif kind == "value":
        return AttributeArgsValue(
            value=from_json_attribute_value(json_field(object_, "value"))
        )
    elif kind == "values":
        return AttributeArgsValues(
            values=[
                from_json_attribute_value(item_0)
                for item_0 in json_array(json_field(object_, "values"))
            ]
        )
    elif kind == "keyValues":
        return AttributeArgsKeyValues(
            key_values=[
                from_json_attribute_key_value(item_0)
                for item_0 in json_array(json_field(object_, "key_values"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AttributeValueIdentifier:
    """An identifier value."""

    identifier: AttributeIdentifier
    kind: typing.Literal["identifier"] = "identifier"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueType:
    """A type value."""

    type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueInteger:
    """An integer literal."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueFloat:
    """A floating point literal."""

    float: FloatValue
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueBoolean:
    """A boolean literal."""

    boolean: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueString:
    """A string literal."""

    string: destack._generated.core.string.StringId
    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueList:
    """A list of values."""

    list: Sequence[AttributeValue]
    kind: typing.Literal["list"] = "list"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueMissing:
    """One required value that was omitted."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


@dataclass(frozen=True, slots=True)
class AttributeValueError:
    """One malformed value fragment."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_value(self)


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


def encode_attribute_value(writer: BinaryWriter, value: AttributeValue) -> None:
    """Encode one AttributeValue."""
    if value.kind == "identifier":
        writer.write_unsigned(0)
        encode_attribute_identifier(writer, value.identifier)
    elif value.kind == "type":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.type)
    elif value.kind == "integer":
        writer.write_unsigned(2)
        writer.write_signed(value.integer)
    elif value.kind == "float":
        writer.write_unsigned(3)
        encode_float_value(writer, value.float)
    elif value.kind == "boolean":
        writer.write_unsigned(4)
        writer.write_bool(value.boolean)
    elif value.kind == "string":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.string)
    elif value.kind == "list":
        writer.write_unsigned(6)
        writer.write_unsigned(len(value.list))
        for item_value_list_0 in value.list:
            encode_attribute_value(writer, item_value_list_0)
    elif value.kind == "missing":
        writer.write_unsigned(7)
    elif value.kind == "error":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_attribute_value(reader: BinaryReader) -> AttributeValue:
    """Decode one AttributeValue."""
    variant = reader.read_number()

    if variant == 0:
        identifier = decode_attribute_identifier(reader)

        return AttributeValueIdentifier(identifier=identifier)
    elif variant == 1:
        type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return AttributeValueType(type=type)
    elif variant == 2:
        integer = reader.read_signed_number()

        return AttributeValueInteger(integer=integer)
    elif variant == 3:
        float = decode_float_value(reader)

        return AttributeValueFloat(float=float)
    elif variant == 4:
        boolean = reader.read_bool()

        return AttributeValueBoolean(boolean=boolean)
    elif variant == 5:
        string = destack._generated.core.string.decode_string_id(reader)

        return AttributeValueString(string=string)
    elif variant == 6:
        list = [decode_attribute_value(reader) for _ in range(reader.read_number())]

        return AttributeValueList(list=list)
    elif variant == 7:
        return AttributeValueMissing()
    elif variant == 8:
        return AttributeValueError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_attribute_value(value: AttributeValue) -> Json:
    """Return one JSON value for one AttributeValue."""
    if value.kind == "identifier":
        return {
            "kind": "identifier",
            "identifier": to_json_attribute_identifier(value.identifier),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "type": destack._generated.mir.tree.node.to_json_local_node_id(value.type),
        }
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": value.integer,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "float": to_json_float_value(value.float),
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
            "boolean": value.boolean,
        }
    elif value.kind == "string":
        return {
            "kind": "string",
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    elif value.kind == "list":
        return {
            "kind": "list",
            "list": [to_json_attribute_value(item_0) for item_0 in value.list],
        }
    elif value.kind == "missing":
        return {
            "kind": "missing",
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_attribute_value(value: Json) -> AttributeValue:
    """Return one AttributeValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "identifier":
        return AttributeValueIdentifier(
            identifier=from_json_attribute_identifier(json_field(object_, "identifier"))
        )
    elif kind == "type":
        return AttributeValueType(
            type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "type")
            )
        )
    elif kind == "integer":
        return AttributeValueInteger(integer=json_int(json_field(object_, "integer")))
    elif kind == "float":
        return AttributeValueFloat(
            float=from_json_float_value(json_field(object_, "float"))
        )
    elif kind == "boolean":
        return AttributeValueBoolean(boolean=json_bool(json_field(object_, "boolean")))
    elif kind == "string":
        return AttributeValueString(
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            )
        )
    elif kind == "list":
        return AttributeValueList(
            list=[
                from_json_attribute_value(item_0)
                for item_0 in json_array(json_field(object_, "list"))
            ]
        )
    elif kind == "missing":
        return AttributeValueMissing()
    elif kind == "error":
        return AttributeValueError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class FloatValue(FloatValueImpl):
    """A floating point literal stored by bit pattern."""

    # the IEEE-754 bits for the value
    bits: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_float_value(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FloatValue:
        """Decode one FloatValue."""
        return decode_float_value(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_float_value(self)

    @classmethod
    def from_json(cls, value: Json) -> FloatValue:
        """Return one FloatValue from one JSON value."""
        return from_json_float_value(value)


def encode_float_value(writer: BinaryWriter, value: FloatValue) -> None:
    """Encode one FloatValue."""
    writer.write_unsigned(value.bits)


def decode_float_value(reader: BinaryReader) -> FloatValue:
    """Decode one FloatValue."""
    bits = reader.read_number()

    return FloatValue(
        bits=bits,
    )


def to_json_float_value(value: FloatValue) -> Json:
    """Return one JSON value for one FloatValue."""
    return {
        "bits": value.bits,
    }


def from_json_float_value(value: Json) -> FloatValue:
    """Return one FloatValue from one JSON value."""
    object_ = json_object(value)

    return FloatValue(
        bits=json_int(json_field(object_, "bits")),
    )


@dataclass(frozen=True, slots=True)
class AttributeKeyValue:
    """A key-value pair within an attribute argument list."""

    # the argument name
    key: AttributeIdentifier
    # the argument value
    value: AttributeValue

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_attribute_key_value(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AttributeKeyValue:
        """Decode one AttributeKeyValue."""
        return decode_attribute_key_value(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_attribute_key_value(self)

    @classmethod
    def from_json(cls, value: Json) -> AttributeKeyValue:
        """Return one AttributeKeyValue from one JSON value."""
        return from_json_attribute_key_value(value)


def encode_attribute_key_value(writer: BinaryWriter, value: AttributeKeyValue) -> None:
    """Encode one AttributeKeyValue."""
    encode_attribute_identifier(writer, value.key)
    encode_attribute_value(writer, value.value)


def decode_attribute_key_value(reader: BinaryReader) -> AttributeKeyValue:
    """Decode one AttributeKeyValue."""
    key = decode_attribute_identifier(reader)
    value_ = decode_attribute_value(reader)

    return AttributeKeyValue(
        key=key,
        value=value_,
    )


def to_json_attribute_key_value(value: AttributeKeyValue) -> Json:
    """Return one JSON value for one AttributeKeyValue."""
    return {
        "key": to_json_attribute_identifier(value.key),
        "value": to_json_attribute_value(value.value),
    }


def from_json_attribute_key_value(value: Json) -> AttributeKeyValue:
    """Return one AttributeKeyValue from one JSON value."""
    object_ = json_object(value)

    return AttributeKeyValue(
        key=from_json_attribute_identifier(json_field(object_, "key")),
        value=from_json_attribute_value(json_field(object_, "value")),
    )


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
