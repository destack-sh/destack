# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

def encode_intrinsic(writer: BinaryWriter, value: Intrinsic) -> None: ...
def decode_intrinsic(reader: BinaryReader) -> Intrinsic: ...
def to_json_intrinsic(value: Intrinsic) -> Json: ...
def from_json_intrinsic(value: Json) -> Intrinsic: ...

__all__ = [
    "Intrinsic",
    "encode_intrinsic",
    "decode_intrinsic",
    "to_json_intrinsic",
    "from_json_intrinsic",
]
