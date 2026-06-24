# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class IndexSlice:
    """Compact reference to an index list stored in the MIR tree."""

    # start index in the index buffer
    start: int
    # number of indices in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndexSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndexSlice: ...

def encode_index_slice(writer: BinaryWriter, value: IndexSlice) -> None: ...
def decode_index_slice(reader: BinaryReader) -> IndexSlice: ...
def to_json_index_slice(value: IndexSlice) -> Json: ...
def from_json_index_slice(value: Json) -> IndexSlice: ...

"""Compact identity for one tensor immediate stored in the MIR tree."""
TensorImmediateId: typing.TypeAlias = int

def encode_tensor_immediate_id(
    writer: BinaryWriter, value: TensorImmediateId
) -> None: ...
def decode_tensor_immediate_id(reader: BinaryReader) -> TensorImmediateId: ...
def to_json_tensor_immediate_id(value: TensorImmediateId) -> Json: ...
def from_json_tensor_immediate_id(value: Json) -> TensorImmediateId: ...

@dataclass(frozen=True, slots=True)
class TensorImmediateDot:
    """Dimension numbers for a tensor dot product."""

    # batch dimensions on the left operand
    lhs_batch: IndexSlice
    # batch dimensions on the right operand
    rhs_batch: IndexSlice
    # contracting dimensions on the left operand
    lhs_contracting: IndexSlice
    # contracting dimensions on the right operand
    rhs_contracting: IndexSlice
    kind: typing.Literal["dot"] = "dot"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorImmediateConvolution:
    """Dimension numbers and window parameters for tensor convolution."""

    # batch dimension for the input tensor
    input_batch: int
    # feature dimension for the input tensor
    input_feature: int
    # spatial dimensions for the input tensor
    input_spatial: IndexSlice
    # input feature dimension for the kernel tensor
    kernel_input_feature: int
    # output feature dimension for the kernel tensor
    kernel_output_feature: int
    # spatial dimensions for the kernel tensor
    kernel_spatial: IndexSlice
    # batch dimension for the output tensor
    output_batch: int
    # feature dimension for the output tensor
    output_feature: int
    # spatial dimensions for the output tensor
    output_spatial: IndexSlice
    # stride for each spatial dimension
    strides: ExtentSlice
    # padding at the low end for each spatial dimension
    padding_low: ExtentSlice
    # padding at the high end for each spatial dimension
    padding_high: ExtentSlice
    # input dilation for each spatial dimension
    lhs_dilation: ExtentSlice
    # kernel dilation for each spatial dimension
    rhs_dilation: ExtentSlice
    # whether each spatial dimension is reversed
    window_reversal: FlagSlice
    # number of feature groups
    feature_group_count: int
    # number of batch groups
    batch_group_count: int
    kind: typing.Literal["convolution"] = "convolution"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorImmediateGather:
    """Dimension numbers for tensor gather."""

    # offset dimensions in the output
    offset_dims: IndexSlice
    # collapsed slice dimensions in the operand
    collapsed_slice_dims: IndexSlice
    # mapping from index components to operand dimensions
    start_index_map: IndexSlice
    # index vector dimension in the indices tensor
    index_vector_dim: int
    # slice sizes for each operand dimension
    slice_sizes: IndexSlice
    kind: typing.Literal["gather"] = "gather"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TensorImmediateScatter:
    """Dimension numbers for tensor scatter."""

    # dimensions of the update window in the updates tensor
    update_window_dims: IndexSlice
    # dimensions inserted into the operand shape
    inserted_window_dims: IndexSlice
    # mapping from scatter indices to operand dimensions
    scatter_dims_to_operand_dims: IndexSlice
    # index vector dimension in the indices tensor
    index_vector_dim: int
    kind: typing.Literal["scatter"] = "scatter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Static tensor instruction immediate."""
TensorImmediate: typing.TypeAlias = (
    TensorImmediateDot
    | TensorImmediateConvolution
    | TensorImmediateGather
    | TensorImmediateScatter
)

def encode_tensor_immediate(writer: BinaryWriter, value: TensorImmediate) -> None: ...
def decode_tensor_immediate(reader: BinaryReader) -> TensorImmediate: ...
def to_json_tensor_immediate(value: TensorImmediate) -> Json: ...
def from_json_tensor_immediate(value: Json) -> TensorImmediate: ...

@dataclass(frozen=True, slots=True)
class ExtentSlice:
    """Compact reference to an extent list stored in the MIR tree."""

    # start index in the extent buffer
    start: int
    # number of extents in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtentSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtentSlice: ...

def encode_extent_slice(writer: BinaryWriter, value: ExtentSlice) -> None: ...
def decode_extent_slice(reader: BinaryReader) -> ExtentSlice: ...
def to_json_extent_slice(value: ExtentSlice) -> Json: ...
def from_json_extent_slice(value: Json) -> ExtentSlice: ...

@dataclass(frozen=True, slots=True)
class FlagSlice:
    """Compact reference to a flag list stored in the MIR tree."""

    # start index in the flag buffer
    start: int
    # number of flags in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FlagSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FlagSlice: ...

def encode_flag_slice(writer: BinaryWriter, value: FlagSlice) -> None: ...
def decode_flag_slice(reader: BinaryReader) -> FlagSlice: ...
def to_json_flag_slice(value: FlagSlice) -> Json: ...
def from_json_flag_slice(value: Json) -> FlagSlice: ...

__all__ = [
    "IndexSlice",
    "encode_index_slice",
    "decode_index_slice",
    "to_json_index_slice",
    "from_json_index_slice",
    "TensorImmediateId",
    "encode_tensor_immediate_id",
    "decode_tensor_immediate_id",
    "to_json_tensor_immediate_id",
    "from_json_tensor_immediate_id",
    "TensorImmediate",
    "encode_tensor_immediate",
    "decode_tensor_immediate",
    "to_json_tensor_immediate",
    "from_json_tensor_immediate",
    "TensorImmediateDot",
    "TensorImmediateConvolution",
    "TensorImmediateGather",
    "TensorImmediateScatter",
    "ExtentSlice",
    "encode_extent_slice",
    "decode_extent_slice",
    "to_json_extent_slice",
    "from_json_extent_slice",
    "FlagSlice",
    "encode_flag_slice",
    "decode_flag_slice",
    "to_json_flag_slice",
    "from_json_flag_slice",
]
