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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_dot_dimension_numbers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorDotDimensionNumbers:
        """Decode one TensorDotDimensionNumbers."""
        return decode_tensor_dot_dimension_numbers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_dot_dimension_numbers(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorDotDimensionNumbers:
        """Return one TensorDotDimensionNumbers from one JSON value."""
        return from_json_tensor_dot_dimension_numbers(value)


def encode_tensor_dot_dimension_numbers(
    writer: BinaryWriter, value: TensorDotDimensionNumbers
) -> None:
    """Encode one TensorDotDimensionNumbers."""
    writer.write_unsigned(len(value.lhs_batch))
    for item_value_lhs_batch_0 in value.lhs_batch:
        writer.write_unsigned(item_value_lhs_batch_0)
    writer.write_unsigned(len(value.rhs_batch))
    for item_value_rhs_batch_0 in value.rhs_batch:
        writer.write_unsigned(item_value_rhs_batch_0)
    writer.write_unsigned(len(value.lhs_contracting))
    for item_value_lhs_contracting_0 in value.lhs_contracting:
        writer.write_unsigned(item_value_lhs_contracting_0)
    writer.write_unsigned(len(value.rhs_contracting))
    for item_value_rhs_contracting_0 in value.rhs_contracting:
        writer.write_unsigned(item_value_rhs_contracting_0)


def decode_tensor_dot_dimension_numbers(
    reader: BinaryReader,
) -> TensorDotDimensionNumbers:
    """Decode one TensorDotDimensionNumbers."""
    lhs_batch = [reader.read_number() for _ in range(reader.read_number())]
    rhs_batch = [reader.read_number() for _ in range(reader.read_number())]
    lhs_contracting = [reader.read_number() for _ in range(reader.read_number())]
    rhs_contracting = [reader.read_number() for _ in range(reader.read_number())]

    return TensorDotDimensionNumbers(
        lhs_batch=lhs_batch,
        rhs_batch=rhs_batch,
        lhs_contracting=lhs_contracting,
        rhs_contracting=rhs_contracting,
    )


def to_json_tensor_dot_dimension_numbers(value: TensorDotDimensionNumbers) -> Json:
    """Return one JSON value for one TensorDotDimensionNumbers."""
    return {
        "lhsBatch": [item_0 for item_0 in value.lhs_batch],
        "rhsBatch": [item_0 for item_0 in value.rhs_batch],
        "lhsContracting": [item_0 for item_0 in value.lhs_contracting],
        "rhsContracting": [item_0 for item_0 in value.rhs_contracting],
    }


def from_json_tensor_dot_dimension_numbers(value: Json) -> TensorDotDimensionNumbers:
    """Return one TensorDotDimensionNumbers from one JSON value."""
    object_ = json_object(value)

    return TensorDotDimensionNumbers(
        lhs_batch=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "lhsBatch"))
        ],
        rhs_batch=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "rhsBatch"))
        ],
        lhs_contracting=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "lhsContracting"))
        ],
        rhs_contracting=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "rhsContracting"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_convolution_dimension_numbers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolutionDimensionNumbers:
        """Decode one TensorConvolutionDimensionNumbers."""
        return decode_tensor_convolution_dimension_numbers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_convolution_dimension_numbers(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorConvolutionDimensionNumbers:
        """Return one TensorConvolutionDimensionNumbers from one JSON value."""
        return from_json_tensor_convolution_dimension_numbers(value)


def encode_tensor_convolution_dimension_numbers(
    writer: BinaryWriter, value: TensorConvolutionDimensionNumbers
) -> None:
    """Encode one TensorConvolutionDimensionNumbers."""
    writer.write_unsigned(value.input_batch)
    writer.write_unsigned(value.input_feature)
    writer.write_unsigned(len(value.input_spatial))
    for item_value_input_spatial_0 in value.input_spatial:
        writer.write_unsigned(item_value_input_spatial_0)
    writer.write_unsigned(value.kernel_input_feature)
    writer.write_unsigned(value.kernel_output_feature)
    writer.write_unsigned(len(value.kernel_spatial))
    for item_value_kernel_spatial_0 in value.kernel_spatial:
        writer.write_unsigned(item_value_kernel_spatial_0)
    writer.write_unsigned(value.output_batch)
    writer.write_unsigned(value.output_feature)
    writer.write_unsigned(len(value.output_spatial))
    for item_value_output_spatial_0 in value.output_spatial:
        writer.write_unsigned(item_value_output_spatial_0)


def decode_tensor_convolution_dimension_numbers(
    reader: BinaryReader,
) -> TensorConvolutionDimensionNumbers:
    """Decode one TensorConvolutionDimensionNumbers."""
    input_batch = reader.read_number()
    input_feature = reader.read_number()
    input_spatial = [reader.read_number() for _ in range(reader.read_number())]
    kernel_input_feature = reader.read_number()
    kernel_output_feature = reader.read_number()
    kernel_spatial = [reader.read_number() for _ in range(reader.read_number())]
    output_batch = reader.read_number()
    output_feature = reader.read_number()
    output_spatial = [reader.read_number() for _ in range(reader.read_number())]

    return TensorConvolutionDimensionNumbers(
        input_batch=input_batch,
        input_feature=input_feature,
        input_spatial=input_spatial,
        kernel_input_feature=kernel_input_feature,
        kernel_output_feature=kernel_output_feature,
        kernel_spatial=kernel_spatial,
        output_batch=output_batch,
        output_feature=output_feature,
        output_spatial=output_spatial,
    )


def to_json_tensor_convolution_dimension_numbers(
    value: TensorConvolutionDimensionNumbers,
) -> Json:
    """Return one JSON value for one TensorConvolutionDimensionNumbers."""
    return {
        "inputBatch": value.input_batch,
        "inputFeature": value.input_feature,
        "inputSpatial": [item_0 for item_0 in value.input_spatial],
        "kernelInputFeature": value.kernel_input_feature,
        "kernelOutputFeature": value.kernel_output_feature,
        "kernelSpatial": [item_0 for item_0 in value.kernel_spatial],
        "outputBatch": value.output_batch,
        "outputFeature": value.output_feature,
        "outputSpatial": [item_0 for item_0 in value.output_spatial],
    }


def from_json_tensor_convolution_dimension_numbers(
    value: Json,
) -> TensorConvolutionDimensionNumbers:
    """Return one TensorConvolutionDimensionNumbers from one JSON value."""
    object_ = json_object(value)

    return TensorConvolutionDimensionNumbers(
        input_batch=json_int(json_field(object_, "inputBatch")),
        input_feature=json_int(json_field(object_, "inputFeature")),
        input_spatial=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "inputSpatial"))
        ],
        kernel_input_feature=json_int(json_field(object_, "kernelInputFeature")),
        kernel_output_feature=json_int(json_field(object_, "kernelOutputFeature")),
        kernel_spatial=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "kernelSpatial"))
        ],
        output_batch=json_int(json_field(object_, "outputBatch")),
        output_feature=json_int(json_field(object_, "outputFeature")),
        output_spatial=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "outputSpatial"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_convolution_window(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolutionWindow:
        """Decode one TensorConvolutionWindow."""
        return decode_tensor_convolution_window(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_convolution_window(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorConvolutionWindow:
        """Return one TensorConvolutionWindow from one JSON value."""
        return from_json_tensor_convolution_window(value)


def encode_tensor_convolution_window(
    writer: BinaryWriter, value: TensorConvolutionWindow
) -> None:
    """Encode one TensorConvolutionWindow."""
    writer.write_unsigned(len(value.strides))
    for item_value_strides_0 in value.strides:
        writer.write_unsigned(item_value_strides_0)
    writer.write_unsigned(len(value.padding_low))
    for item_value_padding_low_0 in value.padding_low:
        writer.write_unsigned(item_value_padding_low_0)
    writer.write_unsigned(len(value.padding_high))
    for item_value_padding_high_0 in value.padding_high:
        writer.write_unsigned(item_value_padding_high_0)
    writer.write_unsigned(len(value.lhs_dilation))
    for item_value_lhs_dilation_0 in value.lhs_dilation:
        writer.write_unsigned(item_value_lhs_dilation_0)
    writer.write_unsigned(len(value.rhs_dilation))
    for item_value_rhs_dilation_0 in value.rhs_dilation:
        writer.write_unsigned(item_value_rhs_dilation_0)
    writer.write_unsigned(len(value.window_reversal))
    for item_value_window_reversal_0 in value.window_reversal:
        writer.write_bool(item_value_window_reversal_0)


def decode_tensor_convolution_window(reader: BinaryReader) -> TensorConvolutionWindow:
    """Decode one TensorConvolutionWindow."""
    strides = [reader.read_number() for _ in range(reader.read_number())]
    padding_low = [reader.read_number() for _ in range(reader.read_number())]
    padding_high = [reader.read_number() for _ in range(reader.read_number())]
    lhs_dilation = [reader.read_number() for _ in range(reader.read_number())]
    rhs_dilation = [reader.read_number() for _ in range(reader.read_number())]
    window_reversal = [reader.read_bool() for _ in range(reader.read_number())]

    return TensorConvolutionWindow(
        strides=strides,
        padding_low=padding_low,
        padding_high=padding_high,
        lhs_dilation=lhs_dilation,
        rhs_dilation=rhs_dilation,
        window_reversal=window_reversal,
    )


def to_json_tensor_convolution_window(value: TensorConvolutionWindow) -> Json:
    """Return one JSON value for one TensorConvolutionWindow."""
    return {
        "strides": [item_0 for item_0 in value.strides],
        "paddingLow": [item_0 for item_0 in value.padding_low],
        "paddingHigh": [item_0 for item_0 in value.padding_high],
        "lhsDilation": [item_0 for item_0 in value.lhs_dilation],
        "rhsDilation": [item_0 for item_0 in value.rhs_dilation],
        "windowReversal": [item_0 for item_0 in value.window_reversal],
    }


def from_json_tensor_convolution_window(value: Json) -> TensorConvolutionWindow:
    """Return one TensorConvolutionWindow from one JSON value."""
    object_ = json_object(value)

    return TensorConvolutionWindow(
        strides=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "strides"))
        ],
        padding_low=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "paddingLow"))
        ],
        padding_high=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "paddingHigh"))
        ],
        lhs_dilation=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "lhsDilation"))
        ],
        rhs_dilation=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "rhsDilation"))
        ],
        window_reversal=[
            json_bool(item_0)
            for item_0 in json_array(json_field(object_, "windowReversal"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_gather_dimension_numbers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorGatherDimensionNumbers:
        """Decode one TensorGatherDimensionNumbers."""
        return decode_tensor_gather_dimension_numbers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_gather_dimension_numbers(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorGatherDimensionNumbers:
        """Return one TensorGatherDimensionNumbers from one JSON value."""
        return from_json_tensor_gather_dimension_numbers(value)


def encode_tensor_gather_dimension_numbers(
    writer: BinaryWriter, value: TensorGatherDimensionNumbers
) -> None:
    """Encode one TensorGatherDimensionNumbers."""
    writer.write_unsigned(len(value.offset_dims))
    for item_value_offset_dims_0 in value.offset_dims:
        writer.write_unsigned(item_value_offset_dims_0)
    writer.write_unsigned(len(value.collapsed_slice_dims))
    for item_value_collapsed_slice_dims_0 in value.collapsed_slice_dims:
        writer.write_unsigned(item_value_collapsed_slice_dims_0)
    writer.write_unsigned(len(value.start_index_map))
    for item_value_start_index_map_0 in value.start_index_map:
        writer.write_unsigned(item_value_start_index_map_0)
    writer.write_unsigned(value.index_vector_dim)


def decode_tensor_gather_dimension_numbers(
    reader: BinaryReader,
) -> TensorGatherDimensionNumbers:
    """Decode one TensorGatherDimensionNumbers."""
    offset_dims = [reader.read_number() for _ in range(reader.read_number())]
    collapsed_slice_dims = [reader.read_number() for _ in range(reader.read_number())]
    start_index_map = [reader.read_number() for _ in range(reader.read_number())]
    index_vector_dim = reader.read_number()

    return TensorGatherDimensionNumbers(
        offset_dims=offset_dims,
        collapsed_slice_dims=collapsed_slice_dims,
        start_index_map=start_index_map,
        index_vector_dim=index_vector_dim,
    )


def to_json_tensor_gather_dimension_numbers(
    value: TensorGatherDimensionNumbers,
) -> Json:
    """Return one JSON value for one TensorGatherDimensionNumbers."""
    return {
        "offsetDims": [item_0 for item_0 in value.offset_dims],
        "collapsedSliceDims": [item_0 for item_0 in value.collapsed_slice_dims],
        "startIndexMap": [item_0 for item_0 in value.start_index_map],
        "indexVectorDim": value.index_vector_dim,
    }


def from_json_tensor_gather_dimension_numbers(
    value: Json,
) -> TensorGatherDimensionNumbers:
    """Return one TensorGatherDimensionNumbers from one JSON value."""
    object_ = json_object(value)

    return TensorGatherDimensionNumbers(
        offset_dims=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "offsetDims"))
        ],
        collapsed_slice_dims=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "collapsedSliceDims"))
        ],
        start_index_map=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "startIndexMap"))
        ],
        index_vector_dim=json_int(json_field(object_, "indexVectorDim")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_scatter_dimension_numbers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorScatterDimensionNumbers:
        """Decode one TensorScatterDimensionNumbers."""
        return decode_tensor_scatter_dimension_numbers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_scatter_dimension_numbers(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorScatterDimensionNumbers:
        """Return one TensorScatterDimensionNumbers from one JSON value."""
        return from_json_tensor_scatter_dimension_numbers(value)


def encode_tensor_scatter_dimension_numbers(
    writer: BinaryWriter, value: TensorScatterDimensionNumbers
) -> None:
    """Encode one TensorScatterDimensionNumbers."""
    writer.write_unsigned(len(value.update_window_dims))
    for item_value_update_window_dims_0 in value.update_window_dims:
        writer.write_unsigned(item_value_update_window_dims_0)
    writer.write_unsigned(len(value.inserted_window_dims))
    for item_value_inserted_window_dims_0 in value.inserted_window_dims:
        writer.write_unsigned(item_value_inserted_window_dims_0)
    writer.write_unsigned(len(value.scatter_dims_to_operand_dims))
    for item_value_scatter_dims_to_operand_dims_0 in value.scatter_dims_to_operand_dims:
        writer.write_unsigned(item_value_scatter_dims_to_operand_dims_0)
    writer.write_unsigned(value.index_vector_dim)


def decode_tensor_scatter_dimension_numbers(
    reader: BinaryReader,
) -> TensorScatterDimensionNumbers:
    """Decode one TensorScatterDimensionNumbers."""
    update_window_dims = [reader.read_number() for _ in range(reader.read_number())]
    inserted_window_dims = [reader.read_number() for _ in range(reader.read_number())]
    scatter_dims_to_operand_dims = [
        reader.read_number() for _ in range(reader.read_number())
    ]
    index_vector_dim = reader.read_number()

    return TensorScatterDimensionNumbers(
        update_window_dims=update_window_dims,
        inserted_window_dims=inserted_window_dims,
        scatter_dims_to_operand_dims=scatter_dims_to_operand_dims,
        index_vector_dim=index_vector_dim,
    )


def to_json_tensor_scatter_dimension_numbers(
    value: TensorScatterDimensionNumbers,
) -> Json:
    """Return one JSON value for one TensorScatterDimensionNumbers."""
    return {
        "updateWindowDims": [item_0 for item_0 in value.update_window_dims],
        "insertedWindowDims": [item_0 for item_0 in value.inserted_window_dims],
        "scatterDimsToOperandDims": [
            item_0 for item_0 in value.scatter_dims_to_operand_dims
        ],
        "indexVectorDim": value.index_vector_dim,
    }


def from_json_tensor_scatter_dimension_numbers(
    value: Json,
) -> TensorScatterDimensionNumbers:
    """Return one TensorScatterDimensionNumbers from one JSON value."""
    object_ = json_object(value)

    return TensorScatterDimensionNumbers(
        update_window_dims=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "updateWindowDims"))
        ],
        inserted_window_dims=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "insertedWindowDims"))
        ],
        scatter_dims_to_operand_dims=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "scatterDimsToOperandDims"))
        ],
        index_vector_dim=json_int(json_field(object_, "indexVectorDim")),
    )


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
