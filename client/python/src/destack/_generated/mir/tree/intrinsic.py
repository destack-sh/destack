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

"""Compiler intrinsic operations."""
Intrinsic: typing.TypeAlias = (
    typing.Literal["typeOf"]
    | typing.Literal["sizeOf"]
    | typing.Literal["alignOf"]
    | typing.Literal["leadingZeroCount"]
    | typing.Literal["trailingZeroCount"]
    | typing.Literal["populationCount"]
    | typing.Literal["byteSwap"]
    | typing.Literal["bitReverse"]
    | typing.Literal["rotateLeft"]
    | typing.Literal["rotateRight"]
    | typing.Literal["addOverflow"]
    | typing.Literal["subOverflow"]
    | typing.Literal["mulOverflow"]
    | typing.Literal["addUnchecked"]
    | typing.Literal["subUnchecked"]
    | typing.Literal["mulUnchecked"]
    | typing.Literal["divUnchecked"]
    | typing.Literal["remUnchecked"]
    | typing.Literal["shlUnchecked"]
    | typing.Literal["shrUnchecked"]
    | typing.Literal["satAdd"]
    | typing.Literal["satSub"]
    | typing.Literal["memcpy"]
    | typing.Literal["memmove"]
    | typing.Literal["memset"]
    | typing.Literal["memcmp"]
    | typing.Literal["prefetchRead"]
    | typing.Literal["prefetchWrite"]
    | typing.Literal["transmute"]
    | typing.Literal["spaceCast"]
    | typing.Literal["pointerOffsetFrom"]
    | typing.Literal["rawEq"]
    | typing.Literal["sqrt"]
    | typing.Literal["abs"]
    | typing.Literal["fma"]
    | typing.Literal["copySign"]
    | typing.Literal["min"]
    | typing.Literal["max"]
    | typing.Literal["sin"]
    | typing.Literal["cos"]
    | typing.Literal["tan"]
    | typing.Literal["asin"]
    | typing.Literal["acos"]
    | typing.Literal["atan"]
    | typing.Literal["atan2"]
    | typing.Literal["exp"]
    | typing.Literal["exp2"]
    | typing.Literal["log"]
    | typing.Literal["log2"]
    | typing.Literal["log10"]
    | typing.Literal["pow"]
    | typing.Literal["floor"]
    | typing.Literal["ceil"]
    | typing.Literal["trunc"]
    | typing.Literal["round"]
    | typing.Literal["breakpoint"]
    | typing.Literal["returnAddress"]
    | typing.Literal["frameAddress"]
    | typing.Literal["expect"]
    | typing.Literal["blackBox"]
)


