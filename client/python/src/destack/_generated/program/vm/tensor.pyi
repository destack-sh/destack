# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

def encode_tensor_address(writer: BinaryWriter, value: TensorAddress) -> None: ...
def decode_tensor_address(reader: BinaryReader) -> TensorAddress: ...
def to_json_tensor_address(value: TensorAddress) -> Json: ...
def from_json_tensor_address(value: Json) -> TensorAddress: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorLayout: ...

def encode_tensor_layout(writer: BinaryWriter, value: TensorLayout) -> None: ...
def decode_tensor_layout(reader: BinaryReader) -> TensorLayout: ...
def to_json_tensor_layout(value: TensorLayout) -> Json: ...
def from_json_tensor_layout(value: Json) -> TensorLayout: ...

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
