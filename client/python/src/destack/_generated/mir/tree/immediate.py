# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class IndexSlice:
    """Compact reference to an index list stored in the MIR tree."""

    # start index in the index buffer
    start: int
    # number of indices in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_index_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndexSlice:
        """Decode one IndexSlice."""
        return decode_index_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_index_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> IndexSlice:
        """Return one IndexSlice from one JSON value."""
        return from_json_index_slice(value)


def encode_index_slice(writer: BinaryWriter, value: IndexSlice) -> None:
    """Encode one IndexSlice."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.count)


def decode_index_slice(reader: BinaryReader) -> IndexSlice:
    """Decode one IndexSlice."""
    start = reader.read_number()
    count = reader.read_number()

    return IndexSlice(
        start=start,
        count=count,
    )


def to_json_index_slice(value: IndexSlice) -> Json:
    """Return one JSON value for one IndexSlice."""
    return {
        "start": value.start,
        "count": value.count,
    }


def from_json_index_slice(value: Json) -> IndexSlice:
    """Return one IndexSlice from one JSON value."""
    object_ = json_object(value)

    return IndexSlice(
        start=json_int(json_field(object_, "start")),
        count=json_int(json_field(object_, "count")),
    )


"""Compact identity for one tensor immediate stored in the MIR tree."""
TensorImmediateId: typing.TypeAlias = int


def encode_tensor_immediate_id(writer: BinaryWriter, value: TensorImmediateId) -> None:
    """Encode one TensorImmediateId."""
    writer.write_unsigned(value)


def decode_tensor_immediate_id(reader: BinaryReader) -> TensorImmediateId:
    """Decode one TensorImmediateId."""
    return reader.read_number()


def to_json_tensor_immediate_id(value: TensorImmediateId) -> Json:
    """Return one JSON value for one TensorImmediateId."""
    return value


def from_json_tensor_immediate_id(value: Json) -> TensorImmediateId:
    """Return one TensorImmediateId from one JSON value."""
    return json_int(value)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_immediate(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_immediate(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_immediate(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_immediate(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_immediate(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_immediate(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_immediate(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_immediate(self)


"""Static tensor instruction immediate."""
TensorImmediate: typing.TypeAlias = (
    TensorImmediateDot
    | TensorImmediateConvolution
    | TensorImmediateGather
    | TensorImmediateScatter
)


def encode_tensor_immediate(writer: BinaryWriter, value: TensorImmediate) -> None:
    """Encode one TensorImmediate."""
    if value.kind == "dot":
        writer.write_unsigned(0)
        encode_index_slice(writer, value.lhs_batch)
        encode_index_slice(writer, value.rhs_batch)
        encode_index_slice(writer, value.lhs_contracting)
        encode_index_slice(writer, value.rhs_contracting)
    elif value.kind == "convolution":
        writer.write_unsigned(1)
        writer.write_unsigned(value.input_batch)
        writer.write_unsigned(value.input_feature)
        encode_index_slice(writer, value.input_spatial)
        writer.write_unsigned(value.kernel_input_feature)
        writer.write_unsigned(value.kernel_output_feature)
        encode_index_slice(writer, value.kernel_spatial)
        writer.write_unsigned(value.output_batch)
        writer.write_unsigned(value.output_feature)
        encode_index_slice(writer, value.output_spatial)
        encode_extent_slice(writer, value.strides)
        encode_extent_slice(writer, value.padding_low)
        encode_extent_slice(writer, value.padding_high)
        encode_extent_slice(writer, value.lhs_dilation)
        encode_extent_slice(writer, value.rhs_dilation)
        encode_flag_slice(writer, value.window_reversal)
        writer.write_unsigned(value.feature_group_count)
        writer.write_unsigned(value.batch_group_count)
    elif value.kind == "gather":
        writer.write_unsigned(2)
        encode_index_slice(writer, value.offset_dims)
        encode_index_slice(writer, value.collapsed_slice_dims)
        encode_index_slice(writer, value.start_index_map)
        writer.write_unsigned(value.index_vector_dim)
        encode_index_slice(writer, value.slice_sizes)
    elif value.kind == "scatter":
        writer.write_unsigned(3)
        encode_index_slice(writer, value.update_window_dims)
        encode_index_slice(writer, value.inserted_window_dims)
        encode_index_slice(writer, value.scatter_dims_to_operand_dims)
        writer.write_unsigned(value.index_vector_dim)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_immediate(reader: BinaryReader) -> TensorImmediate:
    """Decode one TensorImmediate."""
    variant = reader.read_number()

    if variant == 0:
        lhs_batch = decode_index_slice(reader)
        rhs_batch = decode_index_slice(reader)
        lhs_contracting = decode_index_slice(reader)
        rhs_contracting = decode_index_slice(reader)

        return TensorImmediateDot(
            lhs_batch=lhs_batch,
            rhs_batch=rhs_batch,
            lhs_contracting=lhs_contracting,
            rhs_contracting=rhs_contracting,
        )
    elif variant == 1:
        input_batch = reader.read_number()
        input_feature = reader.read_number()
        input_spatial = decode_index_slice(reader)
        kernel_input_feature = reader.read_number()
        kernel_output_feature = reader.read_number()
        kernel_spatial = decode_index_slice(reader)
        output_batch = reader.read_number()
        output_feature = reader.read_number()
        output_spatial = decode_index_slice(reader)
        strides = decode_extent_slice(reader)
        padding_low = decode_extent_slice(reader)
        padding_high = decode_extent_slice(reader)
        lhs_dilation = decode_extent_slice(reader)
        rhs_dilation = decode_extent_slice(reader)
        window_reversal = decode_flag_slice(reader)
        feature_group_count = reader.read_number()
        batch_group_count = reader.read_number()

        return TensorImmediateConvolution(
            input_batch=input_batch,
            input_feature=input_feature,
            input_spatial=input_spatial,
            kernel_input_feature=kernel_input_feature,
            kernel_output_feature=kernel_output_feature,
            kernel_spatial=kernel_spatial,
            output_batch=output_batch,
            output_feature=output_feature,
            output_spatial=output_spatial,
            strides=strides,
            padding_low=padding_low,
            padding_high=padding_high,
            lhs_dilation=lhs_dilation,
            rhs_dilation=rhs_dilation,
            window_reversal=window_reversal,
            feature_group_count=feature_group_count,
            batch_group_count=batch_group_count,
        )
    elif variant == 2:
        offset_dims = decode_index_slice(reader)
        collapsed_slice_dims = decode_index_slice(reader)
        start_index_map = decode_index_slice(reader)
        index_vector_dim = reader.read_number()
        slice_sizes = decode_index_slice(reader)

        return TensorImmediateGather(
            offset_dims=offset_dims,
            collapsed_slice_dims=collapsed_slice_dims,
            start_index_map=start_index_map,
            index_vector_dim=index_vector_dim,
            slice_sizes=slice_sizes,
        )
    elif variant == 3:
        update_window_dims = decode_index_slice(reader)
        inserted_window_dims = decode_index_slice(reader)
        scatter_dims_to_operand_dims = decode_index_slice(reader)
        index_vector_dim = reader.read_number()

        return TensorImmediateScatter(
            update_window_dims=update_window_dims,
            inserted_window_dims=inserted_window_dims,
            scatter_dims_to_operand_dims=scatter_dims_to_operand_dims,
            index_vector_dim=index_vector_dim,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_immediate(value: TensorImmediate) -> Json:
    """Return one JSON value for one TensorImmediate."""
    if value.kind == "dot":
        return {
            "kind": "dot",
            "lhsBatch": to_json_index_slice(value.lhs_batch),
            "rhsBatch": to_json_index_slice(value.rhs_batch),
            "lhsContracting": to_json_index_slice(value.lhs_contracting),
            "rhsContracting": to_json_index_slice(value.rhs_contracting),
        }
    elif value.kind == "convolution":
        return {
            "kind": "convolution",
            "inputBatch": value.input_batch,
            "inputFeature": value.input_feature,
            "inputSpatial": to_json_index_slice(value.input_spatial),
            "kernelInputFeature": value.kernel_input_feature,
            "kernelOutputFeature": value.kernel_output_feature,
            "kernelSpatial": to_json_index_slice(value.kernel_spatial),
            "outputBatch": value.output_batch,
            "outputFeature": value.output_feature,
            "outputSpatial": to_json_index_slice(value.output_spatial),
            "strides": to_json_extent_slice(value.strides),
            "paddingLow": to_json_extent_slice(value.padding_low),
            "paddingHigh": to_json_extent_slice(value.padding_high),
            "lhsDilation": to_json_extent_slice(value.lhs_dilation),
            "rhsDilation": to_json_extent_slice(value.rhs_dilation),
            "windowReversal": to_json_flag_slice(value.window_reversal),
            "featureGroupCount": value.feature_group_count,
            "batchGroupCount": value.batch_group_count,
        }
    elif value.kind == "gather":
        return {
            "kind": "gather",
            "offsetDims": to_json_index_slice(value.offset_dims),
            "collapsedSliceDims": to_json_index_slice(value.collapsed_slice_dims),
            "startIndexMap": to_json_index_slice(value.start_index_map),
            "indexVectorDim": value.index_vector_dim,
            "sliceSizes": to_json_index_slice(value.slice_sizes),
        }
    elif value.kind == "scatter":
        return {
            "kind": "scatter",
            "updateWindowDims": to_json_index_slice(value.update_window_dims),
            "insertedWindowDims": to_json_index_slice(value.inserted_window_dims),
            "scatterDimsToOperandDims": to_json_index_slice(
                value.scatter_dims_to_operand_dims
            ),
            "indexVectorDim": value.index_vector_dim,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_immediate(value: Json) -> TensorImmediate:
    """Return one TensorImmediate from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dot":
        return TensorImmediateDot(
            lhs_batch=from_json_index_slice(json_field(object_, "lhsBatch")),
            rhs_batch=from_json_index_slice(json_field(object_, "rhsBatch")),
            lhs_contracting=from_json_index_slice(
                json_field(object_, "lhsContracting")
            ),
            rhs_contracting=from_json_index_slice(
                json_field(object_, "rhsContracting")
            ),
        )
    elif kind == "convolution":
        return TensorImmediateConvolution(
            input_batch=json_int(json_field(object_, "inputBatch")),
            input_feature=json_int(json_field(object_, "inputFeature")),
            input_spatial=from_json_index_slice(json_field(object_, "inputSpatial")),
            kernel_input_feature=json_int(json_field(object_, "kernelInputFeature")),
            kernel_output_feature=json_int(json_field(object_, "kernelOutputFeature")),
            kernel_spatial=from_json_index_slice(json_field(object_, "kernelSpatial")),
            output_batch=json_int(json_field(object_, "outputBatch")),
            output_feature=json_int(json_field(object_, "outputFeature")),
            output_spatial=from_json_index_slice(json_field(object_, "outputSpatial")),
            strides=from_json_extent_slice(json_field(object_, "strides")),
            padding_low=from_json_extent_slice(json_field(object_, "paddingLow")),
            padding_high=from_json_extent_slice(json_field(object_, "paddingHigh")),
            lhs_dilation=from_json_extent_slice(json_field(object_, "lhsDilation")),
            rhs_dilation=from_json_extent_slice(json_field(object_, "rhsDilation")),
            window_reversal=from_json_flag_slice(json_field(object_, "windowReversal")),
            feature_group_count=json_int(json_field(object_, "featureGroupCount")),
            batch_group_count=json_int(json_field(object_, "batchGroupCount")),
        )
    elif kind == "gather":
        return TensorImmediateGather(
            offset_dims=from_json_index_slice(json_field(object_, "offsetDims")),
            collapsed_slice_dims=from_json_index_slice(
                json_field(object_, "collapsedSliceDims")
            ),
            start_index_map=from_json_index_slice(json_field(object_, "startIndexMap")),
            index_vector_dim=json_int(json_field(object_, "indexVectorDim")),
            slice_sizes=from_json_index_slice(json_field(object_, "sliceSizes")),
        )
    elif kind == "scatter":
        return TensorImmediateScatter(
            update_window_dims=from_json_index_slice(
                json_field(object_, "updateWindowDims")
            ),
            inserted_window_dims=from_json_index_slice(
                json_field(object_, "insertedWindowDims")
            ),
            scatter_dims_to_operand_dims=from_json_index_slice(
                json_field(object_, "scatterDimsToOperandDims")
            ),
            index_vector_dim=json_int(json_field(object_, "indexVectorDim")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ExtentSlice:
    """Compact reference to an extent list stored in the MIR tree."""

    # start index in the extent buffer
    start: int
    # number of extents in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extent_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtentSlice:
        """Decode one ExtentSlice."""
        return decode_extent_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extent_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtentSlice:
        """Return one ExtentSlice from one JSON value."""
        return from_json_extent_slice(value)


def encode_extent_slice(writer: BinaryWriter, value: ExtentSlice) -> None:
    """Encode one ExtentSlice."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.count)


def decode_extent_slice(reader: BinaryReader) -> ExtentSlice:
    """Decode one ExtentSlice."""
    start = reader.read_number()
    count = reader.read_number()

    return ExtentSlice(
        start=start,
        count=count,
    )


def to_json_extent_slice(value: ExtentSlice) -> Json:
    """Return one JSON value for one ExtentSlice."""
    return {
        "start": value.start,
        "count": value.count,
    }


def from_json_extent_slice(value: Json) -> ExtentSlice:
    """Return one ExtentSlice from one JSON value."""
    object_ = json_object(value)

    return ExtentSlice(
        start=json_int(json_field(object_, "start")),
        count=json_int(json_field(object_, "count")),
    )


@dataclass(frozen=True, slots=True)
class FlagSlice:
    """Compact reference to a flag list stored in the MIR tree."""

    # start index in the flag buffer
    start: int
    # number of flags in the slice
    count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_flag_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FlagSlice:
        """Decode one FlagSlice."""
        return decode_flag_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_flag_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> FlagSlice:
        """Return one FlagSlice from one JSON value."""
        return from_json_flag_slice(value)


def encode_flag_slice(writer: BinaryWriter, value: FlagSlice) -> None:
    """Encode one FlagSlice."""
    writer.write_unsigned(value.start)
    writer.write_unsigned(value.count)


def decode_flag_slice(reader: BinaryReader) -> FlagSlice:
    """Decode one FlagSlice."""
    start = reader.read_number()
    count = reader.read_number()

    return FlagSlice(
        start=start,
        count=count,
    )


def to_json_flag_slice(value: FlagSlice) -> Json:
    """Return one JSON value for one FlagSlice."""
    return {
        "start": value.start,
        "count": value.count,
    }


def from_json_flag_slice(value: Json) -> FlagSlice:
    """Return one FlagSlice from one JSON value."""
    object_ = json_object(value)

    return FlagSlice(
        start=json_int(json_field(object_, "start")),
        count=json_int(json_field(object_, "count")),
    )


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
