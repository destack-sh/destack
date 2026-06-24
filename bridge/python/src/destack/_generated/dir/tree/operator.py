# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""The end-bound spelling of one range."""
RangeEnd: typing.TypeAlias = typing.Literal["open"] | typing.Literal["inclusive"]


def encode_range_end(writer: BinaryWriter, value: RangeEnd) -> None:
    """Encode one RangeEnd."""
    if value == "open":
        writer.write_unsigned(0)
    elif value == "inclusive":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_range_end(reader: BinaryReader) -> RangeEnd:
    """Decode one RangeEnd."""
    variant = reader.read_number()

    if variant == 0:
        return "open"
    elif variant == 1:
        return "inclusive"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_range_end(value: RangeEnd) -> Json:
    """Return one JSON value for one RangeEnd."""
    return value


def from_json_range_end(value: Json) -> RangeEnd:
    """Return one RangeEnd from one JSON value."""
    variant = json_string(value)

    if variant == "open":
        return "open"
    elif variant == "inclusive":
        return "inclusive"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_unary_operator(writer: BinaryWriter, value: UnaryOperator) -> None:
    """Encode one UnaryOperator."""
    if value == "postIncrement":
        writer.write_unsigned(0)
    elif value == "postDecrement":
        writer.write_unsigned(1)
    elif value == "preIncrement":
        writer.write_unsigned(2)
    elif value == "preDecrement":
        writer.write_unsigned(3)
    elif value == "not":
        writer.write_unsigned(4)
    elif value == "plus":
        writer.write_unsigned(5)
    elif value == "negate":
        writer.write_unsigned(6)
    elif value == "elementwiseNot":
        writer.write_unsigned(7)
    elif value == "typeof":
        writer.write_unsigned(8)
    elif value == "void":
        writer.write_unsigned(9)
    elif value == "dereference":
        writer.write_unsigned(10)
    elif value == "spread":
        writer.write_unsigned(11)
    else:
        raise SerdeError("unknown enum variant")


def decode_unary_operator(reader: BinaryReader) -> UnaryOperator:
    """Decode one UnaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "postIncrement"
    elif variant == 1:
        return "postDecrement"
    elif variant == 2:
        return "preIncrement"
    elif variant == 3:
        return "preDecrement"
    elif variant == 4:
        return "not"
    elif variant == 5:
        return "plus"
    elif variant == 6:
        return "negate"
    elif variant == 7:
        return "elementwiseNot"
    elif variant == 8:
        return "typeof"
    elif variant == 9:
        return "void"
    elif variant == 10:
        return "dereference"
    elif variant == 11:
        return "spread"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_unary_operator(value: UnaryOperator) -> Json:
    """Return one JSON value for one UnaryOperator."""
    return value


def from_json_unary_operator(value: Json) -> UnaryOperator:
    """Return one UnaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "postIncrement":
        return "postIncrement"
    elif variant == "postDecrement":
        return "postDecrement"
    elif variant == "preIncrement":
        return "preIncrement"
    elif variant == "preDecrement":
        return "preDecrement"
    elif variant == "not":
        return "not"
    elif variant == "plus":
        return "plus"
    elif variant == "negate":
        return "negate"
    elif variant == "elementwiseNot":
        return "elementwiseNot"
    elif variant == "typeof":
        return "typeof"
    elif variant == "void":
        return "void"
    elif variant == "dereference":
        return "dereference"
    elif variant == "spread":
        return "spread"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_binary_operator(writer: BinaryWriter, value: BinaryOperator) -> None:
    """Encode one BinaryOperator."""
    if value == "exponent":
        writer.write_unsigned(0)
    elif value == "multiply":
        writer.write_unsigned(1)
    elif value == "divide":
        writer.write_unsigned(2)
    elif value == "remainder":
        writer.write_unsigned(3)
    elif value == "add":
        writer.write_unsigned(4)
    elif value == "subtract":
        writer.write_unsigned(5)
    elif value == "shiftLeft":
        writer.write_unsigned(6)
    elif value == "shiftRight":
        writer.write_unsigned(7)
    elif value == "unsignedShiftRight":
        writer.write_unsigned(8)
    elif value == "elementwiseAnd":
        writer.write_unsigned(9)
    elif value == "elementwiseXor":
        writer.write_unsigned(10)
    elif value == "elementwiseOr":
        writer.write_unsigned(11)
    elif value == "equal":
        writer.write_unsigned(12)
    elif value == "notEqual":
        writer.write_unsigned(13)
    elif value == "equalStrict":
        writer.write_unsigned(14)
    elif value == "notEqualStrict":
        writer.write_unsigned(15)
    elif value == "lessThan":
        writer.write_unsigned(16)
    elif value == "lessThanOrEqual":
        writer.write_unsigned(17)
    elif value == "greaterThan":
        writer.write_unsigned(18)
    elif value == "greaterThanOrEqual":
        writer.write_unsigned(19)
    elif value == "and":
        writer.write_unsigned(20)
    elif value == "or":
        writer.write_unsigned(21)
    elif value == "coalesce":
        writer.write_unsigned(22)
    elif value == "in":
        writer.write_unsigned(23)
    else:
        raise SerdeError("unknown enum variant")


