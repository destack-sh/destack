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


@dataclass(frozen=True, slots=True)
class IntegerTypeInteger:
    """The signed or unsigned integer family, `int` or `uint`."""

    is_signed: bool
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_integer_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_integer_type(self)


@dataclass(frozen=True, slots=True)
class IntegerTypeFixed:
    """A fixed-width signed or unsigned integer, like `int32` or `uint8`."""

    width: int
    is_signed: bool
    kind: typing.Literal["fixed"] = "fixed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_integer_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_integer_type(self)


@dataclass(frozen=True, slots=True)
class IntegerTypePointer:
    """A pointer-sized signed or unsigned integer, `isize` or `usize`."""

    is_signed: bool
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_integer_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_integer_type(self)


"""An integer type."""
IntegerType: typing.TypeAlias = (
    IntegerTypeInteger | IntegerTypeFixed | IntegerTypePointer
)


def encode_integer_type(writer: BinaryWriter, value: IntegerType) -> None:
    """Encode one IntegerType."""
    if value.kind == "integer":
        writer.write_unsigned(0)
        writer.write_bool(value.is_signed)
    elif value.kind == "fixed":
        writer.write_unsigned(1)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "pointer":
        writer.write_unsigned(2)
        writer.write_bool(value.is_signed)
    else:
        raise SerdeError("unknown enum variant")