def encode_intrinsic(writer: BinaryWriter, value: Intrinsic) -> None:
    """Encode one Intrinsic."""
    if value == "typeOf":
        writer.write_unsigned(0)
    elif value == "sizeOf":
        writer.write_unsigned(1)
    elif value == "alignOf":
        writer.write_unsigned(2)
    elif value == "leadingZeroCount":
        writer.write_unsigned(3)
    elif value == "trailingZeroCount":
        writer.write_unsigned(4)
    elif value == "populationCount":
        writer.write_unsigned(5)
    elif value == "byteSwap":
        writer.write_unsigned(6)
    elif value == "bitReverse":
        writer.write_unsigned(7)
    elif value == "rotateLeft":
        writer.write_unsigned(8)
    elif value == "rotateRight":
        writer.write_unsigned(9)
    elif value == "addOverflow":
        writer.write_unsigned(10)
    elif value == "subOverflow":
        writer.write_unsigned(11)
    elif value == "mulOverflow":
        writer.write_unsigned(12)
    elif value == "addUnchecked":
        writer.write_unsigned(13)
    elif value == "subUnchecked":
        writer.write_unsigned(14)
    elif value == "mulUnchecked":
        writer.write_unsigned(15)
    elif value == "divUnchecked":
        writer.write_unsigned(16)
    elif value == "remUnchecked":
        writer.write_unsigned(17)
    elif value == "shlUnchecked":
        writer.write_unsigned(18)
    elif value == "shrUnchecked":
        writer.write_unsigned(19)
    elif value == "satAdd":
        writer.write_unsigned(20)
    elif value == "satSub":
        writer.write_unsigned(21)
    elif value == "memcpy":
        writer.write_unsigned(22)
    elif value == "memmove":
        writer.write_unsigned(23)
    elif value == "memset":
        writer.write_unsigned(24)
    elif value == "memcmp":
        writer.write_unsigned(25)
    elif value == "prefetchRead":
        writer.write_unsigned(26)
    elif value == "prefetchWrite":
        writer.write_unsigned(27)
    elif value == "transmute":
        writer.write_unsigned(28)
    elif value == "spaceCast":
        writer.write_unsigned(29)
    elif value == "pointerOffsetFrom":
        writer.write_unsigned(30)
    elif value == "rawEq":
        writer.write_unsigned(31)
    elif value == "sqrt":
        writer.write_unsigned(32)
    elif value == "abs":
        writer.write_unsigned(33)
    elif value == "fma":
        writer.write_unsigned(34)
    elif value == "copySign":
        writer.write_unsigned(35)
    elif value == "min":
        writer.write_unsigned(36)
    elif value == "max":
        writer.write_unsigned(37)
    elif value == "sin":
        writer.write_unsigned(38)
    elif value == "cos":
        writer.write_unsigned(39)
    elif value == "tan":
        writer.write_unsigned(40)
    elif value == "asin":
        writer.write_unsigned(41)
    elif value == "acos":
        writer.write_unsigned(42)
    elif value == "atan":
        writer.write_unsigned(43)
    elif value == "atan2":
        writer.write_unsigned(44)
    elif value == "exp":
        writer.write_unsigned(45)
    elif value == "exp2":
        writer.write_unsigned(46)
    elif value == "log":
        writer.write_unsigned(47)
    elif value == "log2":
        writer.write_unsigned(48)
    elif value == "log10":
        writer.write_unsigned(49)
    elif value == "pow":
        writer.write_unsigned(50)
    elif value == "floor":
        writer.write_unsigned(51)
    elif value == "ceil":
        writer.write_unsigned(52)
    elif value == "trunc":
        writer.write_unsigned(53)
    elif value == "round":
        writer.write_unsigned(54)
    elif value == "breakpoint":
        writer.write_unsigned(55)
    elif value == "returnAddress":
        writer.write_unsigned(56)
    elif value == "frameAddress":
        writer.write_unsigned(57)
    elif value == "expect":
        writer.write_unsigned(58)
    elif value == "blackBox":
        writer.write_unsigned(59)
    else:
        raise SerdeError("unknown enum variant")


def decode_intrinsic(reader: BinaryReader) -> Intrinsic:
    """Decode one Intrinsic."""
    variant = reader.read_number()

    if variant == 0:
        return "typeOf"
    elif variant == 1:
        return "sizeOf"
    elif variant == 2:
        return "alignOf"
    elif variant == 3:
        return "leadingZeroCount"
    elif variant == 4:
        return "trailingZeroCount"
    elif variant == 5:
        return "populationCount"
    elif variant == 6:
        return "byteSwap"
    elif variant == 7:
        return "bitReverse"
    elif variant == 8:
        return "rotateLeft"
    elif variant == 9:
        return "rotateRight"
    elif variant == 10:
        return "addOverflow"
    elif variant == 11:
        return "subOverflow"
    elif variant == 12:
        return "mulOverflow"
    elif variant == 13:
        return "addUnchecked"
    elif variant == 14:
        return "subUnchecked"
    elif variant == 15:
        return "mulUnchecked"
    elif variant == 16:
        return "divUnchecked"
    elif variant == 17:
        return "remUnchecked"
    elif variant == 18:
        return "shlUnchecked"
    elif variant == 19:
        return "shrUnchecked"
    elif variant == 20:
        return "satAdd"
    elif variant == 21:
        return "satSub"
    elif variant == 22:
        return "memcpy"
    elif variant == 23:
        return "memmove"
    elif variant == 24:
        return "memset"
    elif variant == 25:
        return "memcmp"
    elif variant == 26:
        return "prefetchRead"
    elif variant == 27:
        return "prefetchWrite"
    elif variant == 28:
        return "transmute"
    elif variant == 29:
        return "spaceCast"
    elif variant == 30:
        return "pointerOffsetFrom"
    elif variant == 31:
        return "rawEq"
    elif variant == 32:
        return "sqrt"
    elif variant == 33:
        return "abs"
    elif variant == 34:
        return "fma"
    elif variant == 35:
        return "copySign"
    elif variant == 36:
        return "min"
    elif variant == 37:
        return "max"
    elif variant == 38:
        return "sin"
    elif variant == 39:
        return "cos"
    elif variant == 40:
        return "tan"
    elif variant == 41:
        return "asin"
    elif variant == 42:
        return "acos"
    elif variant == 43:
        return "atan"
    elif variant == 44:
        return "atan2"
    elif variant == 45:
        return "exp"
    elif variant == 46:
        return "exp2"
    elif variant == 47:
        return "log"
    elif variant == 48:
        return "log2"
    elif variant == 49:
        return "log10"
    elif variant == 50:
        return "pow"
    elif variant == 51:
        return "floor"
    elif variant == 52:
        return "ceil"
    elif variant == 53:
        return "trunc"
    elif variant == 54:
        return "round"
    elif variant == 55:
        return "breakpoint"
    elif variant == 56:
        return "returnAddress"
    elif variant == 57:
        return "frameAddress"
    elif variant == 58:
        return "expect"
    elif variant == 59:
        return "blackBox"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_intrinsic(value: Intrinsic) -> Json:
    """Return one JSON value for one Intrinsic."""
    return value


