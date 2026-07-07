# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Reduction operators for tensor reductions."""
TensorReduceOperator: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["min"]
    | typing.Literal["max"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
)

def encode_tensor_reduce_operator(
    writer: BinaryWriter, value: TensorReduceOperator
) -> None: ...
def decode_tensor_reduce_operator(reader: BinaryReader) -> TensorReduceOperator: ...
def to_json_tensor_reduce_operator(value: TensorReduceOperator) -> Json: ...
def from_json_tensor_reduce_operator(value: Json) -> TensorReduceOperator: ...

"""Index reduction operators for tensor reductions that return indices."""
TensorIndexReduceOperator: typing.TypeAlias = (
    typing.Literal["min"] | typing.Literal["max"]
)

def encode_tensor_index_reduce_operator(
    writer: BinaryWriter, value: TensorIndexReduceOperator
) -> None: ...
def decode_tensor_index_reduce_operator(
    reader: BinaryReader,
) -> TensorIndexReduceOperator: ...
def to_json_tensor_index_reduce_operator(value: TensorIndexReduceOperator) -> Json: ...
def from_json_tensor_index_reduce_operator(
    value: Json,
) -> TensorIndexReduceOperator: ...

"""Tie-breaking behavior for tensor index reductions."""
TensorIndexTieBreak: typing.TypeAlias = typing.Literal["first"] | typing.Literal["last"]

def encode_tensor_index_tie_break(
    writer: BinaryWriter, value: TensorIndexTieBreak
) -> None: ...
def decode_tensor_index_tie_break(reader: BinaryReader) -> TensorIndexTieBreak: ...
def to_json_tensor_index_tie_break(value: TensorIndexTieBreak) -> Json: ...
def from_json_tensor_index_tie_break(value: Json) -> TensorIndexTieBreak: ...

"""Update modes for tensor scatter operations."""
TensorScatterMode: typing.TypeAlias = (
    typing.Literal["replace"]
    | typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["min"]
    | typing.Literal["max"]
    | typing.Literal["and"]
    | typing.Literal["or"]
    | typing.Literal["xor"]
)

def encode_tensor_scatter_mode(
    writer: BinaryWriter, value: TensorScatterMode
) -> None: ...
def decode_tensor_scatter_mode(reader: BinaryReader) -> TensorScatterMode: ...
def to_json_tensor_scatter_mode(value: TensorScatterMode) -> Json: ...
def from_json_tensor_scatter_mode(value: Json) -> TensorScatterMode: ...

"""Conversion modes for tensor element conversions."""
TensorConvertMode: typing.TypeAlias = (
    typing.Literal["exact"]
    | typing.Literal["roundTiesEven"]
    | typing.Literal["roundTowardZero"]
    | typing.Literal["roundFloor"]
    | typing.Literal["roundCeil"]
    | typing.Literal["saturate"]
)

def encode_tensor_convert_mode(
    writer: BinaryWriter, value: TensorConvertMode
) -> None: ...
def decode_tensor_convert_mode(reader: BinaryReader) -> TensorConvertMode: ...
def to_json_tensor_convert_mode(value: TensorConvertMode) -> Json: ...
def from_json_tensor_convert_mode(value: Json) -> TensorConvertMode: ...

__all__ = [
    "TensorReduceOperator",
    "encode_tensor_reduce_operator",
    "decode_tensor_reduce_operator",
    "to_json_tensor_reduce_operator",
    "from_json_tensor_reduce_operator",
    "TensorIndexReduceOperator",
    "encode_tensor_index_reduce_operator",
    "decode_tensor_index_reduce_operator",
    "to_json_tensor_index_reduce_operator",
    "from_json_tensor_index_reduce_operator",
    "TensorIndexTieBreak",
    "encode_tensor_index_tie_break",
    "decode_tensor_index_tie_break",
    "to_json_tensor_index_tie_break",
    "from_json_tensor_index_tie_break",
    "TensorScatterMode",
    "encode_tensor_scatter_mode",
    "decode_tensor_scatter_mode",
    "to_json_tensor_scatter_mode",
    "from_json_tensor_scatter_mode",
    "TensorConvertMode",
    "encode_tensor_convert_mode",
    "decode_tensor_convert_mode",
    "to_json_tensor_convert_mode",
    "from_json_tensor_convert_mode",
]