def decode_integer_type(reader: BinaryReader) -> IntegerType:
    """Decode one IntegerType."""
    variant = reader.read_number()

    if variant == 0:
        is_signed = reader.read_bool()

        return IntegerTypeInteger(
            is_signed=is_signed,
        )
    elif variant == 1:
        width = reader.read_number()
        is_signed = reader.read_bool()

        return IntegerTypeFixed(
            width=width,
            is_signed=is_signed,
        )
    elif variant == 2:
        is_signed = reader.read_bool()

        return IntegerTypePointer(
            is_signed=is_signed,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_integer_type(value: IntegerType) -> Json:
    """Return one JSON value for one IntegerType."""
    if value.kind == "integer":
        return {
            "kind": "integer",
            "isSigned": value.is_signed,
        }
    elif value.kind == "fixed":
        return {
            "kind": "fixed",
            "width": value.width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "pointer":
        return {
            "kind": "pointer",
            "isSigned": value.is_signed,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_integer_type(value: Json) -> IntegerType:
    """Return one IntegerType from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "integer":
        return IntegerTypeInteger(
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "fixed":
        return IntegerTypeFixed(
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "pointer":
        return IntegerTypePointer(
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""A floating-point type."""
FloatType: typing.TypeAlias = (
    typing.Literal["float"]
    | typing.Literal["float16"]
    | typing.Literal["bfloat16"]
    | typing.Literal["float32"]
    | typing.Literal["float64"]
)


def encode_float_type(writer: BinaryWriter, value: FloatType) -> None:
    """Encode one FloatType."""
    if value == "float":
        writer.write_unsigned(0)
    elif value == "float16":
        writer.write_unsigned(1)
    elif value == "bfloat16":
        writer.write_unsigned(2)
    elif value == "float32":
        writer.write_unsigned(3)
    elif value == "float64":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_float_type(reader: BinaryReader) -> FloatType:
    """Decode one FloatType."""
    variant = reader.read_number()

    if variant == 0:
        return "float"
    elif variant == 1:
        return "float16"
    elif variant == 2:
        return "bfloat16"
    elif variant == 3:
        return "float32"
    elif variant == 4:
        return "float64"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_float_type(value: FloatType) -> Json:
    """Return one JSON value for one FloatType."""
    return value


def from_json_float_type(value: Json) -> FloatType:
    """Return one FloatType from one JSON value."""
    variant = json_string(value)

    if variant == "float":
        return "float"
    elif variant == "float16":
        return "float16"
    elif variant == "bfloat16":
        return "bfloat16"
    elif variant == "float32":
        return "float32"
    elif variant == "float64":
        return "float64"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class PrimitiveTypeBoolean:
    """Boolean type `boolean`."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeCharacter:
    """Character type `char`."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeString:
    """String type `string`."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeBigint:
    """Bigint type `bigint`."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeInteger:
    """Integer type, like `int32`, `uint8`, or `usize`."""

    integer: IntegerType
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeFloat:
    """Float type, like `float64` or `bfloat16`."""

    float: FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeSymbol:
    """Symbol type `symbol`."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


@dataclass(frozen=True, slots=True)
class PrimitiveTypeUniqueSymbol:
    """Unique symbol type `unique symbol`."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_primitive_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_primitive_type(self)


"""A primitive type."""
PrimitiveType: typing.TypeAlias = (
    PrimitiveTypeBoolean
    | PrimitiveTypeCharacter
    | PrimitiveTypeString
    | PrimitiveTypeBigint
    | PrimitiveTypeInteger
    | PrimitiveTypeFloat
    | PrimitiveTypeSymbol
    | PrimitiveTypeUniqueSymbol
)


def encode_primitive_type(writer: BinaryWriter, value: PrimitiveType) -> None:
    """Encode one PrimitiveType."""
    if value.kind == "boolean":
        writer.write_unsigned(0)
    elif value.kind == "character":
        writer.write_unsigned(1)
    elif value.kind == "string":
        writer.write_unsigned(2)
    elif value.kind == "bigint":
        writer.write_unsigned(3)
    elif value.kind == "integer":
        writer.write_unsigned(4)
        encode_integer_type(writer, value.integer)
    elif value.kind == "float":
        writer.write_unsigned(5)
        encode_float_type(writer, value.float)
    elif value.kind == "symbol":
        writer.write_unsigned(6)
    elif value.kind == "uniqueSymbol":
        writer.write_unsigned(7)
    else:
        raise SerdeError("unknown enum variant")


def decode_primitive_type(reader: BinaryReader) -> PrimitiveType:
    """Decode one PrimitiveType."""
    variant = reader.read_number()

    if variant == 0:
        return PrimitiveTypeBoolean()
    elif variant == 1:
        return PrimitiveTypeCharacter()
    elif variant == 2:
        return PrimitiveTypeString()
    elif variant == 3:
        return PrimitiveTypeBigint()
    elif variant == 4:
        integer = decode_integer_type(reader)

        return PrimitiveTypeInteger(integer=integer)
    elif variant == 5:
        float = decode_float_type(reader)

        return PrimitiveTypeFloat(float=float)
    elif variant == 6:
        return PrimitiveTypeSymbol()
    elif variant == 7:
        return PrimitiveTypeUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_primitive_type(value: PrimitiveType) -> Json:
    """Return one JSON value for one PrimitiveType."""
    if value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    elif value.kind == "character":
        return {
            "kind": "character",
        }
    elif value.kind == "string":
        return {
            "kind": "string",
        }
    elif value.kind == "bigint":
        return {
            "kind": "bigint",
        }
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": to_json_integer_type(value.integer),
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "float": to_json_float_type(value.float),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
        }
    elif value.kind == "uniqueSymbol":
        return {
            "kind": "uniqueSymbol",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_primitive_type(value: Json) -> PrimitiveType:
    """Return one PrimitiveType from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "boolean":
        return PrimitiveTypeBoolean()
    elif kind == "character":
        return PrimitiveTypeCharacter()
    elif kind == "string":
        return PrimitiveTypeString()
    elif kind == "bigint":
        return PrimitiveTypeBigint()
    elif kind == "integer":
        return PrimitiveTypeInteger(
            integer=from_json_integer_type(json_field(object_, "integer"))
        )
    elif kind == "float":
        return PrimitiveTypeFloat(
            float=from_json_float_type(json_field(object_, "float"))
        )
    elif kind == "symbol":
        return PrimitiveTypeSymbol()
    elif kind == "uniqueSymbol":
        return PrimitiveTypeUniqueSymbol()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "IntegerType",
    "encode_integer_type",
    "decode_integer_type",
    "to_json_integer_type",
    "from_json_integer_type",
    "IntegerTypeInteger",
    "IntegerTypeFixed",
    "IntegerTypePointer",
    "FloatType",
    "encode_float_type",
    "decode_float_type",
    "to_json_float_type",
    "from_json_float_type",
    "PrimitiveType",
    "encode_primitive_type",
    "decode_primitive_type",
    "to_json_primitive_type",
    "from_json_primitive_type",
    "PrimitiveTypeBoolean",
    "PrimitiveTypeCharacter",
    "PrimitiveTypeString",
    "PrimitiveTypeBigint",
    "PrimitiveTypeInteger",
    "PrimitiveTypeFloat",
    "PrimitiveTypeSymbol",
    "PrimitiveTypeUniqueSymbol",
]