def from_json_intrinsic(value: Json) -> Intrinsic:
    """Return one Intrinsic from one JSON value."""
    variant = json_string(value)

    if variant == "typeOf":
        return "typeOf"
    elif variant == "sizeOf":
        return "sizeOf"
    elif variant == "alignOf":
        return "alignOf"
    elif variant == "leadingZeroCount":
        return "leadingZeroCount"
    elif variant == "trailingZeroCount":
        return "trailingZeroCount"
    elif variant == "populationCount":
        return "populationCount"
    elif variant == "byteSwap":
        return "byteSwap"
    elif variant == "bitReverse":
        return "bitReverse"
    elif variant == "rotateLeft":
        return "rotateLeft"
    elif variant == "rotateRight":
        return "rotateRight"
    elif variant == "addOverflow":
        return "addOverflow"
    elif variant == "subOverflow":
        return "subOverflow"
    elif variant == "mulOverflow":
        return "mulOverflow"
    elif variant == "addUnchecked":
        return "addUnchecked"
    elif variant == "subUnchecked":
        return "subUnchecked"
    elif variant == "mulUnchecked":
        return "mulUnchecked"
    elif variant == "divUnchecked":
        return "divUnchecked"
    elif variant == "remUnchecked":
        return "remUnchecked"
    elif variant == "shlUnchecked":
        return "shlUnchecked"
    elif variant == "shrUnchecked":
        return "shrUnchecked"
    elif variant == "satAdd":
        return "satAdd"
    elif variant == "satSub":
        return "satSub"
    elif variant == "memcpy":
        return "memcpy"
    elif variant == "memmove":
        return "memmove"
    elif variant == "memset":
        return "memset"
    elif variant == "memcmp":
        return "memcmp"
    elif variant == "prefetchRead":
        return "prefetchRead"
    elif variant == "prefetchWrite":
        return "prefetchWrite"
    elif variant == "transmute":
        return "transmute"
    elif variant == "spaceCast":
        return "spaceCast"
    elif variant == "pointerOffsetFrom":
        return "pointerOffsetFrom"
    elif variant == "rawEq":
        return "rawEq"
    elif variant == "sqrt":
        return "sqrt"
    elif variant == "abs":
        return "abs"
    elif variant == "fma":
        return "fma"
    elif variant == "copySign":
        return "copySign"
    elif variant == "min":
        return "min"
    elif variant == "max":
        return "max"
    elif variant == "sin":
        return "sin"
    elif variant == "cos":
        return "cos"
    elif variant == "tan":
        return "tan"
    elif variant == "asin":
        return "asin"
    elif variant == "acos":
        return "acos"
    elif variant == "atan":
        return "atan"
    elif variant == "atan2":
        return "atan2"
    elif variant == "exp":
        return "exp"
    elif variant == "exp2":
        return "exp2"
    elif variant == "log":
        return "log"
    elif variant == "log2":
        return "log2"
    elif variant == "log10":
        return "log10"
    elif variant == "pow":
        return "pow"
    elif variant == "floor":
        return "floor"
    elif variant == "ceil":
        return "ceil"
    elif variant == "trunc":
        return "trunc"
    elif variant == "round":
        return "round"
    elif variant == "breakpoint":
        return "breakpoint"
    elif variant == "returnAddress":
        return "returnAddress"
    elif variant == "frameAddress":
        return "frameAddress"
    elif variant == "expect":
        return "expect"
    elif variant == "blackBox":
        return "blackBox"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Intrinsic",
    "encode_intrinsic",
    "decode_intrinsic",
    "to_json_intrinsic",
    "from_json_intrinsic",
]
