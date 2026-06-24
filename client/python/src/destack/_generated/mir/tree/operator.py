# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

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


def encode_binary_operator(writer: BinaryWriter, value: BinaryOperator) -> None:
    """Encode one BinaryOperator."""
    if value == "add":
        writer.write_unsigned(0)
    elif value == "subtract":
        writer.write_unsigned(1)
    elif value == "multiply":
        writer.write_unsigned(2)
    elif value == "signedDivide":
        writer.write_unsigned(3)
    elif value == "unsignedDivide":
        writer.write_unsigned(4)
    elif value == "signedRemainder":
        writer.write_unsigned(5)
    elif value == "unsignedRemainder":
        writer.write_unsigned(6)
    elif value == "floatAdd":
        writer.write_unsigned(7)
    elif value == "floatSubtract":
        writer.write_unsigned(8)
    elif value == "floatMultiply":
        writer.write_unsigned(9)
    elif value == "floatDivide":
        writer.write_unsigned(10)
    elif value == "and":
        writer.write_unsigned(11)
    elif value == "or":
        writer.write_unsigned(12)
    elif value == "xor":
        writer.write_unsigned(13)
    elif value == "shiftLeft":
        writer.write_unsigned(14)
    elif value == "arithmeticShiftRight":
        writer.write_unsigned(15)
    elif value == "logicalShiftRight":
        writer.write_unsigned(16)
    elif value == "equal":
        writer.write_unsigned(17)
    elif value == "notEqual":
        writer.write_unsigned(18)
    elif value == "signedLessThan":
        writer.write_unsigned(19)
    elif value == "signedLessEqual":
        writer.write_unsigned(20)
    elif value == "signedGreaterThan":
        writer.write_unsigned(21)
    elif value == "signedGreaterEqual":
        writer.write_unsigned(22)
    elif value == "unsignedLessThan":
        writer.write_unsigned(23)
    elif value == "unsignedLessEqual":
        writer.write_unsigned(24)
    elif value == "unsignedGreaterThan":
        writer.write_unsigned(25)
    elif value == "unsignedGreaterEqual":
        writer.write_unsigned(26)
    elif value == "floatEqual":
        writer.write_unsigned(27)
    elif value == "floatNotEqual":
        writer.write_unsigned(28)
    elif value == "floatLessThan":
        writer.write_unsigned(29)
    elif value == "floatLessEqual":
        writer.write_unsigned(30)
    elif value == "floatGreaterThan":
        writer.write_unsigned(31)
    elif value == "floatGreaterEqual":
        writer.write_unsigned(32)
    else:
        raise SerdeError("unknown enum variant")


def decode_binary_operator(reader: BinaryReader) -> BinaryOperator:
    """Decode one BinaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "add"
    elif variant == 1:
        return "subtract"
    elif variant == 2:
        return "multiply"
    elif variant == 3:
        return "signedDivide"
    elif variant == 4:
        return "unsignedDivide"
    elif variant == 5:
        return "signedRemainder"
    elif variant == 6:
        return "unsignedRemainder"
    elif variant == 7:
        return "floatAdd"
    elif variant == 8:
        return "floatSubtract"
    elif variant == 9:
        return "floatMultiply"
    elif variant == 10:
        return "floatDivide"
    elif variant == 11:
        return "and"
    elif variant == 12:
        return "or"
    elif variant == 13:
        return "xor"
    elif variant == 14:
        return "shiftLeft"
    elif variant == 15:
        return "arithmeticShiftRight"
    elif variant == 16:
        return "logicalShiftRight"
    elif variant == 17:
        return "equal"
    elif variant == 18:
        return "notEqual"
    elif variant == 19:
        return "signedLessThan"
    elif variant == 20:
        return "signedLessEqual"
    elif variant == 21:
        return "signedGreaterThan"
    elif variant == 22:
        return "signedGreaterEqual"
    elif variant == 23:
        return "unsignedLessThan"
    elif variant == 24:
        return "unsignedLessEqual"
    elif variant == 25:
        return "unsignedGreaterThan"
    elif variant == 26:
        return "unsignedGreaterEqual"
    elif variant == 27:
        return "floatEqual"
    elif variant == 28:
        return "floatNotEqual"
    elif variant == 29:
        return "floatLessThan"
    elif variant == 30:
        return "floatLessEqual"
    elif variant == 31:
        return "floatGreaterThan"
    elif variant == 32:
        return "floatGreaterEqual"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_binary_operator(value: BinaryOperator) -> Json:
    """Return one JSON value for one BinaryOperator."""
    return value


def from_json_binary_operator(value: Json) -> BinaryOperator:
    """Return one BinaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "add":
        return "add"
    elif variant == "subtract":
        return "subtract"
    elif variant == "multiply":
        return "multiply"
    elif variant == "signedDivide":
        return "signedDivide"
    elif variant == "unsignedDivide":
        return "unsignedDivide"
    elif variant == "signedRemainder":
        return "signedRemainder"
    elif variant == "unsignedRemainder":
        return "unsignedRemainder"
    elif variant == "floatAdd":
        return "floatAdd"
    elif variant == "floatSubtract":
        return "floatSubtract"
    elif variant == "floatMultiply":
        return "floatMultiply"
    elif variant == "floatDivide":
        return "floatDivide"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    elif variant == "xor":
        return "xor"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "arithmeticShiftRight":
        return "arithmeticShiftRight"
    elif variant == "logicalShiftRight":
        return "logicalShiftRight"
    elif variant == "equal":
        return "equal"
    elif variant == "notEqual":
        return "notEqual"
    elif variant == "signedLessThan":
        return "signedLessThan"
    elif variant == "signedLessEqual":
        return "signedLessEqual"
    elif variant == "signedGreaterThan":
        return "signedGreaterThan"
    elif variant == "signedGreaterEqual":
        return "signedGreaterEqual"
    elif variant == "unsignedLessThan":
        return "unsignedLessThan"
    elif variant == "unsignedLessEqual":
        return "unsignedLessEqual"
    elif variant == "unsignedGreaterThan":
        return "unsignedGreaterThan"
    elif variant == "unsignedGreaterEqual":
        return "unsignedGreaterEqual"
    elif variant == "floatEqual":
        return "floatEqual"
    elif variant == "floatNotEqual":
        return "floatNotEqual"
    elif variant == "floatLessThan":
        return "floatLessThan"
    elif variant == "floatLessEqual":
        return "floatLessEqual"
    elif variant == "floatGreaterThan":
        return "floatGreaterThan"
    elif variant == "floatGreaterEqual":
        return "floatGreaterEqual"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Unary operator."""
UnaryOperator: typing.TypeAlias = (
    typing.Literal["negate"] | typing.Literal["floatNegate"] | typing.Literal["not"]
)


def encode_unary_operator(writer: BinaryWriter, value: UnaryOperator) -> None:
    """Encode one UnaryOperator."""
    if value == "negate":
        writer.write_unsigned(0)
    elif value == "floatNegate":
        writer.write_unsigned(1)
    elif value == "not":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_unary_operator(reader: BinaryReader) -> UnaryOperator:
    """Decode one UnaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "negate"
    elif variant == 1:
        return "floatNegate"
    elif variant == 2:
        return "not"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_unary_operator(value: UnaryOperator) -> Json:
    """Return one JSON value for one UnaryOperator."""
    return value


def from_json_unary_operator(value: Json) -> UnaryOperator:
    """Return one UnaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "negate":
        return "negate"
    elif variant == "floatNegate":
        return "floatNegate"
    elif variant == "not":
        return "not"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
