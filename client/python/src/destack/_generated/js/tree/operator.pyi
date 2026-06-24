# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Assignment operator."""
AssignOperator: typing.TypeAlias = (
    typing.Literal["addAssign"]
    | typing.Literal["subtractAssign"]
    | typing.Literal["multiplyAssign"]
    | typing.Literal["divideAssign"]
    | typing.Literal["remainderAssign"]
    | typing.Literal["exponentAssign"]
    | typing.Literal["shiftLeftAssign"]
    | typing.Literal["shiftRightAssign"]
    | typing.Literal["unsignedShiftRightAssign"]
    | typing.Literal["elementwiseAndAssign"]
    | typing.Literal["elementwiseXorAssign"]
    | typing.Literal["elementwiseOrAssign"]
    | typing.Literal["andAssign"]
    | typing.Literal["orAssign"]
    | typing.Literal["coalesceAssign"]
)

def encode_assign_operator(writer: BinaryWriter, value: AssignOperator) -> None: ...
def decode_assign_operator(reader: BinaryReader) -> AssignOperator: ...
def to_json_assign_operator(value: AssignOperator) -> Json: ...
def from_json_assign_operator(value: Json) -> AssignOperator: ...

"""Unary operator."""
UnaryOperator: typing.TypeAlias = (
    typing.Literal["postIncrement"]
    | typing.Literal["postDecrement"]
    | typing.Literal["preIncrement"]
    | typing.Literal["preDecrement"]
    | typing.Literal["plus"]
    | typing.Literal["negate"]
    | typing.Literal["elementwiseNot"]
    | typing.Literal["not"]
    | typing.Literal["typeof"]
    | typing.Literal["void"]
)

def encode_unary_operator(writer: BinaryWriter, value: UnaryOperator) -> None: ...
def decode_unary_operator(reader: BinaryReader) -> UnaryOperator: ...
def to_json_unary_operator(value: UnaryOperator) -> Json: ...
def from_json_unary_operator(value: Json) -> UnaryOperator: ...

"""Binary operator."""
BinaryOperator: typing.TypeAlias = (
    typing.Literal["multiply"]
    | typing.Literal["exponent"]
    | typing.Literal["divide"]
    | typing.Literal["remainder"]
    | typing.Literal["add"]
    | typing.Literal["subtract"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["unsignedShiftRight"]
    | typing.Literal["elementwiseAnd"]
    | typing.Literal["elementwiseXor"]
    | typing.Literal["elementwiseOr"]
    | typing.Literal["equal"]
    | typing.Literal["notEqual"]
    | typing.Literal["equalStrict"]
    | typing.Literal["notEqualStrict"]
    | typing.Literal["lessThan"]
    | typing.Literal["lessThanOrEqual"]
    | typing.Literal["greaterThan"]
    | typing.Literal["greaterThanOrEqual"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["coalesce"]
    | typing.Literal["in"]
    | typing.Literal["instanceOf"]
)

def encode_binary_operator(writer: BinaryWriter, value: BinaryOperator) -> None: ...
def decode_binary_operator(reader: BinaryReader) -> BinaryOperator: ...
def to_json_binary_operator(value: BinaryOperator) -> Json: ...
def from_json_binary_operator(value: Json) -> BinaryOperator: ...

__all__ = [
    "AssignOperator",
    "encode_assign_operator",
    "decode_assign_operator",
    "to_json_assign_operator",
    "from_json_assign_operator",
    "UnaryOperator",
    "encode_unary_operator",
    "decode_unary_operator",
    "to_json_unary_operator",
    "from_json_unary_operator",
    "BinaryOperator",
    "encode_binary_operator",
    "decode_binary_operator",
    "to_json_binary_operator",
    "from_json_binary_operator",
]
