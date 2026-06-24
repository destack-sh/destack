# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""The end-bound spelling of one range."""
RangeEnd: typing.TypeAlias = typing.Literal["open"] | typing.Literal["inclusive"]

def encode_range_end(writer: BinaryWriter, value: RangeEnd) -> None: ...
def decode_range_end(reader: BinaryReader) -> RangeEnd: ...
def to_json_range_end(value: RangeEnd) -> Json: ...
def from_json_range_end(value: Json) -> RangeEnd: ...

"""A UnaryOperator is unary operator."""
UnaryOperator: typing.TypeAlias = (
    typing.Literal["postIncrement"]
    | typing.Literal["postDecrement"]
    | typing.Literal["preIncrement"]
    | typing.Literal["preDecrement"]
    | typing.Literal["not"]
    | typing.Literal["plus"]
    | typing.Literal["negate"]
    | typing.Literal["elementwiseNot"]
    | typing.Literal["typeof"]
    | typing.Literal["void"]
    | typing.Literal["dereference"]
    | typing.Literal["spread"]
)

def encode_unary_operator(writer: BinaryWriter, value: UnaryOperator) -> None: ...
def decode_unary_operator(reader: BinaryReader) -> UnaryOperator: ...
def to_json_unary_operator(value: UnaryOperator) -> Json: ...
def from_json_unary_operator(value: Json) -> UnaryOperator: ...

"""A BinaryOperator is an infix binary operator."""
BinaryOperator: typing.TypeAlias = (
    typing.Literal["exponent"]
    | typing.Literal["multiply"]
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
)

def encode_binary_operator(writer: BinaryWriter, value: BinaryOperator) -> None: ...
def decode_binary_operator(reader: BinaryReader) -> BinaryOperator: ...
def to_json_binary_operator(value: BinaryOperator) -> Json: ...
def from_json_binary_operator(value: Json) -> BinaryOperator: ...

"""An AssignOperator is an assignment type."""
AssignOperator: typing.TypeAlias = (
    typing.Literal["assign"]
    | typing.Literal["multiplyAssign"]
    | typing.Literal["exponentAssign"]
    | typing.Literal["divideAssign"]
    | typing.Literal["remainderAssign"]
    | typing.Literal["addAssign"]
    | typing.Literal["subtractAssign"]
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

__all__ = [
    "RangeEnd",
    "encode_range_end",
    "decode_range_end",
    "to_json_range_end",
    "from_json_range_end",
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
    "AssignOperator",
    "encode_assign_operator",
    "decode_assign_operator",
    "to_json_assign_operator",
    "from_json_assign_operator",
]
