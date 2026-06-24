# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
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

@dataclass(frozen=True, slots=True)
class TensorDotDimensionNumbers:
    """Dimension numbers for tensor dot operations."""

    # batch dimensions on the left operand
    lhs_batch: Sequence[int]
    # batch dimensions on the right operand
    rhs_batch: Sequence[int]
    # contracting dimensions on the left operand
    lhs_contracting: Sequence[int]
    # contracting dimensions on the right operand
    rhs_contracting: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorDotDimensionNumbers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorDotDimensionNumbers: ...

def encode_tensor_dot_dimension_numbers(
    writer: BinaryWriter, value: TensorDotDimensionNumbers
) -> None: ...
def decode_tensor_dot_dimension_numbers(
    reader: BinaryReader,
) -> TensorDotDimensionNumbers: ...
def to_json_tensor_dot_dimension_numbers(value: TensorDotDimensionNumbers) -> Json: ...
def from_json_tensor_dot_dimension_numbers(
    value: Json,
) -> TensorDotDimensionNumbers: ...

@dataclass(frozen=True, slots=True)
class TensorConvolutionDimensionNumbers:
    """Dimension numbers for tensor convolution operations."""

    # batch dimension for the input tensor
    input_batch: int
    # feature dimension for the input tensor
    input_feature: int
    # spatial dimensions for the input tensor
    input_spatial: Sequence[int]
    # input feature dimension for the kernel tensor
    kernel_input_feature: int
    # output feature dimension for the kernel tensor
    kernel_output_feature: int
    # spatial dimensions for the kernel tensor
    kernel_spatial: Sequence[int]
    # batch dimension for the output tensor
    output_batch: int
    # feature dimension for the output tensor
    output_feature: int
    # spatial dimensions for the output tensor
    output_spatial: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolutionDimensionNumbers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorConvolutionDimensionNumbers: ...

def encode_tensor_convolution_dimension_numbers(
    writer: BinaryWriter, value: TensorConvolutionDimensionNumbers
) -> None: ...
def decode_tensor_convolution_dimension_numbers(
    reader: BinaryReader,
) -> TensorConvolutionDimensionNumbers: ...
def to_json_tensor_convolution_dimension_numbers(
    value: TensorConvolutionDimensionNumbers,
) -> Json: ...
def from_json_tensor_convolution_dimension_numbers(
    value: Json,
) -> TensorConvolutionDimensionNumbers: ...

@dataclass(frozen=True, slots=True)
class TensorConvolutionWindow:
    """Window parameters for tensor convolution."""

    # stride for each spatial dimension
    strides: Sequence[int]
    # padding at the low end for each spatial dimension
    padding_low: Sequence[int]
    # padding at the high end for each spatial dimension
    padding_high: Sequence[int]
    # input dilation for each spatial dimension
    lhs_dilation: Sequence[int]
    # kernel dilation for each spatial dimension
    rhs_dilation: Sequence[int]
    # whether each spatial dimension is reversed
    window_reversal: Sequence[bool]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolutionWindow: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorConvolutionWindow: ...

def encode_tensor_convolution_window(
    writer: BinaryWriter, value: TensorConvolutionWindow
) -> None: ...
def decode_tensor_convolution_window(
    reader: BinaryReader,
) -> TensorConvolutionWindow: ...
def to_json_tensor_convolution_window(value: TensorConvolutionWindow) -> Json: ...
def from_json_tensor_convolution_window(value: Json) -> TensorConvolutionWindow: ...

@dataclass(frozen=True, slots=True)
class TensorGatherDimensionNumbers:
    """Dimension numbers for tensor gather operations."""

    # offset dimensions in the output
    offset_dims: Sequence[int]
    # collapsed slice dimensions in the operand
    collapsed_slice_dims: Sequence[int]
    # mapping from index components to operand dimensions
    start_index_map: Sequence[int]
    # index vector dimension in the indices tensor
    index_vector_dim: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorGatherDimensionNumbers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorGatherDimensionNumbers: ...

def encode_tensor_gather_dimension_numbers(
    writer: BinaryWriter, value: TensorGatherDimensionNumbers
) -> None: ...
def decode_tensor_gather_dimension_numbers(
    reader: BinaryReader,
) -> TensorGatherDimensionNumbers: ...
def to_json_tensor_gather_dimension_numbers(
    value: TensorGatherDimensionNumbers,
) -> Json: ...
def from_json_tensor_gather_dimension_numbers(
    value: Json,
) -> TensorGatherDimensionNumbers: ...

@dataclass(frozen=True, slots=True)
class TensorScatterDimensionNumbers:
    """Dimension numbers for tensor scatter operations."""

    # dimensions of the update window in the updates tensor
    update_window_dims: Sequence[int]
    # dimensions inserted into the operand shape
    inserted_window_dims: Sequence[int]
    # mapping from scatter indices to operand dimensions
    scatter_dims_to_operand_dims: Sequence[int]
    # index vector dimension in the indices tensor
    index_vector_dim: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorScatterDimensionNumbers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorScatterDimensionNumbers: ...

def encode_tensor_scatter_dimension_numbers(
    writer: BinaryWriter, value: TensorScatterDimensionNumbers
) -> None: ...
def decode_tensor_scatter_dimension_numbers(
    reader: BinaryReader,
) -> TensorScatterDimensionNumbers: ...
def to_json_tensor_scatter_dimension_numbers(
    value: TensorScatterDimensionNumbers,
) -> Json: ...
def from_json_tensor_scatter_dimension_numbers(
    value: Json,
) -> TensorScatterDimensionNumbers: ...

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
    "TensorDotDimensionNumbers",
    "encode_tensor_dot_dimension_numbers",
    "decode_tensor_dot_dimension_numbers",
    "to_json_tensor_dot_dimension_numbers",
    "from_json_tensor_dot_dimension_numbers",
    "TensorConvolutionDimensionNumbers",
    "encode_tensor_convolution_dimension_numbers",
    "decode_tensor_convolution_dimension_numbers",
    "to_json_tensor_convolution_dimension_numbers",
    "from_json_tensor_convolution_dimension_numbers",
    "TensorConvolutionWindow",
    "encode_tensor_convolution_window",
    "decode_tensor_convolution_window",
    "to_json_tensor_convolution_window",
    "from_json_tensor_convolution_window",
    "TensorGatherDimensionNumbers",
    "encode_tensor_gather_dimension_numbers",
    "decode_tensor_gather_dimension_numbers",
    "to_json_tensor_gather_dimension_numbers",
    "from_json_tensor_gather_dimension_numbers",
    "TensorScatterDimensionNumbers",
    "encode_tensor_scatter_dimension_numbers",
    "decode_tensor_scatter_dimension_numbers",
    "to_json_tensor_scatter_dimension_numbers",
    "from_json_tensor_scatter_dimension_numbers",
]