def decode_binary_operator(reader: BinaryReader) -> BinaryOperator:
    """Decode one BinaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "exponent"
    elif variant == 1:
        return "multiply"
    elif variant == 2:
        return "divide"
    elif variant == 3:
        return "remainder"
    elif variant == 4:
        return "add"
    elif variant == 5:
        return "subtract"
    elif variant == 6:
        return "shiftLeft"
    elif variant == 7:
        return "shiftRight"
    elif variant == 8:
        return "unsignedShiftRight"
    elif variant == 9:
        return "elementwiseAnd"
    elif variant == 10:
        return "elementwiseXor"
    elif variant == 11:
        return "elementwiseOr"
    elif variant == 12:
        return "equal"
    elif variant == 13:
        return "notEqual"
    elif variant == 14:
        return "equalStrict"
    elif variant == 15:
        return "notEqualStrict"
    elif variant == 16:
        return "lessThan"
    elif variant == 17:
        return "lessThanOrEqual"
    elif variant == 18:
        return "greaterThan"
    elif variant == 19:
        return "greaterThanOrEqual"
    elif variant == 20:
        return "and"
    elif variant == 21:
        return "or"
    elif variant == 22:
        return "coalesce"
    elif variant == 23:
        return "in"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binary_operator(value: BinaryOperator) -> Json:
    """Return one JSON value for one BinaryOperator."""
    return value


def from_json_binary_operator(value: Json) -> BinaryOperator:
    """Return one BinaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "exponent":
        return "exponent"
    elif variant == "multiply":
        return "multiply"
    elif variant == "divide":
        return "divide"
    elif variant == "remainder":
        return "remainder"
    elif variant == "add":
        return "add"
    elif variant == "subtract":
        return "subtract"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "shiftRight":
        return "shiftRight"
    elif variant == "unsignedShiftRight":
        return "unsignedShiftRight"
    elif variant == "elementwiseAnd":
        return "elementwiseAnd"
    elif variant == "elementwiseXor":
        return "elementwiseXor"
    elif variant == "elementwiseOr":
        return "elementwiseOr"
    elif variant == "equal":
        return "equal"
    elif variant == "notEqual":
        return "notEqual"
    elif variant == "equalStrict":
        return "equalStrict"
    elif variant == "notEqualStrict":
        return "notEqualStrict"
    elif variant == "lessThan":
        return "lessThan"
    elif variant == "lessThanOrEqual":
        return "lessThanOrEqual"
    elif variant == "greaterThan":
        return "greaterThan"
    elif variant == "greaterThanOrEqual":
        return "greaterThanOrEqual"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    elif variant == "coalesce":
        return "coalesce"
    elif variant == "in":
        return "in"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_assign_operator(writer: BinaryWriter, value: AssignOperator) -> None:
    """Encode one AssignOperator."""
    if value == "assign":
        writer.write_unsigned(0)
    elif value == "multiplyAssign":
        writer.write_unsigned(1)
    elif value == "exponentAssign":
        writer.write_unsigned(2)
    elif value == "divideAssign":
        writer.write_unsigned(3)
    elif value == "remainderAssign":
        writer.write_unsigned(4)
    elif value == "addAssign":
        writer.write_unsigned(5)
    elif value == "subtractAssign":
        writer.write_unsigned(6)
    elif value == "shiftLeftAssign":
        writer.write_unsigned(7)
    elif value == "shiftRightAssign":
        writer.write_unsigned(8)
    elif value == "unsignedShiftRightAssign":
        writer.write_unsigned(9)
    elif value == "elementwiseAndAssign":
        writer.write_unsigned(10)
    elif value == "elementwiseXorAssign":
        writer.write_unsigned(11)
    elif value == "elementwiseOrAssign":
        writer.write_unsigned(12)
    elif value == "andAssign":
        writer.write_unsigned(13)
    elif value == "orAssign":
        writer.write_unsigned(14)
    elif value == "coalesceAssign":
        writer.write_unsigned(15)
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_operator(reader: BinaryReader) -> AssignOperator:
    """Decode one AssignOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "assign"
    elif variant == 1:
        return "multiplyAssign"
    elif variant == 2:
        return "exponentAssign"
    elif variant == 3:
        return "divideAssign"
    elif variant == 4:
        return "remainderAssign"
    elif variant == 5:
        return "addAssign"
    elif variant == 6:
        return "subtractAssign"
    elif variant == 7:
        return "shiftLeftAssign"
    elif variant == 8:
        return "shiftRightAssign"
    elif variant == 9:
        return "unsignedShiftRightAssign"
    elif variant == 10:
        return "elementwiseAndAssign"
    elif variant == 11:
        return "elementwiseXorAssign"
    elif variant == 12:
        return "elementwiseOrAssign"
    elif variant == 13:
        return "andAssign"
    elif variant == 14:
        return "orAssign"
    elif variant == 15:
        return "coalesceAssign"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_assign_operator(value: AssignOperator) -> Json:
    """Return one JSON value for one AssignOperator."""
    return value


def from_json_assign_operator(value: Json) -> AssignOperator:
    """Return one AssignOperator from one JSON value."""
    variant = json_string(value)

    if variant == "assign":
        return "assign"
    elif variant == "multiplyAssign":
        return "multiplyAssign"
    elif variant == "exponentAssign":
        return "exponentAssign"
    elif variant == "divideAssign":
        return "divideAssign"
    elif variant == "remainderAssign":
        return "remainderAssign"
    elif variant == "addAssign":
        return "addAssign"
    elif variant == "subtractAssign":
        return "subtractAssign"
    elif variant == "shiftLeftAssign":
        return "shiftLeftAssign"
    elif variant == "shiftRightAssign":
        return "shiftRightAssign"
    elif variant == "unsignedShiftRightAssign":
        return "unsignedShiftRightAssign"
    elif variant == "elementwiseAndAssign":
        return "elementwiseAndAssign"
    elif variant == "elementwiseXorAssign":
        return "elementwiseXorAssign"
    elif variant == "elementwiseOrAssign":
        return "elementwiseOrAssign"
    elif variant == "andAssign":
        return "andAssign"
    elif variant == "orAssign":
        return "orAssign"
    elif variant == "coalesceAssign":
        return "coalesceAssign"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
