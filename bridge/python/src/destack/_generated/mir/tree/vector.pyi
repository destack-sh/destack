# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Reduction operators for vector reductions."""
VectorReduceOperator: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["min"]
    | typing.Literal["max"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
)

def encode_vector_reduce_operator(
    writer: BinaryWriter, value: VectorReduceOperator
) -> None: ...
def decode_vector_reduce_operator(reader: BinaryReader) -> VectorReduceOperator: ...
def to_json_vector_reduce_operator(value: VectorReduceOperator) -> Json: ...
def from_json_vector_reduce_operator(value: Json) -> VectorReduceOperator: ...

"""Conversion modes for vector element conversions."""
VectorConvertMode: typing.TypeAlias = (
    typing.Literal["exact"]
    | typing.Literal["roundTiesEven"]
    | typing.Literal["roundTowardZero"]
    | typing.Literal["roundFloor"]
    | typing.Literal["roundCeil"]
    | typing.Literal["saturate"]
)

def encode_vector_convert_mode(
    writer: BinaryWriter, value: VectorConvertMode
) -> None: ...
def decode_vector_convert_mode(reader: BinaryReader) -> VectorConvertMode: ...
def to_json_vector_convert_mode(value: VectorConvertMode) -> Json: ...
def from_json_vector_convert_mode(value: Json) -> VectorConvertMode: ...

__all__ = [
    "VectorReduceOperator",
    "encode_vector_reduce_operator",
    "decode_vector_reduce_operator",
    "to_json_vector_reduce_operator",
    "from_json_vector_reduce_operator",
    "VectorConvertMode",
    "encode_vector_convert_mode",
    "decode_vector_convert_mode",
    "to_json_vector_convert_mode",
    "from_json_vector_convert_mode",
]
