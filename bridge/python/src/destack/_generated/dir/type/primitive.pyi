# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class IntegerTypeInteger:
    """The signed or unsigned integer family, `int` or `uint`."""

    is_signed: bool
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class IntegerTypeFixed:
    """A fixed-width signed or unsigned integer, like `int32` or `uint8`."""

    width: int
    is_signed: bool
    kind: typing.Literal["fixed"] = "fixed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class IntegerTypePointer:
    """A pointer-sized signed or unsigned integer, `isize` or `usize`."""

    is_signed: bool
    kind: typing.Literal["pointer"] = "pointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""An integer type."""
IntegerType: typing.TypeAlias = (
    IntegerTypeInteger | IntegerTypeFixed | IntegerTypePointer
)

def encode_integer_type(writer: BinaryWriter, value: IntegerType) -> None: ...
def decode_integer_type(reader: BinaryReader) -> IntegerType: ...
def to_json_integer_type(value: IntegerType) -> Json: ...
def from_json_integer_type(value: Json) -> IntegerType: ...

"""A floating-point type."""
FloatType: typing.TypeAlias = (
    typing.Literal["float"]
    | typing.Literal["float16"]
    | typing.Literal["bfloat16"]
    | typing.Literal["float32"]
    | typing.Literal["float64"]
)

def encode_float_type(writer: BinaryWriter, value: FloatType) -> None: ...
def decode_float_type(reader: BinaryReader) -> FloatType: ...
def to_json_float_type(value: FloatType) -> Json: ...
def from_json_float_type(value: Json) -> FloatType: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeBoolean:
    """Boolean type `boolean`."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeCharacter:
    """Character type `char`."""

    kind: typing.Literal["character"] = "character"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeString:
    """String type `string`."""

    kind: typing.Literal["string"] = "string"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeBigint:
    """Bigint type `bigint`."""

    kind: typing.Literal["bigint"] = "bigint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeInteger:
    """Integer type, like `int32`, `uint8`, or `usize`."""

    integer: IntegerType
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeFloat:
    """Float type, like `float64` or `bfloat16`."""

    float: FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeSymbol:
    """Symbol type `symbol`."""

    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PrimitiveTypeUniqueSymbol:
    """Unique symbol type `unique symbol`."""

    kind: typing.Literal["uniqueSymbol"] = "uniqueSymbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_primitive_type(writer: BinaryWriter, value: PrimitiveType) -> None: ...
def decode_primitive_type(reader: BinaryReader) -> PrimitiveType: ...
def to_json_primitive_type(value: PrimitiveType) -> Json: ...
def from_json_primitive_type(value: Json) -> PrimitiveType: ...

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
