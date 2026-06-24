# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.type

@dataclass(frozen=True, slots=True)
class ConstantNull:
    """Null reference constant."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConstantBoolean:
    """Boolean constant."""

    # the boolean value
    value: bool
    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConstantUInt:
    """Unsigned integer constant."""

    # the value, zero-extended to 128 bits
    value: int
    # the bit width of the integer type
    width: int
    kind: typing.Literal["uInt"] = "uInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConstantFloat:
    """Floating point constant."""

    # the value stored as raw bits
    bits: int
    # the concrete float format
    format: destack._generated.mir.tree.type.FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConstantChar:
    """Character constant."""

    # the character value
    value: str
    kind: typing.Literal["char"] = "char"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A compile-time constant value in MIR."""
Constant: typing.TypeAlias = (
    ConstantNull
    | ConstantBoolean
    | ConstantInt
    | ConstantUInt
    | ConstantFloat
    | ConstantChar
)

def encode_constant(writer: BinaryWriter, value: Constant) -> None: ...
def decode_constant(reader: BinaryReader) -> Constant: ...
def to_json_constant(value: Constant) -> Json: ...
def from_json_constant(value: Json) -> Constant: ...

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
