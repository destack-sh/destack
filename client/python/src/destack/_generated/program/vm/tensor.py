# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.program.vm.projection
import destack._generated.program.vm.value

"""Tensor view backing memory selected by lowering."""
TensorAddress: typing.TypeAlias = (
    typing.Literal["heap"]
    | typing.Literal["sharedHeap"]
    | typing.Literal["address"]
    | typing.Literal["stack"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)


def encode_tensor_address(writer: BinaryWriter, value: TensorAddress) -> None:
    """Encode one TensorAddress."""
    if value == "heap":
        writer.write_unsigned(0)
    elif value == "sharedHeap":
        writer.write_unsigned(1)
    elif value == "address":
        writer.write_unsigned(2)
    elif value == "stack":
        writer.write_unsigned(3)
    elif value == "frame":
        writer.write_unsigned(4)
    elif value == "static":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_address(reader: BinaryReader) -> TensorAddress:
    """Decode one TensorAddress."""
    variant = reader.read_number()

    if variant == 0:
        return "heap"
    elif variant == 1:
        return "sharedHeap"
    elif variant == 2:
        return "address"
    elif variant == 3:
        return "stack"
    elif variant == 4:
        return "frame"
    elif variant == 5:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_address(value: TensorAddress) -> Json:
    """Return one JSON value for one TensorAddress."""
    return value


def from_json_tensor_address(value: Json) -> TensorAddress:
    """Return one TensorAddress from one JSON value."""
    variant = json_string(value)

    if variant == "heap":
        return "heap"
    elif variant == "sharedHeap":
        return "sharedHeap"
    elif variant == "address":
        return "address"
    elif variant == "stack":
        return "stack"
    elif variant == "frame":
        return "frame"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TensorLayout:
    """Flattened tensor layout compiled for VM execution."""

    # the tensor payload byte width
    byte_len: int
    # the static tensor shape
    shape: Sequence[int]
    # the per-dimension strides in element units
    strides: Sequence[int]
    # the number of logical tensor elements
    element_count: int
    # the number of addressable element positions
    element_span_len: int
    # whether logical elements are stored contiguously
    is_contiguous: bool
    # the scalar layout of each element
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the frame projection for each element
    element: destack._generated.program.vm.projection.Projection

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLayout:
        """Decode one TensorLayout."""
        return decode_tensor_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorLayout:
        """Return one TensorLayout from one JSON value."""
        return from_json_tensor_layout(value)


def encode_tensor_layout(writer: BinaryWriter, value: TensorLayout) -> None:
    """Encode one TensorLayout."""
    writer.write_unsigned(value.byte_len)
    writer.write_unsigned(len(value.shape))
    for item_value_shape_0 in value.shape:
        writer.write_unsigned(item_value_shape_0)
    writer.write_unsigned(len(value.strides))
    for item_value_strides_0 in value.strides:
        writer.write_unsigned(item_value_strides_0)
    writer.write_unsigned(value.element_count)
    writer.write_unsigned(value.element_span_len)
    writer.write_bool(value.is_contiguous)
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    destack._generated.program.vm.projection.encode_projection(writer, value.element)


def decode_tensor_layout(reader: BinaryReader) -> TensorLayout:
    """Decode one TensorLayout."""
    byte_len = reader.read_number()
    shape = [reader.read_number() for _ in range(reader.read_number())]
    strides = [reader.read_number() for _ in range(reader.read_number())]
    element_count = reader.read_number()
    element_span_len = reader.read_number()
    is_contiguous = reader.read_bool()
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    element = destack._generated.program.vm.projection.decode_projection(reader)

    return TensorLayout(
        byte_len=byte_len,
        shape=shape,
        strides=strides,
        element_count=element_count,
        element_span_len=element_span_len,
        is_contiguous=is_contiguous,
        element_layout=element_layout,
        element=element,
    )


def to_json_tensor_layout(value: TensorLayout) -> Json:
    """Return one JSON value for one TensorLayout."""
    return {
        "byteLen": value.byte_len,
        "shape": [item_0 for item_0 in value.shape],
        "strides": [item_0 for item_0 in value.strides],
        "elementCount": value.element_count,
        "elementSpanLen": value.element_span_len,
        "isContiguous": value.is_contiguous,
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "element": destack._generated.program.vm.projection.to_json_projection(
            value.element
        ),
    }


def from_json_tensor_layout(value: Json) -> TensorLayout:
    """Return one TensorLayout from one JSON value."""
    object_ = json_object(value)

    return TensorLayout(
        byte_len=json_int(json_field(object_, "byteLen")),
        shape=[json_int(item_0) for item_0 in json_array(json_field(object_, "shape"))],
        strides=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "strides"))
        ],
        element_count=json_int(json_field(object_, "elementCount")),
        element_span_len=json_int(json_field(object_, "elementSpanLen")),
        is_contiguous=json_bool(json_field(object_, "isContiguous")),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "element")
        ),
    )


__all__ = [
    "TensorAddress",
    "encode_tensor_address",
    "decode_tensor_address",
    "to_json_tensor_address",
    "from_json_tensor_address",
    "TensorLayout",
    "encode_tensor_layout",
    "decode_tensor_layout",
    "to_json_tensor_layout",
    "from_json_tensor_layout",
]
