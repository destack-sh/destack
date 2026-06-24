# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Binary arithmetic/logic operator."""
BinaryOperator: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["subtract"]
    | typing.Literal["multiply"]
    | typing.Literal["signedDivide"]
    | typing.Literal["unsignedDivide"]
    | typing.Literal["signedRemainder"]
    | typing.Literal["unsignedRemainder"]
    | typing.Literal["floatAdd"]
    | typing.Literal["floatSubtract"]
    | typing.Literal["floatMultiply"]
    | typing.Literal["floatDivide"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["arithmeticShiftRight"]
    | typing.Literal["logicalShiftRight"]
    | typing.Literal["equal"]
    | typing.Literal["notEqual"]
    | typing.Literal["signedLessThan"]
    | typing.Literal["signedLessEqual"]
    | typing.Literal["signedGreaterThan"]
    | typing.Literal["signedGreaterEqual"]
    | typing.Literal["unsignedLessThan"]
    | typing.Literal["unsignedLessEqual"]
    | typing.Literal["unsignedGreaterThan"]
    | typing.Literal["unsignedGreaterEqual"]
    | typing.Literal["floatEqual"]
    | typing.Literal["floatNotEqual"]
    | typing.Literal["floatLessThan"]
    | typing.Literal["floatLessEqual"]
    | typing.Literal["floatGreaterThan"]
    | typing.Literal["floatGreaterEqual"]
)

def encode_binary_operator(writer: BinaryWriter, value: BinaryOperator) -> None: ...
def decode_binary_operator(reader: BinaryReader) -> BinaryOperator: ...
def to_json_binary_operator(value: BinaryOperator) -> Json: ...
def from_json_binary_operator(value: Json) -> BinaryOperator: ...

"""Unary operator."""
UnaryOperator: typing.TypeAlias = (
    typing.Literal["negate"] | typing.Literal["floatNegate"] | typing.Literal["not"]
)

def encode_unary_operator(writer: BinaryWriter, value: UnaryOperator) -> None: ...
def decode_unary_operator(reader: BinaryReader) -> UnaryOperator: ...
def to_json_unary_operator(value: UnaryOperator) -> Json: ...
def from_json_unary_operator(value: Json) -> UnaryOperator: ...

__all__ = [
    "BinaryOperator",
    "encode_binary_operator",
    "decode_binary_operator",
    "to_json_binary_operator",
    "from_json_binary_operator",
    "UnaryOperator",
    "encode_unary_operator",
    "decode_unary_operator",
    "to_json_unary_operator",
    "from_json_unary_operator",
]
