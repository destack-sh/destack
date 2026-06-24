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
) -> None:
    """Encode one VectorReduceOperator."""
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


def decode_vector_reduce_operator(reader: BinaryReader) -> VectorReduceOperator:
    """Decode one VectorReduceOperator."""
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


def to_json_vector_reduce_operator(value: VectorReduceOperator) -> Json:
    """Return one JSON value for one VectorReduceOperator."""
    return value


def from_json_vector_reduce_operator(value: Json) -> VectorReduceOperator:
    """Return one VectorReduceOperator from one JSON value."""
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


"""Conversion modes for vector element conversions."""
VectorConvertMode: typing.TypeAlias = (
    typing.Literal["exact"]
    | typing.Literal["roundTiesEven"]
    | typing.Literal["roundTowardZero"]
    | typing.Literal["roundFloor"]
    | typing.Literal["roundCeil"]
    | typing.Literal["saturate"]
)


def encode_vector_convert_mode(writer: BinaryWriter, value: VectorConvertMode) -> None:
    """Encode one VectorConvertMode."""
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


def decode_vector_convert_mode(reader: BinaryReader) -> VectorConvertMode:
    """Decode one VectorConvertMode."""
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


def to_json_vector_convert_mode(value: VectorConvertMode) -> Json:
    """Return one JSON value for one VectorConvertMode."""
    return value


def from_json_vector_convert_mode(value: Json) -> VectorConvertMode:
    """Return one VectorConvertMode from one JSON value."""
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
