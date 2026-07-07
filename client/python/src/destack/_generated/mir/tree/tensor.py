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
) -> None:
    """Encode one TensorReduceOperator."""
    if value == "add":
        writer.write_unsigned(0)
    elif value == "multiply":
        writer.write_unsigned(1)
    elif value == "min":
        writer.write_unsigned(2)
    elif value == "max":
        writer.write_unsigned(3)
    elif value == "and":
        writer.write_unsigned(4)
    elif value == "or":
        writer.write_unsigned(5)
    elif value == "xor":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_reduce_operator(reader: BinaryReader) -> TensorReduceOperator:
    """Decode one TensorReduceOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "add"
    elif variant == 1:
        return "multiply"
    elif variant == 2:
        return "min"
    elif variant == 3:
        return "max"
    elif variant == 4:
        return "and"
    elif variant == 5:
        return "or"
    elif variant == 6:
        return "xor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_reduce_operator(value: TensorReduceOperator) -> Json:
    """Return one JSON value for one TensorReduceOperator."""
    return value


def from_json_tensor_reduce_operator(value: Json) -> TensorReduceOperator:
    """Return one TensorReduceOperator from one JSON value."""
    variant = json_string(value)

    if variant == "add":
        return "add"
    elif variant == "multiply":
        return "multiply"
    elif variant == "min":
        return "min"
    elif variant == "max":
        return "max"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    elif variant == "xor":
        return "xor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Index reduction operators for tensor reductions that return indices."""
TensorIndexReduceOperator: typing.TypeAlias = (
    typing.Literal["min"] | typing.Literal["max"]
)


def encode_tensor_index_reduce_operator(
    writer: BinaryWriter, value: TensorIndexReduceOperator
) -> None:
    """Encode one TensorIndexReduceOperator."""
    if value == "min":
        writer.write_unsigned(0)
    elif value == "max":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_index_reduce_operator(
    reader: BinaryReader,
) -> TensorIndexReduceOperator:
    """Decode one TensorIndexReduceOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "min"
    elif variant == 1:
        return "max"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_index_reduce_operator(value: TensorIndexReduceOperator) -> Json:
    """Return one JSON value for one TensorIndexReduceOperator."""
    return value


def from_json_tensor_index_reduce_operator(value: Json) -> TensorIndexReduceOperator:
    """Return one TensorIndexReduceOperator from one JSON value."""
    variant = json_string(value)

    if variant == "min":
        return "min"
    elif variant == "max":
        return "max"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Tie-breaking behavior for tensor index reductions."""
TensorIndexTieBreak: typing.TypeAlias = typing.Literal["first"] | typing.Literal["last"]


def encode_tensor_index_tie_break(
    writer: BinaryWriter, value: TensorIndexTieBreak
) -> None:
    """Encode one TensorIndexTieBreak."""
    if value == "first":
        writer.write_unsigned(0)
    elif value == "last":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_index_tie_break(reader: BinaryReader) -> TensorIndexTieBreak:
    """Decode one TensorIndexTieBreak."""
    variant = reader.read_number()

    if variant == 0:
        return "first"
    elif variant == 1:
        return "last"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_index_tie_break(value: TensorIndexTieBreak) -> Json:
    """Return one JSON value for one TensorIndexTieBreak."""
    return value


def from_json_tensor_index_tie_break(value: Json) -> TensorIndexTieBreak:
    """Return one TensorIndexTieBreak from one JSON value."""
    variant = json_string(value)

    if variant == "first":
        return "first"
    elif variant == "last":
        return "last"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_tensor_scatter_mode(writer: BinaryWriter, value: TensorScatterMode) -> None:
    """Encode one TensorScatterMode."""
    if value == "replace":
        writer.write_unsigned(0)
    elif value == "add":
        writer.write_unsigned(1)
    elif value == "multiply":
        writer.write_unsigned(2)
    elif value == "min":
        writer.write_unsigned(3)
    elif value == "max":
        writer.write_unsigned(4)
    elif value == "and":
        writer.write_unsigned(5)
    elif value == "or":
        writer.write_unsigned(6)
    elif value == "xor":
        writer.write_unsigned(7)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_scatter_mode(reader: BinaryReader) -> TensorScatterMode:
    """Decode one TensorScatterMode."""
    variant = reader.read_number()

    if variant == 0:
        return "replace"
    elif variant == 1:
        return "add"
    elif variant == 2:
        return "multiply"
    elif variant == 3:
        return "min"
    elif variant == 4:
        return "max"
    elif variant == 5:
        return "and"
    elif variant == 6:
        return "or"
    elif variant == 7:
        return "xor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_scatter_mode(value: TensorScatterMode) -> Json:
    """Return one JSON value for one TensorScatterMode."""
    return value


def from_json_tensor_scatter_mode(value: Json) -> TensorScatterMode:
    """Return one TensorScatterMode from one JSON value."""
    variant = json_string(value)

    if variant == "replace":
        return "replace"
    elif variant == "add":
        return "add"
    elif variant == "multiply":
        return "multiply"
    elif variant == "min":
        return "min"
    elif variant == "max":
        return "max"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    elif variant == "xor":
        return "xor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Conversion modes for tensor element conversions."""
TensorConvertMode: typing.TypeAlias = (
    typing.Literal["exact"]
    | typing.Literal["roundTiesEven"]
    | typing.Literal["roundTowardZero"]
    | typing.Literal["roundFloor"]
    | typing.Literal["roundCeil"]
    | typing.Literal["saturate"]
)


def encode_tensor_convert_mode(writer: BinaryWriter, value: TensorConvertMode) -> None:
    """Encode one TensorConvertMode."""
    if value == "exact":
        writer.write_unsigned(0)
    elif value == "roundTiesEven":
        writer.write_unsigned(1)
    elif value == "roundTowardZero":
        writer.write_unsigned(2)
    elif value == "roundFloor":
        writer.write_unsigned(3)
    elif value == "roundCeil":
        writer.write_unsigned(4)
    elif value == "saturate":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_convert_mode(reader: BinaryReader) -> TensorConvertMode:
    """Decode one TensorConvertMode."""
    variant = reader.read_number()

    if variant == 0:
        return "exact"
    elif variant == 1:
        return "roundTiesEven"
    elif variant == 2:
        return "roundTowardZero"
    elif variant == 3:
        return "roundFloor"
    elif variant == 4:
        return "roundCeil"
    elif variant == 5:
        return "saturate"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_convert_mode(value: TensorConvertMode) -> Json:
    """Return one JSON value for one TensorConvertMode."""
    return value


def from_json_tensor_convert_mode(value: Json) -> TensorConvertMode:
    """Return one TensorConvertMode from one JSON value."""
    variant = json_string(value)

    if variant == "exact":
        return "exact"
    elif variant == "roundTiesEven":
        return "roundTiesEven"
    elif variant == "roundTowardZero":
        return "roundTowardZero"
    elif variant == "roundFloor":
        return "roundFloor"
    elif variant == "roundCeil":
        return "roundCeil"
    elif variant == "saturate":
        return "saturate"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
