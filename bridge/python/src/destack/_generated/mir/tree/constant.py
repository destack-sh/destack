# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.tree.type


@dataclass(frozen=True, slots=True)
class ConstantNull:
    """Null reference constant."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


@dataclass(frozen=True, slots=True)
class ConstantBoolean:
    """Boolean constant."""

    # the boolean value
    value: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


@dataclass(frozen=True, slots=True)
class ConstantInt:
    """Signed integer constant."""

    # the value, sign-extended to 128 bits
    value: int
    # the bit width of the integer type
    width: int
    # whether this represents a signed integer type
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


@dataclass(frozen=True, slots=True)
class ConstantUInt:
    """Unsigned integer constant."""

    # the value, zero-extended to 128 bits
    value: int
    # the bit width of the integer type
    width: int
    kind: typing.Literal["uInt"] = "uInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


@dataclass(frozen=True, slots=True)
class ConstantFloat:
    """Floating point constant."""

    # the value stored as raw bits
    bits: int
    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


@dataclass(frozen=True, slots=True)
class ConstantChar:
    """Character constant."""

    # the character value
    value: str
    kind: typing.Literal["char"] = "char"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constant(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constant(self)


"""A compile-time constant value in MIR."""
Constant: typing.TypeAlias = (
    ConstantNull
    | ConstantBoolean
    | ConstantInt
    | ConstantUInt
    | ConstantFloat
    | ConstantChar
)


def encode_constant(writer: BinaryWriter, value: Constant) -> None:
    """Encode one Constant."""
    if value.kind == "null":
        writer.write_unsigned(0)
    elif value.kind == "boolean":
        writer.write_unsigned(1)
        writer.write_bool(value.value)
    elif value.kind == "int":
        writer.write_unsigned(2)
        writer.write_signed(value.value)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "uInt":
        writer.write_unsigned(3)
        writer.write_unsigned(value.value)
        writer.write_unsigned(value.width)
    elif value.kind == "float":
        writer.write_unsigned(4)
        writer.write_unsigned(value.bits)
        destack._generated.mir.tree.type.encode_float_type(writer, value.format)
    elif value.kind == "char":
        writer.write_unsigned(5)
        writer.write_char(value.value)
    else:
        raise SerdeError("unknown enum variant")


def decode_constant(reader: BinaryReader) -> Constant:
    """Decode one Constant."""
    variant = reader.read_number()

    if variant == 0:
        return ConstantNull()
    elif variant == 1:
        value_ = reader.read_bool()

        return ConstantBoolean(
            value=value_,
        )
    elif variant == 2:
        value_ = reader.read_signed_number()
        width = reader.read_number()
        is_signed = reader.read_bool()

        return ConstantInt(
            value=value_,
            width=width,
            is_signed=is_signed,
        )
    elif variant == 3:
        value_ = reader.read_unsigned()
        width = reader.read_number()

        return ConstantUInt(
            value=value_,
            width=width,
        )
    elif variant == 4:
        bits = reader.read_number()
        format = destack._generated.mir.tree.type.decode_float_type(reader)

        return ConstantFloat(
            bits=bits,
            format=format,
        )
    elif variant == 5:
        value_ = reader.read_char()

        return ConstantChar(
            value=value_,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_constant(value: Constant) -> Json:
    """Return one JSON value for one Constant."""
    if value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
            "value": value.value,
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "value": value.value,
            "width": value.width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "uInt":
        return {
            "kind": "uInt",
            "value": value.value,
            "width": value.width,
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "bits": value.bits,
            "format": destack._generated.mir.tree.type.to_json_float_type(value.format),
        }
    elif value.kind == "char":
        return {
            "kind": "char",
            "value": value.value,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_constant(value: Json) -> Constant:
    """Return one Constant from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "null":
        return ConstantNull()
    elif kind == "boolean":
        return ConstantBoolean(
            value=json_bool(json_field(object_, "value")),
        )
    elif kind == "int":
        return ConstantInt(
            value=json_int(json_field(object_, "value")),
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "uInt":
        return ConstantUInt(
            value=json_int(json_field(object_, "value")),
            width=json_int(json_field(object_, "width")),
        )
    elif kind == "float":
        return ConstantFloat(
            bits=json_int(json_field(object_, "bits")),
            format=destack._generated.mir.tree.type.from_json_float_type(
                json_field(object_, "format")
            ),
        )
    elif kind == "char":
        return ConstantChar(
            value=json_string(json_field(object_, "value")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Constant",
    "encode_constant",
    "decode_constant",
    "to_json_constant",
    "from_json_constant",
    "ConstantNull",
    "ConstantBoolean",
    "ConstantInt",
    "ConstantUInt",
    "ConstantFloat",
    "ConstantChar",
]
