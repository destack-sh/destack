# generated bridge target, do not edit

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
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.tree.tensor
import destack._generated.program.vm.allocation
import destack._generated.program.vm.constant
import destack._generated.program.vm.function
import destack._generated.program.vm.projection
import destack._generated.program.vm.range
import destack._generated.program.vm.side
import destack._generated.program.vm.tensor


@dataclass(frozen=True, slots=True)
class SideTable:
    """Immutable side table referenced by compact side records."""

    # pooled side records
    record: SideRecordTable
    # pooled allocation sites
    allocation_site: Sequence[destack._generated.program.vm.allocation.AllocationSite]
    # pooled small allocation sites
    small_allocation_site: Sequence[
        destack._generated.program.vm.allocation.SmallAllocationSite
    ]
    # pooled constants
    constant: Sequence[destack._generated.program.vm.constant.ConstValue]
    # pooled address projections
    projection: Sequence[destack._generated.program.vm.projection.Projection]
    # pooled slice projectiones
    slice_projection: Sequence[destack._generated.program.vm.projection.SliceProjection]
    # pooled check constraints
    check: Sequence[Check]
    # pooled switch case tables
    switch_cases: Sequence[Sequence[destack._generated.program.vm.function.SwitchCase]]
    # pooled dense switch tables
    switch_table: Sequence[SwitchTable]
    # pooled control edges
    edge: Sequence[Edge]
    # pooled u32 slices
    u32_ranges: Sequence[Sequence[int]]
    # pooled tensor dot descriptors
    tensor_dot: Sequence[destack._generated.mir.tree.tensor.TensorDotDimensionNumbers]
    # pooled tensor convolution dimension descriptors
    tensor_convolution: Sequence[
        destack._generated.mir.tree.tensor.TensorConvolutionDimensionNumbers
    ]
    # pooled tensor convolution window descriptors
    tensor_window: Sequence[destack._generated.mir.tree.tensor.TensorConvolutionWindow]
    # pooled tensor gather descriptors
    tensor_gather: Sequence[
        destack._generated.mir.tree.tensor.TensorGatherDimensionNumbers
    ]
    # pooled tensor scatter descriptors
    tensor_scatter: Sequence[
        destack._generated.mir.tree.tensor.TensorScatterDimensionNumbers
    ]
    # pooled tensor layouts
    tensor_layout: Sequence[destack._generated.program.vm.tensor.TensorLayout]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_side_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SideTable:
        """Decode one SideTable."""
        return decode_side_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_side_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SideTable:
        """Return one SideTable from one JSON value."""
        return from_json_side_table(value)


def encode_side_table(writer: BinaryWriter, value: SideTable) -> None:
    """Encode one SideTable."""
    encode_side_record_table(writer, value.record)
    writer.write_unsigned(len(value.allocation_site))
    for item_value_allocation_site_0 in value.allocation_site:
        destack._generated.program.vm.allocation.encode_allocation_site(
            writer, item_value_allocation_site_0
        )
    writer.write_unsigned(len(value.small_allocation_site))
    for item_value_small_allocation_site_0 in value.small_allocation_site:
        destack._generated.program.vm.allocation.encode_small_allocation_site(
            writer, item_value_small_allocation_site_0
        )
    writer.write_unsigned(len(value.constant))
    for item_value_constant_0 in value.constant:
        destack._generated.program.vm.constant.encode_const_value(
            writer, item_value_constant_0
        )
    writer.write_unsigned(len(value.projection))
    for item_value_projection_0 in value.projection:
        destack._generated.program.vm.projection.encode_projection(
            writer, item_value_projection_0
        )
    writer.write_unsigned(len(value.slice_projection))
    for item_value_slice_projection_0 in value.slice_projection:
        destack._generated.program.vm.projection.encode_slice_projection(
            writer, item_value_slice_projection_0
        )
    writer.write_unsigned(len(value.check))
    for item_value_check_0 in value.check:
        encode_check(writer, item_value_check_0)
    writer.write_unsigned(len(value.switch_cases))
    for item_value_switch_cases_0 in value.switch_cases:
        writer.write_unsigned(len(item_value_switch_cases_0))
        for item_item_value_switch_cases_0_1 in item_value_switch_cases_0:
            destack._generated.program.vm.function.encode_switch_case(
                writer, item_item_value_switch_cases_0_1
            )
    writer.write_unsigned(len(value.switch_table))
    for item_value_switch_table_0 in value.switch_table:
        encode_switch_table(writer, item_value_switch_table_0)
    writer.write_unsigned(len(value.edge))
    for item_value_edge_0 in value.edge:
        encode_edge(writer, item_value_edge_0)
    writer.write_unsigned(len(value.u32_ranges))
    for item_value_u32_ranges_0 in value.u32_ranges:
        writer.write_unsigned(len(item_value_u32_ranges_0))
        for item_item_value_u32_ranges_0_1 in item_value_u32_ranges_0:
            writer.write_unsigned(item_item_value_u32_ranges_0_1)
    writer.write_unsigned(len(value.tensor_dot))
    for item_value_tensor_dot_0 in value.tensor_dot:
        destack._generated.mir.tree.tensor.encode_tensor_dot_dimension_numbers(
            writer, item_value_tensor_dot_0
        )
    writer.write_unsigned(len(value.tensor_convolution))
    for item_value_tensor_convolution_0 in value.tensor_convolution:
        destack._generated.mir.tree.tensor.encode_tensor_convolution_dimension_numbers(
            writer, item_value_tensor_convolution_0
        )
    writer.write_unsigned(len(value.tensor_window))
    for item_value_tensor_window_0 in value.tensor_window:
        destack._generated.mir.tree.tensor.encode_tensor_convolution_window(
            writer, item_value_tensor_window_0
        )
    writer.write_unsigned(len(value.tensor_gather))
    for item_value_tensor_gather_0 in value.tensor_gather:
        destack._generated.mir.tree.tensor.encode_tensor_gather_dimension_numbers(
            writer, item_value_tensor_gather_0
        )
    writer.write_unsigned(len(value.tensor_scatter))
    for item_value_tensor_scatter_0 in value.tensor_scatter:
        destack._generated.mir.tree.tensor.encode_tensor_scatter_dimension_numbers(
            writer, item_value_tensor_scatter_0
        )
    writer.write_unsigned(len(value.tensor_layout))
    for item_value_tensor_layout_0 in value.tensor_layout:
        destack._generated.program.vm.tensor.encode_tensor_layout(
            writer, item_value_tensor_layout_0
        )


def decode_side_table(reader: BinaryReader) -> SideTable:
    """Decode one SideTable."""
    record = decode_side_record_table(reader)
    allocation_site = [
        destack._generated.program.vm.allocation.decode_allocation_site(reader)
        for _ in range(reader.read_number())
    ]
    small_allocation_site = [
        destack._generated.program.vm.allocation.decode_small_allocation_site(reader)
        for _ in range(reader.read_number())
    ]
    constant = [
        destack._generated.program.vm.constant.decode_const_value(reader)
        for _ in range(reader.read_number())
    ]
    projection = [
        destack._generated.program.vm.projection.decode_projection(reader)
        for _ in range(reader.read_number())
    ]
    slice_projection = [
        destack._generated.program.vm.projection.decode_slice_projection(reader)
        for _ in range(reader.read_number())
    ]
    check = [decode_check(reader) for _ in range(reader.read_number())]
    switch_cases = [
        [
            destack._generated.program.vm.function.decode_switch_case(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    ]
    switch_table = [decode_switch_table(reader) for _ in range(reader.read_number())]
    edge = [decode_edge(reader) for _ in range(reader.read_number())]
    u32_ranges = [
        [reader.read_number() for _ in range(reader.read_number())]
        for _ in range(reader.read_number())
    ]
    tensor_dot = [
        destack._generated.mir.tree.tensor.decode_tensor_dot_dimension_numbers(reader)
        for _ in range(reader.read_number())
    ]
    tensor_convolution = [
        destack._generated.mir.tree.tensor.decode_tensor_convolution_dimension_numbers(
            reader
        )
        for _ in range(reader.read_number())
    ]
    tensor_window = [
        destack._generated.mir.tree.tensor.decode_tensor_convolution_window(reader)
        for _ in range(reader.read_number())
    ]
    tensor_gather = [
        destack._generated.mir.tree.tensor.decode_tensor_gather_dimension_numbers(
            reader
        )
        for _ in range(reader.read_number())
    ]
    tensor_scatter = [
        destack._generated.mir.tree.tensor.decode_tensor_scatter_dimension_numbers(
            reader
        )
        for _ in range(reader.read_number())
    ]
    tensor_layout = [
        destack._generated.program.vm.tensor.decode_tensor_layout(reader)
        for _ in range(reader.read_number())
    ]

    return SideTable(
        record=record,
        allocation_site=allocation_site,
        small_allocation_site=small_allocation_site,
        constant=constant,
        projection=projection,
        slice_projection=slice_projection,
        check=check,
        switch_cases=switch_cases,
        switch_table=switch_table,
        edge=edge,
        u32_ranges=u32_ranges,
        tensor_dot=tensor_dot,
        tensor_convolution=tensor_convolution,
        tensor_window=tensor_window,
        tensor_gather=tensor_gather,
        tensor_scatter=tensor_scatter,
        tensor_layout=tensor_layout,
    )


def to_json_side_table(value: SideTable) -> Json:
    """Return one JSON value for one SideTable."""
    return {
        "record": to_json_side_record_table(value.record),
        "allocationSite": [
            destack._generated.program.vm.allocation.to_json_allocation_site(item_0)
            for item_0 in value.allocation_site
        ],
        "smallAllocationSite": [
            destack._generated.program.vm.allocation.to_json_small_allocation_site(
                item_0
            )
            for item_0 in value.small_allocation_site
        ],
        "constant": [
            destack._generated.program.vm.constant.to_json_const_value(item_0)
            for item_0 in value.constant
        ],
        "projection": [
            destack._generated.program.vm.projection.to_json_projection(item_0)
            for item_0 in value.projection
        ],
        "sliceProjection": [
            destack._generated.program.vm.projection.to_json_slice_projection(item_0)
            for item_0 in value.slice_projection
        ],
        "check": [to_json_check(item_0) for item_0 in value.check],
        "switchCases": [
            [
                destack._generated.program.vm.function.to_json_switch_case(item_1)
                for item_1 in item_0
            ]
            for item_0 in value.switch_cases
        ],
        "switchTable": [to_json_switch_table(item_0) for item_0 in value.switch_table],
        "edge": [to_json_edge(item_0) for item_0 in value.edge],
        "u32Ranges": [[item_1 for item_1 in item_0] for item_0 in value.u32_ranges],
        "tensorDot": [
            destack._generated.mir.tree.tensor.to_json_tensor_dot_dimension_numbers(
                item_0
            )
            for item_0 in value.tensor_dot
        ],
        "tensorConvolution": [
            destack._generated.mir.tree.tensor.to_json_tensor_convolution_dimension_numbers(
                item_0
            )
            for item_0 in value.tensor_convolution
        ],
        "tensorWindow": [
            destack._generated.mir.tree.tensor.to_json_tensor_convolution_window(item_0)
            for item_0 in value.tensor_window
        ],
        "tensorGather": [
            destack._generated.mir.tree.tensor.to_json_tensor_gather_dimension_numbers(
                item_0
            )
            for item_0 in value.tensor_gather
        ],
        "tensorScatter": [
            destack._generated.mir.tree.tensor.to_json_tensor_scatter_dimension_numbers(
                item_0
            )
            for item_0 in value.tensor_scatter
        ],
        "tensorLayout": [
            destack._generated.program.vm.tensor.to_json_tensor_layout(item_0)
            for item_0 in value.tensor_layout
        ],
    }


def from_json_side_table(value: Json) -> SideTable:
    """Return one SideTable from one JSON value."""
    object_ = json_object(value)

    return SideTable(
        record=from_json_side_record_table(json_field(object_, "record")),
        allocation_site=[
            destack._generated.program.vm.allocation.from_json_allocation_site(item_0)
            for item_0 in json_array(json_field(object_, "allocationSite"))
        ],
        small_allocation_site=[
            destack._generated.program.vm.allocation.from_json_small_allocation_site(
                item_0
            )
            for item_0 in json_array(json_field(object_, "smallAllocationSite"))
        ],
        constant=[
            destack._generated.program.vm.constant.from_json_const_value(item_0)
            for item_0 in json_array(json_field(object_, "constant"))
        ],
        projection=[
            destack._generated.program.vm.projection.from_json_projection(item_0)
            for item_0 in json_array(json_field(object_, "projection"))
        ],
        slice_projection=[
            destack._generated.program.vm.projection.from_json_slice_projection(item_0)
            for item_0 in json_array(json_field(object_, "sliceProjection"))
        ],
        check=[
            from_json_check(item_0)
            for item_0 in json_array(json_field(object_, "check"))
        ],
        switch_cases=[
            [
                destack._generated.program.vm.function.from_json_switch_case(item_1)
                for item_1 in json_array(item_0)
            ]
            for item_0 in json_array(json_field(object_, "switchCases"))
        ],
        switch_table=[
            from_json_switch_table(item_0)
            for item_0 in json_array(json_field(object_, "switchTable"))
        ],
        edge=[
            from_json_edge(item_0) for item_0 in json_array(json_field(object_, "edge"))
        ],
        u32_ranges=[
            [json_int(item_1) for item_1 in json_array(item_0)]
            for item_0 in json_array(json_field(object_, "u32Ranges"))
        ],
        tensor_dot=[
            destack._generated.mir.tree.tensor.from_json_tensor_dot_dimension_numbers(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorDot"))
        ],
        tensor_convolution=[
            destack._generated.mir.tree.tensor.from_json_tensor_convolution_dimension_numbers(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorConvolution"))
        ],
        tensor_window=[
            destack._generated.mir.tree.tensor.from_json_tensor_convolution_window(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorWindow"))
        ],
        tensor_gather=[
            destack._generated.mir.tree.tensor.from_json_tensor_gather_dimension_numbers(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorGather"))
        ],
        tensor_scatter=[
            destack._generated.mir.tree.tensor.from_json_tensor_scatter_dimension_numbers(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorScatter"))
        ],
        tensor_layout=[
            destack._generated.program.vm.tensor.from_json_tensor_layout(item_0)
            for item_0 in json_array(json_field(object_, "tensorLayout"))
        ],
    )


@dataclass(frozen=True, slots=True)
class SideRecordTable:
    """Immutable side records referenced by instruction ids."""

    aggregate_select: Sequence[destack._generated.program.vm.side.AggregateSelect]
    allocation_branch: Sequence[
        destack._generated.program.vm.allocation.AllocationBranch
    ]
    slice_allocation_branch: Sequence[
        destack._generated.program.vm.allocation.SliceAllocationBranch
    ]
    atomic_compare_exchange: Sequence[
        destack._generated.program.vm.side.AtomicCompareExchange
    ]
    vector_splat: Sequence[destack._generated.program.vm.side.VectorSplat]
    vector_extract: Sequence[destack._generated.program.vm.side.VectorExtract]
    vector_binary: Sequence[destack._generated.program.vm.side.VectorBinary]
    vector_unary: Sequence[destack._generated.program.vm.side.VectorUnary]
    vector_insert: Sequence[destack._generated.program.vm.side.VectorInsert]
    vector_shuffle: Sequence[destack._generated.program.vm.side.VectorShuffle]
    vector_select: Sequence[destack._generated.program.vm.side.VectorSelect]
    vector_reduce: Sequence[destack._generated.program.vm.side.VectorReduce]
    vector_convert: Sequence[destack._generated.program.vm.side.VectorConvert]
    function_bind: Sequence[destack._generated.program.vm.side.FunctionBind]
    call: Sequence[destack._generated.program.vm.side.Call]
    call_branch: Sequence[destack._generated.program.vm.side.CallBranch]
    call_virtual: Sequence[destack._generated.program.vm.side.CallVirtual]
    call_virtual_branch: Sequence[destack._generated.program.vm.side.CallVirtualBranch]
    call_dynamic: Sequence[destack._generated.program.vm.side.CallDynamic]
    call_dynamic_branch: Sequence[destack._generated.program.vm.side.CallDynamicBranch]
    indirect_call: Sequence[destack._generated.program.vm.side.IndirectCall]
    indirect_call_branch: Sequence[
        destack._generated.program.vm.side.IndirectCallBranch
    ]
    tensor_load: Sequence[destack._generated.program.vm.side.TensorLoad]
    tensor_extract: Sequence[destack._generated.program.vm.side.TensorExtract]
    tensor_binary: Sequence[destack._generated.program.vm.side.TensorBinary]
    tensor_contiguous_binary: Sequence[
        destack._generated.program.vm.side.TensorContiguousBinary
    ]
    tensor_unary: Sequence[destack._generated.program.vm.side.TensorUnary]
    tensor_contiguous_unary: Sequence[
        destack._generated.program.vm.side.TensorContiguousUnary
    ]
    tensor_store: Sequence[destack._generated.program.vm.side.TensorStore]
    tensor_fill: Sequence[destack._generated.program.vm.side.TensorFill]
    tensor_copy: Sequence[destack._generated.program.vm.side.TensorCopy]
    tensor_view_cast: Sequence[destack._generated.program.vm.side.TensorViewCast]
    tensor_reshape: Sequence[destack._generated.program.vm.side.TensorReshape]
    tensor_broadcast: Sequence[destack._generated.program.vm.side.TensorBroadcast]
    tensor_transpose: Sequence[destack._generated.program.vm.side.TensorTranspose]
    tensor_slice: Sequence[destack._generated.program.vm.side.TensorSlice]
    tensor_pad: Sequence[destack._generated.program.vm.side.TensorPad]
    tensor_concat: Sequence[destack._generated.program.vm.side.TensorConcat]
    tensor_reduce: Sequence[destack._generated.program.vm.side.TensorReduce]
    tensor_index_reduce: Sequence[destack._generated.program.vm.side.TensorIndexReduce]
    tensor_dot: Sequence[destack._generated.program.vm.side.TensorDot]
    tensor_convolution: Sequence[destack._generated.program.vm.side.TensorConvolution]
    tensor_gather: Sequence[destack._generated.program.vm.side.TensorGather]
    tensor_scatter: Sequence[destack._generated.program.vm.side.TensorScatter]
    tensor_select: Sequence[destack._generated.program.vm.side.TensorSelect]
    tensor_convert: Sequence[destack._generated.program.vm.side.TensorConvert]
    tensor_view: Sequence[destack._generated.program.vm.side.TensorView]
    intrinsic: Sequence[destack._generated.program.vm.side.Intrinsic]
    tail_call: Sequence[destack._generated.program.vm.side.TailCall]
    tail_call_virtual: Sequence[destack._generated.program.vm.side.TailCallVirtual]
    tail_call_dynamic: Sequence[destack._generated.program.vm.side.TailCallDynamic]
    indirect_tail_call: Sequence[destack._generated.program.vm.side.IndirectTailCall]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_side_record_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SideRecordTable:
        """Decode one SideRecordTable."""
        return decode_side_record_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_side_record_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SideRecordTable:
        """Return one SideRecordTable from one JSON value."""
        return from_json_side_record_table(value)


def encode_side_record_table(writer: BinaryWriter, value: SideRecordTable) -> None:
    """Encode one SideRecordTable."""
    writer.write_unsigned(len(value.aggregate_select))
    for item_value_aggregate_select_0 in value.aggregate_select:
        destack._generated.program.vm.side.encode_aggregate_select(
            writer, item_value_aggregate_select_0
        )
    writer.write_unsigned(len(value.allocation_branch))
    for item_value_allocation_branch_0 in value.allocation_branch:
        destack._generated.program.vm.allocation.encode_allocation_branch(
            writer, item_value_allocation_branch_0
        )
    writer.write_unsigned(len(value.slice_allocation_branch))
    for item_value_slice_allocation_branch_0 in value.slice_allocation_branch:
        destack._generated.program.vm.allocation.encode_slice_allocation_branch(
            writer, item_value_slice_allocation_branch_0
        )
    writer.write_unsigned(len(value.atomic_compare_exchange))
    for item_value_atomic_compare_exchange_0 in value.atomic_compare_exchange:
        destack._generated.program.vm.side.encode_atomic_compare_exchange(
            writer, item_value_atomic_compare_exchange_0
        )
    writer.write_unsigned(len(value.vector_splat))
    for item_value_vector_splat_0 in value.vector_splat:
        destack._generated.program.vm.side.encode_vector_splat(
            writer, item_value_vector_splat_0
        )
    writer.write_unsigned(len(value.vector_extract))
    for item_value_vector_extract_0 in value.vector_extract:
        destack._generated.program.vm.side.encode_vector_extract(
            writer, item_value_vector_extract_0
        )
    writer.write_unsigned(len(value.vector_binary))
    for item_value_vector_binary_0 in value.vector_binary:
        destack._generated.program.vm.side.encode_vector_binary(
            writer, item_value_vector_binary_0
        )
    writer.write_unsigned(len(value.vector_unary))
    for item_value_vector_unary_0 in value.vector_unary:
        destack._generated.program.vm.side.encode_vector_unary(
            writer, item_value_vector_unary_0
        )
    writer.write_unsigned(len(value.vector_insert))
    for item_value_vector_insert_0 in value.vector_insert:
        destack._generated.program.vm.side.encode_vector_insert(
            writer, item_value_vector_insert_0
        )
    writer.write_unsigned(len(value.vector_shuffle))
    for item_value_vector_shuffle_0 in value.vector_shuffle:
        destack._generated.program.vm.side.encode_vector_shuffle(
            writer, item_value_vector_shuffle_0
        )
    writer.write_unsigned(len(value.vector_select))
    for item_value_vector_select_0 in value.vector_select:
        destack._generated.program.vm.side.encode_vector_select(
            writer, item_value_vector_select_0
        )
    writer.write_unsigned(len(value.vector_reduce))
    for item_value_vector_reduce_0 in value.vector_reduce:
        destack._generated.program.vm.side.encode_vector_reduce(
            writer, item_value_vector_reduce_0
        )
    writer.write_unsigned(len(value.vector_convert))
    for item_value_vector_convert_0 in value.vector_convert:
        destack._generated.program.vm.side.encode_vector_convert(
            writer, item_value_vector_convert_0
        )
    writer.write_unsigned(len(value.function_bind))
    for item_value_function_bind_0 in value.function_bind:
        destack._generated.program.vm.side.encode_function_bind(
            writer, item_value_function_bind_0
        )
    writer.write_unsigned(len(value.call))
    for item_value_call_0 in value.call:
        destack._generated.program.vm.side.encode_call(writer, item_value_call_0)
    writer.write_unsigned(len(value.call_branch))
    for item_value_call_branch_0 in value.call_branch:
        destack._generated.program.vm.side.encode_call_branch(
            writer, item_value_call_branch_0
        )
    writer.write_unsigned(len(value.call_virtual))
    for item_value_call_virtual_0 in value.call_virtual:
        destack._generated.program.vm.side.encode_call_virtual(
            writer, item_value_call_virtual_0
        )
    writer.write_unsigned(len(value.call_virtual_branch))
    for item_value_call_virtual_branch_0 in value.call_virtual_branch:
        destack._generated.program.vm.side.encode_call_virtual_branch(
            writer, item_value_call_virtual_branch_0
        )
    writer.write_unsigned(len(value.call_dynamic))
    for item_value_call_dynamic_0 in value.call_dynamic:
        destack._generated.program.vm.side.encode_call_dynamic(
            writer, item_value_call_dynamic_0
        )
    writer.write_unsigned(len(value.call_dynamic_branch))
    for item_value_call_dynamic_branch_0 in value.call_dynamic_branch:
        destack._generated.program.vm.side.encode_call_dynamic_branch(
            writer, item_value_call_dynamic_branch_0
        )
    writer.write_unsigned(len(value.indirect_call))
    for item_value_indirect_call_0 in value.indirect_call:
        destack._generated.program.vm.side.encode_indirect_call(
            writer, item_value_indirect_call_0
        )
    writer.write_unsigned(len(value.indirect_call_branch))
    for item_value_indirect_call_branch_0 in value.indirect_call_branch:
        destack._generated.program.vm.side.encode_indirect_call_branch(
            writer, item_value_indirect_call_branch_0
        )
    writer.write_unsigned(len(value.tensor_load))
    for item_value_tensor_load_0 in value.tensor_load:
        destack._generated.program.vm.side.encode_tensor_load(
            writer, item_value_tensor_load_0
        )
    writer.write_unsigned(len(value.tensor_extract))
    for item_value_tensor_extract_0 in value.tensor_extract:
        destack._generated.program.vm.side.encode_tensor_extract(
            writer, item_value_tensor_extract_0
        )
    writer.write_unsigned(len(value.tensor_binary))
    for item_value_tensor_binary_0 in value.tensor_binary:
        destack._generated.program.vm.side.encode_tensor_binary(
            writer, item_value_tensor_binary_0
        )
    writer.write_unsigned(len(value.tensor_contiguous_binary))
    for item_value_tensor_contiguous_binary_0 in value.tensor_contiguous_binary:
        destack._generated.program.vm.side.encode_tensor_contiguous_binary(
            writer, item_value_tensor_contiguous_binary_0
        )
    writer.write_unsigned(len(value.tensor_unary))
    for item_value_tensor_unary_0 in value.tensor_unary:
        destack._generated.program.vm.side.encode_tensor_unary(
            writer, item_value_tensor_unary_0
        )
    writer.write_unsigned(len(value.tensor_contiguous_unary))
    for item_value_tensor_contiguous_unary_0 in value.tensor_contiguous_unary:
        destack._generated.program.vm.side.encode_tensor_contiguous_unary(
            writer, item_value_tensor_contiguous_unary_0
        )
    writer.write_unsigned(len(value.tensor_store))
    for item_value_tensor_store_0 in value.tensor_store:
        destack._generated.program.vm.side.encode_tensor_store(
            writer, item_value_tensor_store_0
        )
    writer.write_unsigned(len(value.tensor_fill))
    for item_value_tensor_fill_0 in value.tensor_fill:
        destack._generated.program.vm.side.encode_tensor_fill(
            writer, item_value_tensor_fill_0
        )
    writer.write_unsigned(len(value.tensor_copy))
    for item_value_tensor_copy_0 in value.tensor_copy:
        destack._generated.program.vm.side.encode_tensor_copy(
            writer, item_value_tensor_copy_0
        )
    writer.write_unsigned(len(value.tensor_view_cast))
    for item_value_tensor_view_cast_0 in value.tensor_view_cast:
        destack._generated.program.vm.side.encode_tensor_view_cast(
            writer, item_value_tensor_view_cast_0
        )
    writer.write_unsigned(len(value.tensor_reshape))
    for item_value_tensor_reshape_0 in value.tensor_reshape:
        destack._generated.program.vm.side.encode_tensor_reshape(
            writer, item_value_tensor_reshape_0
        )
    writer.write_unsigned(len(value.tensor_broadcast))
    for item_value_tensor_broadcast_0 in value.tensor_broadcast:
        destack._generated.program.vm.side.encode_tensor_broadcast(
            writer, item_value_tensor_broadcast_0
        )
    writer.write_unsigned(len(value.tensor_transpose))
    for item_value_tensor_transpose_0 in value.tensor_transpose:
        destack._generated.program.vm.side.encode_tensor_transpose(
            writer, item_value_tensor_transpose_0
        )
    writer.write_unsigned(len(value.tensor_slice))
    for item_value_tensor_slice_0 in value.tensor_slice:
        destack._generated.program.vm.side.encode_tensor_slice(
            writer, item_value_tensor_slice_0
        )
    writer.write_unsigned(len(value.tensor_pad))
    for item_value_tensor_pad_0 in value.tensor_pad:
        destack._generated.program.vm.side.encode_tensor_pad(
            writer, item_value_tensor_pad_0
        )
    writer.write_unsigned(len(value.tensor_concat))
    for item_value_tensor_concat_0 in value.tensor_concat:
        destack._generated.program.vm.side.encode_tensor_concat(
            writer, item_value_tensor_concat_0
        )
    writer.write_unsigned(len(value.tensor_reduce))
    for item_value_tensor_reduce_0 in value.tensor_reduce:
        destack._generated.program.vm.side.encode_tensor_reduce(
            writer, item_value_tensor_reduce_0
        )
    writer.write_unsigned(len(value.tensor_index_reduce))
    for item_value_tensor_index_reduce_0 in value.tensor_index_reduce:
        destack._generated.program.vm.side.encode_tensor_index_reduce(
            writer, item_value_tensor_index_reduce_0
        )
    writer.write_unsigned(len(value.tensor_dot))
    for item_value_tensor_dot_0 in value.tensor_dot:
        destack._generated.program.vm.side.encode_tensor_dot(
            writer, item_value_tensor_dot_0
        )
    writer.write_unsigned(len(value.tensor_convolution))
    for item_value_tensor_convolution_0 in value.tensor_convolution:
        destack._generated.program.vm.side.encode_tensor_convolution(
            writer, item_value_tensor_convolution_0
        )
    writer.write_unsigned(len(value.tensor_gather))
    for item_value_tensor_gather_0 in value.tensor_gather:
        destack._generated.program.vm.side.encode_tensor_gather(
            writer, item_value_tensor_gather_0
        )
    writer.write_unsigned(len(value.tensor_scatter))
    for item_value_tensor_scatter_0 in value.tensor_scatter:
        destack._generated.program.vm.side.encode_tensor_scatter(
            writer, item_value_tensor_scatter_0
        )
    writer.write_unsigned(len(value.tensor_select))
    for item_value_tensor_select_0 in value.tensor_select:
        destack._generated.program.vm.side.encode_tensor_select(
            writer, item_value_tensor_select_0
        )
    writer.write_unsigned(len(value.tensor_convert))
    for item_value_tensor_convert_0 in value.tensor_convert:
        destack._generated.program.vm.side.encode_tensor_convert(
            writer, item_value_tensor_convert_0
        )
    writer.write_unsigned(len(value.tensor_view))
    for item_value_tensor_view_0 in value.tensor_view:
        destack._generated.program.vm.side.encode_tensor_view(
            writer, item_value_tensor_view_0
        )
    writer.write_unsigned(len(value.intrinsic))
    for item_value_intrinsic_0 in value.intrinsic:
        destack._generated.program.vm.side.encode_intrinsic(
            writer, item_value_intrinsic_0
        )
    writer.write_unsigned(len(value.tail_call))
    for item_value_tail_call_0 in value.tail_call:
        destack._generated.program.vm.side.encode_tail_call(
            writer, item_value_tail_call_0
        )
    writer.write_unsigned(len(value.tail_call_virtual))
    for item_value_tail_call_virtual_0 in value.tail_call_virtual:
        destack._generated.program.vm.side.encode_tail_call_virtual(
            writer, item_value_tail_call_virtual_0
        )
    writer.write_unsigned(len(value.tail_call_dynamic))
    for item_value_tail_call_dynamic_0 in value.tail_call_dynamic:
        destack._generated.program.vm.side.encode_tail_call_dynamic(
            writer, item_value_tail_call_dynamic_0
        )
    writer.write_unsigned(len(value.indirect_tail_call))
    for item_value_indirect_tail_call_0 in value.indirect_tail_call:
        destack._generated.program.vm.side.encode_indirect_tail_call(
            writer, item_value_indirect_tail_call_0
        )


def decode_side_record_table(reader: BinaryReader) -> SideRecordTable:
    """Decode one SideRecordTable."""
    aggregate_select = [
        destack._generated.program.vm.side.decode_aggregate_select(reader)
        for _ in range(reader.read_number())
    ]
    allocation_branch = [
        destack._generated.program.vm.allocation.decode_allocation_branch(reader)
        for _ in range(reader.read_number())
    ]
    slice_allocation_branch = [
        destack._generated.program.vm.allocation.decode_slice_allocation_branch(reader)
        for _ in range(reader.read_number())
    ]
    atomic_compare_exchange = [
        destack._generated.program.vm.side.decode_atomic_compare_exchange(reader)
        for _ in range(reader.read_number())
    ]
    vector_splat = [
        destack._generated.program.vm.side.decode_vector_splat(reader)
        for _ in range(reader.read_number())
    ]
    vector_extract = [
        destack._generated.program.vm.side.decode_vector_extract(reader)
        for _ in range(reader.read_number())
    ]
    vector_binary = [
        destack._generated.program.vm.side.decode_vector_binary(reader)
        for _ in range(reader.read_number())
    ]
    vector_unary = [
        destack._generated.program.vm.side.decode_vector_unary(reader)
        for _ in range(reader.read_number())
    ]
    vector_insert = [
        destack._generated.program.vm.side.decode_vector_insert(reader)
        for _ in range(reader.read_number())
    ]
    vector_shuffle = [
        destack._generated.program.vm.side.decode_vector_shuffle(reader)
        for _ in range(reader.read_number())
    ]
    vector_select = [
        destack._generated.program.vm.side.decode_vector_select(reader)
        for _ in range(reader.read_number())
    ]
    vector_reduce = [
        destack._generated.program.vm.side.decode_vector_reduce(reader)
        for _ in range(reader.read_number())
    ]
    vector_convert = [
        destack._generated.program.vm.side.decode_vector_convert(reader)
        for _ in range(reader.read_number())
    ]
    function_bind = [
        destack._generated.program.vm.side.decode_function_bind(reader)
        for _ in range(reader.read_number())
    ]
    call = [
        destack._generated.program.vm.side.decode_call(reader)
        for _ in range(reader.read_number())
    ]
    call_branch = [
        destack._generated.program.vm.side.decode_call_branch(reader)
        for _ in range(reader.read_number())
    ]
    call_virtual = [
        destack._generated.program.vm.side.decode_call_virtual(reader)
        for _ in range(reader.read_number())
    ]
    call_virtual_branch = [
        destack._generated.program.vm.side.decode_call_virtual_branch(reader)
        for _ in range(reader.read_number())
    ]
    call_dynamic = [
        destack._generated.program.vm.side.decode_call_dynamic(reader)
        for _ in range(reader.read_number())
    ]
    call_dynamic_branch = [
        destack._generated.program.vm.side.decode_call_dynamic_branch(reader)
        for _ in range(reader.read_number())
    ]
    indirect_call = [
        destack._generated.program.vm.side.decode_indirect_call(reader)
        for _ in range(reader.read_number())
    ]
    indirect_call_branch = [
        destack._generated.program.vm.side.decode_indirect_call_branch(reader)
        for _ in range(reader.read_number())
    ]
    tensor_load = [
        destack._generated.program.vm.side.decode_tensor_load(reader)
        for _ in range(reader.read_number())
    ]
    tensor_extract = [
        destack._generated.program.vm.side.decode_tensor_extract(reader)
        for _ in range(reader.read_number())
    ]
    tensor_binary = [
        destack._generated.program.vm.side.decode_tensor_binary(reader)
        for _ in range(reader.read_number())
    ]
    tensor_contiguous_binary = [
        destack._generated.program.vm.side.decode_tensor_contiguous_binary(reader)
        for _ in range(reader.read_number())
    ]
    tensor_unary = [
        destack._generated.program.vm.side.decode_tensor_unary(reader)
        for _ in range(reader.read_number())
    ]
    tensor_contiguous_unary = [
        destack._generated.program.vm.side.decode_tensor_contiguous_unary(reader)
        for _ in range(reader.read_number())
    ]
    tensor_store = [
        destack._generated.program.vm.side.decode_tensor_store(reader)
        for _ in range(reader.read_number())
    ]
    tensor_fill = [
        destack._generated.program.vm.side.decode_tensor_fill(reader)
        for _ in range(reader.read_number())
    ]
    tensor_copy = [
        destack._generated.program.vm.side.decode_tensor_copy(reader)
        for _ in range(reader.read_number())
    ]
    tensor_view_cast = [
        destack._generated.program.vm.side.decode_tensor_view_cast(reader)
        for _ in range(reader.read_number())
    ]
    tensor_reshape = [
        destack._generated.program.vm.side.decode_tensor_reshape(reader)
        for _ in range(reader.read_number())
    ]
    tensor_broadcast = [
        destack._generated.program.vm.side.decode_tensor_broadcast(reader)
        for _ in range(reader.read_number())
    ]
    tensor_transpose = [
        destack._generated.program.vm.side.decode_tensor_transpose(reader)
        for _ in range(reader.read_number())
    ]
    tensor_slice = [
        destack._generated.program.vm.side.decode_tensor_slice(reader)
        for _ in range(reader.read_number())
    ]
    tensor_pad = [
        destack._generated.program.vm.side.decode_tensor_pad(reader)
        for _ in range(reader.read_number())
    ]
    tensor_concat = [
        destack._generated.program.vm.side.decode_tensor_concat(reader)
        for _ in range(reader.read_number())
    ]
    tensor_reduce = [
        destack._generated.program.vm.side.decode_tensor_reduce(reader)
        for _ in range(reader.read_number())
    ]
    tensor_index_reduce = [
        destack._generated.program.vm.side.decode_tensor_index_reduce(reader)
        for _ in range(reader.read_number())
    ]
    tensor_dot = [
        destack._generated.program.vm.side.decode_tensor_dot(reader)
        for _ in range(reader.read_number())
    ]
    tensor_convolution = [
        destack._generated.program.vm.side.decode_tensor_convolution(reader)
        for _ in range(reader.read_number())
    ]
    tensor_gather = [
        destack._generated.program.vm.side.decode_tensor_gather(reader)
        for _ in range(reader.read_number())
    ]
    tensor_scatter = [
        destack._generated.program.vm.side.decode_tensor_scatter(reader)
        for _ in range(reader.read_number())
    ]
    tensor_select = [
        destack._generated.program.vm.side.decode_tensor_select(reader)
        for _ in range(reader.read_number())
    ]
    tensor_convert = [
        destack._generated.program.vm.side.decode_tensor_convert(reader)
        for _ in range(reader.read_number())
    ]
    tensor_view = [
        destack._generated.program.vm.side.decode_tensor_view(reader)
        for _ in range(reader.read_number())
    ]
    intrinsic = [
        destack._generated.program.vm.side.decode_intrinsic(reader)
        for _ in range(reader.read_number())
    ]
    tail_call = [
        destack._generated.program.vm.side.decode_tail_call(reader)
        for _ in range(reader.read_number())
    ]
    tail_call_virtual = [
        destack._generated.program.vm.side.decode_tail_call_virtual(reader)
        for _ in range(reader.read_number())
    ]
    tail_call_dynamic = [
        destack._generated.program.vm.side.decode_tail_call_dynamic(reader)
        for _ in range(reader.read_number())
    ]
    indirect_tail_call = [
        destack._generated.program.vm.side.decode_indirect_tail_call(reader)
        for _ in range(reader.read_number())
    ]

    return SideRecordTable(
        aggregate_select=aggregate_select,
        allocation_branch=allocation_branch,
        slice_allocation_branch=slice_allocation_branch,
        atomic_compare_exchange=atomic_compare_exchange,
        vector_splat=vector_splat,
        vector_extract=vector_extract,
        vector_binary=vector_binary,
        vector_unary=vector_unary,
        vector_insert=vector_insert,
        vector_shuffle=vector_shuffle,
        vector_select=vector_select,
        vector_reduce=vector_reduce,
        vector_convert=vector_convert,
        function_bind=function_bind,
        call=call,
        call_branch=call_branch,
        call_virtual=call_virtual,
        call_virtual_branch=call_virtual_branch,
        call_dynamic=call_dynamic,
        call_dynamic_branch=call_dynamic_branch,
        indirect_call=indirect_call,
        indirect_call_branch=indirect_call_branch,
        tensor_load=tensor_load,
        tensor_extract=tensor_extract,
        tensor_binary=tensor_binary,
        tensor_contiguous_binary=tensor_contiguous_binary,
        tensor_unary=tensor_unary,
        tensor_contiguous_unary=tensor_contiguous_unary,
        tensor_store=tensor_store,
        tensor_fill=tensor_fill,
        tensor_copy=tensor_copy,
        tensor_view_cast=tensor_view_cast,
        tensor_reshape=tensor_reshape,
        tensor_broadcast=tensor_broadcast,
        tensor_transpose=tensor_transpose,
        tensor_slice=tensor_slice,
        tensor_pad=tensor_pad,
        tensor_concat=tensor_concat,
        tensor_reduce=tensor_reduce,
        tensor_index_reduce=tensor_index_reduce,
        tensor_dot=tensor_dot,
        tensor_convolution=tensor_convolution,
        tensor_gather=tensor_gather,
        tensor_scatter=tensor_scatter,
        tensor_select=tensor_select,
        tensor_convert=tensor_convert,
        tensor_view=tensor_view,
        intrinsic=intrinsic,
        tail_call=tail_call,
        tail_call_virtual=tail_call_virtual,
        tail_call_dynamic=tail_call_dynamic,
        indirect_tail_call=indirect_tail_call,
    )


def to_json_side_record_table(value: SideRecordTable) -> Json:
    """Return one JSON value for one SideRecordTable."""
    return {
        "aggregateSelect": [
            destack._generated.program.vm.side.to_json_aggregate_select(item_0)
            for item_0 in value.aggregate_select
        ],
        "allocationBranch": [
            destack._generated.program.vm.allocation.to_json_allocation_branch(item_0)
            for item_0 in value.allocation_branch
        ],
        "sliceAllocationBranch": [
            destack._generated.program.vm.allocation.to_json_slice_allocation_branch(
                item_0
            )
            for item_0 in value.slice_allocation_branch
        ],
        "atomicCompareExchange": [
            destack._generated.program.vm.side.to_json_atomic_compare_exchange(item_0)
            for item_0 in value.atomic_compare_exchange
        ],
        "vectorSplat": [
            destack._generated.program.vm.side.to_json_vector_splat(item_0)
            for item_0 in value.vector_splat
        ],
        "vectorExtract": [
            destack._generated.program.vm.side.to_json_vector_extract(item_0)
            for item_0 in value.vector_extract
        ],
        "vectorBinary": [
            destack._generated.program.vm.side.to_json_vector_binary(item_0)
            for item_0 in value.vector_binary
        ],
        "vectorUnary": [
            destack._generated.program.vm.side.to_json_vector_unary(item_0)
            for item_0 in value.vector_unary
        ],
        "vectorInsert": [
            destack._generated.program.vm.side.to_json_vector_insert(item_0)
            for item_0 in value.vector_insert
        ],
        "vectorShuffle": [
            destack._generated.program.vm.side.to_json_vector_shuffle(item_0)
            for item_0 in value.vector_shuffle
        ],
        "vectorSelect": [
            destack._generated.program.vm.side.to_json_vector_select(item_0)
            for item_0 in value.vector_select
        ],
        "vectorReduce": [
            destack._generated.program.vm.side.to_json_vector_reduce(item_0)
            for item_0 in value.vector_reduce
        ],
        "vectorConvert": [
            destack._generated.program.vm.side.to_json_vector_convert(item_0)
            for item_0 in value.vector_convert
        ],
        "functionBind": [
            destack._generated.program.vm.side.to_json_function_bind(item_0)
            for item_0 in value.function_bind
        ],
        "call": [
            destack._generated.program.vm.side.to_json_call(item_0)
            for item_0 in value.call
        ],
        "callBranch": [
            destack._generated.program.vm.side.to_json_call_branch(item_0)
            for item_0 in value.call_branch
        ],
        "callVirtual": [
            destack._generated.program.vm.side.to_json_call_virtual(item_0)
            for item_0 in value.call_virtual
        ],
        "callVirtualBranch": [
            destack._generated.program.vm.side.to_json_call_virtual_branch(item_0)
            for item_0 in value.call_virtual_branch
        ],
        "callDynamic": [
            destack._generated.program.vm.side.to_json_call_dynamic(item_0)
            for item_0 in value.call_dynamic
        ],
        "callDynamicBranch": [
            destack._generated.program.vm.side.to_json_call_dynamic_branch(item_0)
            for item_0 in value.call_dynamic_branch
        ],
        "indirectCall": [
            destack._generated.program.vm.side.to_json_indirect_call(item_0)
            for item_0 in value.indirect_call
        ],
        "indirectCallBranch": [
            destack._generated.program.vm.side.to_json_indirect_call_branch(item_0)
            for item_0 in value.indirect_call_branch
        ],
        "tensorLoad": [
            destack._generated.program.vm.side.to_json_tensor_load(item_0)
            for item_0 in value.tensor_load
        ],
        "tensorExtract": [
            destack._generated.program.vm.side.to_json_tensor_extract(item_0)
            for item_0 in value.tensor_extract
        ],
        "tensorBinary": [
            destack._generated.program.vm.side.to_json_tensor_binary(item_0)
            for item_0 in value.tensor_binary
        ],
        "tensorContiguousBinary": [
            destack._generated.program.vm.side.to_json_tensor_contiguous_binary(item_0)
            for item_0 in value.tensor_contiguous_binary
        ],
        "tensorUnary": [
            destack._generated.program.vm.side.to_json_tensor_unary(item_0)
            for item_0 in value.tensor_unary
        ],
        "tensorContiguousUnary": [
            destack._generated.program.vm.side.to_json_tensor_contiguous_unary(item_0)
            for item_0 in value.tensor_contiguous_unary
        ],
        "tensorStore": [
            destack._generated.program.vm.side.to_json_tensor_store(item_0)
            for item_0 in value.tensor_store
        ],
        "tensorFill": [
            destack._generated.program.vm.side.to_json_tensor_fill(item_0)
            for item_0 in value.tensor_fill
        ],
        "tensorCopy": [
            destack._generated.program.vm.side.to_json_tensor_copy(item_0)
            for item_0 in value.tensor_copy
        ],
        "tensorViewCast": [
            destack._generated.program.vm.side.to_json_tensor_view_cast(item_0)
            for item_0 in value.tensor_view_cast
        ],
        "tensorReshape": [
            destack._generated.program.vm.side.to_json_tensor_reshape(item_0)
            for item_0 in value.tensor_reshape
        ],
        "tensorBroadcast": [
            destack._generated.program.vm.side.to_json_tensor_broadcast(item_0)
            for item_0 in value.tensor_broadcast
        ],
        "tensorTranspose": [
            destack._generated.program.vm.side.to_json_tensor_transpose(item_0)
            for item_0 in value.tensor_transpose
        ],
        "tensorSlice": [
            destack._generated.program.vm.side.to_json_tensor_slice(item_0)
            for item_0 in value.tensor_slice
        ],
        "tensorPad": [
            destack._generated.program.vm.side.to_json_tensor_pad(item_0)
            for item_0 in value.tensor_pad
        ],
        "tensorConcat": [
            destack._generated.program.vm.side.to_json_tensor_concat(item_0)
            for item_0 in value.tensor_concat
        ],
        "tensorReduce": [
            destack._generated.program.vm.side.to_json_tensor_reduce(item_0)
            for item_0 in value.tensor_reduce
        ],
        "tensorIndexReduce": [
            destack._generated.program.vm.side.to_json_tensor_index_reduce(item_0)
            for item_0 in value.tensor_index_reduce
        ],
        "tensorDot": [
            destack._generated.program.vm.side.to_json_tensor_dot(item_0)
            for item_0 in value.tensor_dot
        ],
        "tensorConvolution": [
            destack._generated.program.vm.side.to_json_tensor_convolution(item_0)
            for item_0 in value.tensor_convolution
        ],
        "tensorGather": [
            destack._generated.program.vm.side.to_json_tensor_gather(item_0)
            for item_0 in value.tensor_gather
        ],
        "tensorScatter": [
            destack._generated.program.vm.side.to_json_tensor_scatter(item_0)
            for item_0 in value.tensor_scatter
        ],
        "tensorSelect": [
            destack._generated.program.vm.side.to_json_tensor_select(item_0)
            for item_0 in value.tensor_select
        ],
        "tensorConvert": [
            destack._generated.program.vm.side.to_json_tensor_convert(item_0)
            for item_0 in value.tensor_convert
        ],
        "tensorView": [
            destack._generated.program.vm.side.to_json_tensor_view(item_0)
            for item_0 in value.tensor_view
        ],
        "intrinsic": [
            destack._generated.program.vm.side.to_json_intrinsic(item_0)
            for item_0 in value.intrinsic
        ],
        "tailCall": [
            destack._generated.program.vm.side.to_json_tail_call(item_0)
            for item_0 in value.tail_call
        ],
        "tailCallVirtual": [
            destack._generated.program.vm.side.to_json_tail_call_virtual(item_0)
            for item_0 in value.tail_call_virtual
        ],
        "tailCallDynamic": [
            destack._generated.program.vm.side.to_json_tail_call_dynamic(item_0)
            for item_0 in value.tail_call_dynamic
        ],
        "indirectTailCall": [
            destack._generated.program.vm.side.to_json_indirect_tail_call(item_0)
            for item_0 in value.indirect_tail_call
        ],
    }


def from_json_side_record_table(value: Json) -> SideRecordTable:
    """Return one SideRecordTable from one JSON value."""
    object_ = json_object(value)

    return SideRecordTable(
        aggregate_select=[
            destack._generated.program.vm.side.from_json_aggregate_select(item_0)
            for item_0 in json_array(json_field(object_, "aggregateSelect"))
        ],
        allocation_branch=[
            destack._generated.program.vm.allocation.from_json_allocation_branch(item_0)
            for item_0 in json_array(json_field(object_, "allocationBranch"))
        ],
        slice_allocation_branch=[
            destack._generated.program.vm.allocation.from_json_slice_allocation_branch(
                item_0
            )
            for item_0 in json_array(json_field(object_, "sliceAllocationBranch"))
        ],
        atomic_compare_exchange=[
            destack._generated.program.vm.side.from_json_atomic_compare_exchange(item_0)
            for item_0 in json_array(json_field(object_, "atomicCompareExchange"))
        ],
        vector_splat=[
            destack._generated.program.vm.side.from_json_vector_splat(item_0)
            for item_0 in json_array(json_field(object_, "vectorSplat"))
        ],
        vector_extract=[
            destack._generated.program.vm.side.from_json_vector_extract(item_0)
            for item_0 in json_array(json_field(object_, "vectorExtract"))
        ],
        vector_binary=[
            destack._generated.program.vm.side.from_json_vector_binary(item_0)
            for item_0 in json_array(json_field(object_, "vectorBinary"))
        ],
        vector_unary=[
            destack._generated.program.vm.side.from_json_vector_unary(item_0)
            for item_0 in json_array(json_field(object_, "vectorUnary"))
        ],
        vector_insert=[
            destack._generated.program.vm.side.from_json_vector_insert(item_0)
            for item_0 in json_array(json_field(object_, "vectorInsert"))
        ],
        vector_shuffle=[
            destack._generated.program.vm.side.from_json_vector_shuffle(item_0)
            for item_0 in json_array(json_field(object_, "vectorShuffle"))
        ],
        vector_select=[
            destack._generated.program.vm.side.from_json_vector_select(item_0)
            for item_0 in json_array(json_field(object_, "vectorSelect"))
        ],
        vector_reduce=[
            destack._generated.program.vm.side.from_json_vector_reduce(item_0)
            for item_0 in json_array(json_field(object_, "vectorReduce"))
        ],
        vector_convert=[
            destack._generated.program.vm.side.from_json_vector_convert(item_0)
            for item_0 in json_array(json_field(object_, "vectorConvert"))
        ],
        function_bind=[
            destack._generated.program.vm.side.from_json_function_bind(item_0)
            for item_0 in json_array(json_field(object_, "functionBind"))
        ],
        call=[
            destack._generated.program.vm.side.from_json_call(item_0)
            for item_0 in json_array(json_field(object_, "call"))
        ],
        call_branch=[
            destack._generated.program.vm.side.from_json_call_branch(item_0)
            for item_0 in json_array(json_field(object_, "callBranch"))
        ],
        call_virtual=[
            destack._generated.program.vm.side.from_json_call_virtual(item_0)
            for item_0 in json_array(json_field(object_, "callVirtual"))
        ],
        call_virtual_branch=[
            destack._generated.program.vm.side.from_json_call_virtual_branch(item_0)
            for item_0 in json_array(json_field(object_, "callVirtualBranch"))
        ],
        call_dynamic=[
            destack._generated.program.vm.side.from_json_call_dynamic(item_0)
            for item_0 in json_array(json_field(object_, "callDynamic"))
        ],
        call_dynamic_branch=[
            destack._generated.program.vm.side.from_json_call_dynamic_branch(item_0)
            for item_0 in json_array(json_field(object_, "callDynamicBranch"))
        ],
        indirect_call=[
            destack._generated.program.vm.side.from_json_indirect_call(item_0)
            for item_0 in json_array(json_field(object_, "indirectCall"))
        ],
        indirect_call_branch=[
            destack._generated.program.vm.side.from_json_indirect_call_branch(item_0)
            for item_0 in json_array(json_field(object_, "indirectCallBranch"))
        ],
        tensor_load=[
            destack._generated.program.vm.side.from_json_tensor_load(item_0)
            for item_0 in json_array(json_field(object_, "tensorLoad"))
        ],
        tensor_extract=[
            destack._generated.program.vm.side.from_json_tensor_extract(item_0)
            for item_0 in json_array(json_field(object_, "tensorExtract"))
        ],
        tensor_binary=[
            destack._generated.program.vm.side.from_json_tensor_binary(item_0)
            for item_0 in json_array(json_field(object_, "tensorBinary"))
        ],
        tensor_contiguous_binary=[
            destack._generated.program.vm.side.from_json_tensor_contiguous_binary(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tensorContiguousBinary"))
        ],
        tensor_unary=[
            destack._generated.program.vm.side.from_json_tensor_unary(item_0)
            for item_0 in json_array(json_field(object_, "tensorUnary"))
        ],
        tensor_contiguous_unary=[
            destack._generated.program.vm.side.from_json_tensor_contiguous_unary(item_0)
            for item_0 in json_array(json_field(object_, "tensorContiguousUnary"))
        ],
        tensor_store=[
            destack._generated.program.vm.side.from_json_tensor_store(item_0)
            for item_0 in json_array(json_field(object_, "tensorStore"))
        ],
        tensor_fill=[
            destack._generated.program.vm.side.from_json_tensor_fill(item_0)
            for item_0 in json_array(json_field(object_, "tensorFill"))
        ],
        tensor_copy=[
            destack._generated.program.vm.side.from_json_tensor_copy(item_0)
            for item_0 in json_array(json_field(object_, "tensorCopy"))
        ],
        tensor_view_cast=[
            destack._generated.program.vm.side.from_json_tensor_view_cast(item_0)
            for item_0 in json_array(json_field(object_, "tensorViewCast"))
        ],
        tensor_reshape=[
            destack._generated.program.vm.side.from_json_tensor_reshape(item_0)
            for item_0 in json_array(json_field(object_, "tensorReshape"))
        ],
        tensor_broadcast=[
            destack._generated.program.vm.side.from_json_tensor_broadcast(item_0)
            for item_0 in json_array(json_field(object_, "tensorBroadcast"))
        ],
        tensor_transpose=[
            destack._generated.program.vm.side.from_json_tensor_transpose(item_0)
            for item_0 in json_array(json_field(object_, "tensorTranspose"))
        ],
        tensor_slice=[
            destack._generated.program.vm.side.from_json_tensor_slice(item_0)
            for item_0 in json_array(json_field(object_, "tensorSlice"))
        ],
        tensor_pad=[
            destack._generated.program.vm.side.from_json_tensor_pad(item_0)
            for item_0 in json_array(json_field(object_, "tensorPad"))
        ],
        tensor_concat=[
            destack._generated.program.vm.side.from_json_tensor_concat(item_0)
            for item_0 in json_array(json_field(object_, "tensorConcat"))
        ],
        tensor_reduce=[
            destack._generated.program.vm.side.from_json_tensor_reduce(item_0)
            for item_0 in json_array(json_field(object_, "tensorReduce"))
        ],
        tensor_index_reduce=[
            destack._generated.program.vm.side.from_json_tensor_index_reduce(item_0)
            for item_0 in json_array(json_field(object_, "tensorIndexReduce"))
        ],
        tensor_dot=[
            destack._generated.program.vm.side.from_json_tensor_dot(item_0)
            for item_0 in json_array(json_field(object_, "tensorDot"))
        ],
        tensor_convolution=[
            destack._generated.program.vm.side.from_json_tensor_convolution(item_0)
            for item_0 in json_array(json_field(object_, "tensorConvolution"))
        ],
        tensor_gather=[
            destack._generated.program.vm.side.from_json_tensor_gather(item_0)
            for item_0 in json_array(json_field(object_, "tensorGather"))
        ],
        tensor_scatter=[
            destack._generated.program.vm.side.from_json_tensor_scatter(item_0)
            for item_0 in json_array(json_field(object_, "tensorScatter"))
        ],
        tensor_select=[
            destack._generated.program.vm.side.from_json_tensor_select(item_0)
            for item_0 in json_array(json_field(object_, "tensorSelect"))
        ],
        tensor_convert=[
            destack._generated.program.vm.side.from_json_tensor_convert(item_0)
            for item_0 in json_array(json_field(object_, "tensorConvert"))
        ],
        tensor_view=[
            destack._generated.program.vm.side.from_json_tensor_view(item_0)
            for item_0 in json_array(json_field(object_, "tensorView"))
        ],
        intrinsic=[
            destack._generated.program.vm.side.from_json_intrinsic(item_0)
            for item_0 in json_array(json_field(object_, "intrinsic"))
        ],
        tail_call=[
            destack._generated.program.vm.side.from_json_tail_call(item_0)
            for item_0 in json_array(json_field(object_, "tailCall"))
        ],
        tail_call_virtual=[
            destack._generated.program.vm.side.from_json_tail_call_virtual(item_0)
            for item_0 in json_array(json_field(object_, "tailCallVirtual"))
        ],
        tail_call_dynamic=[
            destack._generated.program.vm.side.from_json_tail_call_dynamic(item_0)
            for item_0 in json_array(json_field(object_, "tailCallDynamic"))
        ],
        indirect_tail_call=[
            destack._generated.program.vm.side.from_json_indirect_tail_call(item_0)
            for item_0 in json_array(json_field(object_, "indirectTailCall"))
        ],
    )


"""Identifier for one pooled allocation site."""
AllocationSiteId: typing.TypeAlias = int


def encode_allocation_site_id(writer: BinaryWriter, value: AllocationSiteId) -> None:
    """Encode one AllocationSiteId."""
    writer.write_unsigned(value)


def decode_allocation_site_id(reader: BinaryReader) -> AllocationSiteId:
    """Decode one AllocationSiteId."""
    return reader.read_number()


def to_json_allocation_site_id(value: AllocationSiteId) -> Json:
    """Return one JSON value for one AllocationSiteId."""
    return value


def from_json_allocation_site_id(value: Json) -> AllocationSiteId:
    """Return one AllocationSiteId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class Edge:
    """One lowered control-flow edge."""

    # the target block
    target: int
    # the block-parameter moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_edge(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Edge:
        """Decode one Edge."""
        return decode_edge(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_edge(self)

    @classmethod
    def from_json(cls, value: Json) -> Edge:
        """Return one Edge from one JSON value."""
        return from_json_edge(value)


def encode_edge(writer: BinaryWriter, value: Edge) -> None:
    """Encode one Edge."""
    writer.write_unsigned(value.target)
    destack._generated.program.vm.range.encode_move_range(writer, value.moves)


def decode_edge(reader: BinaryReader) -> Edge:
    """Decode one Edge."""
    target = reader.read_number()
    moves = destack._generated.program.vm.range.decode_move_range(reader)

    return Edge(
        target=target,
        moves=moves,
    )


def to_json_edge(value: Edge) -> Json:
    """Return one JSON value for one Edge."""
    return {
        "target": value.target,
        "moves": destack._generated.program.vm.range.to_json_move_range(value.moves),
    }


def from_json_edge(value: Json) -> Edge:
    """Return one Edge from one JSON value."""
    object_ = json_object(value)

    return Edge(
        target=json_int(json_field(object_, "target")),
        moves=destack._generated.program.vm.range.from_json_move_range(
            json_field(object_, "moves")
        ),
    )


"""Identifier for one pooled slice projection."""
SliceProjectionId: typing.TypeAlias = int


def encode_slice_projection_id(writer: BinaryWriter, value: SliceProjectionId) -> None:
    """Encode one SliceProjectionId."""
    writer.write_unsigned(value)


def decode_slice_projection_id(reader: BinaryReader) -> SliceProjectionId:
    """Decode one SliceProjectionId."""
    return reader.read_number()


def to_json_slice_projection_id(value: SliceProjectionId) -> Json:
    """Return one JSON value for one SliceProjectionId."""
    return value


def from_json_slice_projection_id(value: Json) -> SliceProjectionId:
    """Return one SliceProjectionId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled u32 slice."""
U32RangeId: typing.TypeAlias = int


def encode_u32_range_id(writer: BinaryWriter, value: U32RangeId) -> None:
    """Encode one U32RangeId."""
    writer.write_unsigned(value)


def decode_u32_range_id(reader: BinaryReader) -> U32RangeId:
    """Decode one U32RangeId."""
    return reader.read_number()


def to_json_u32_range_id(value: U32RangeId) -> Json:
    """Return one JSON value for one U32RangeId."""
    return value


def from_json_u32_range_id(value: Json) -> U32RangeId:
    """Return one U32RangeId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled address projection."""
ProjectionId: typing.TypeAlias = int


def encode_projection_id(writer: BinaryWriter, value: ProjectionId) -> None:
    """Encode one ProjectionId."""
    writer.write_unsigned(value)


def decode_projection_id(reader: BinaryReader) -> ProjectionId:
    """Decode one ProjectionId."""
    return reader.read_number()


def to_json_projection_id(value: ProjectionId) -> Json:
    """Return one JSON value for one ProjectionId."""
    return value


def from_json_projection_id(value: Json) -> ProjectionId:
    """Return one ProjectionId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor layout."""
TensorLayoutId: typing.TypeAlias = int


def encode_tensor_layout_id(writer: BinaryWriter, value: TensorLayoutId) -> None:
    """Encode one TensorLayoutId."""
    writer.write_unsigned(value)


def decode_tensor_layout_id(reader: BinaryReader) -> TensorLayoutId:
    """Decode one TensorLayoutId."""
    return reader.read_number()


def to_json_tensor_layout_id(value: TensorLayoutId) -> Json:
    """Return one JSON value for one TensorLayoutId."""
    return value


def from_json_tensor_layout_id(value: Json) -> TensorLayoutId:
    """Return one TensorLayoutId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor dot descriptor."""
TensorDotId: typing.TypeAlias = int


def encode_tensor_dot_id(writer: BinaryWriter, value: TensorDotId) -> None:
    """Encode one TensorDotId."""
    writer.write_unsigned(value)


def decode_tensor_dot_id(reader: BinaryReader) -> TensorDotId:
    """Decode one TensorDotId."""
    return reader.read_number()


def to_json_tensor_dot_id(value: TensorDotId) -> Json:
    """Return one JSON value for one TensorDotId."""
    return value


def from_json_tensor_dot_id(value: Json) -> TensorDotId:
    """Return one TensorDotId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor convolution dimension descriptor."""
TensorConvolutionId: typing.TypeAlias = int


def encode_tensor_convolution_id(
    writer: BinaryWriter, value: TensorConvolutionId
) -> None:
    """Encode one TensorConvolutionId."""
    writer.write_unsigned(value)


def decode_tensor_convolution_id(reader: BinaryReader) -> TensorConvolutionId:
    """Decode one TensorConvolutionId."""
    return reader.read_number()


def to_json_tensor_convolution_id(value: TensorConvolutionId) -> Json:
    """Return one JSON value for one TensorConvolutionId."""
    return value


def from_json_tensor_convolution_id(value: Json) -> TensorConvolutionId:
    """Return one TensorConvolutionId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor convolution window descriptor."""
TensorWindowId: typing.TypeAlias = int


def encode_tensor_window_id(writer: BinaryWriter, value: TensorWindowId) -> None:
    """Encode one TensorWindowId."""
    writer.write_unsigned(value)


def decode_tensor_window_id(reader: BinaryReader) -> TensorWindowId:
    """Decode one TensorWindowId."""
    return reader.read_number()


def to_json_tensor_window_id(value: TensorWindowId) -> Json:
    """Return one JSON value for one TensorWindowId."""
    return value


def from_json_tensor_window_id(value: Json) -> TensorWindowId:
    """Return one TensorWindowId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor gather descriptor."""
TensorGatherId: typing.TypeAlias = int


def encode_tensor_gather_id(writer: BinaryWriter, value: TensorGatherId) -> None:
    """Encode one TensorGatherId."""
    writer.write_unsigned(value)


def decode_tensor_gather_id(reader: BinaryReader) -> TensorGatherId:
    """Decode one TensorGatherId."""
    return reader.read_number()


def to_json_tensor_gather_id(value: TensorGatherId) -> Json:
    """Return one JSON value for one TensorGatherId."""
    return value


def from_json_tensor_gather_id(value: Json) -> TensorGatherId:
    """Return one TensorGatherId from one JSON value."""
    return json_int(value)


"""Identifier for one pooled tensor scatter descriptor."""
TensorScatterId: typing.TypeAlias = int


def encode_tensor_scatter_id(writer: BinaryWriter, value: TensorScatterId) -> None:
    """Encode one TensorScatterId."""
    writer.write_unsigned(value)


def decode_tensor_scatter_id(reader: BinaryReader) -> TensorScatterId:
    """Decode one TensorScatterId."""
    return reader.read_number()


def to_json_tensor_scatter_id(value: TensorScatterId) -> Json:
    """Return one JSON value for one TensorScatterId."""
    return value


def from_json_tensor_scatter_id(value: Json) -> TensorScatterId:
    """Return one TensorScatterId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class CheckBoundsIntInt:
    """Bounds check over signed index and signed length cells."""

    bounds_int_int: BoundsCheck
    kind: typing.Literal["boundsIntInt"] = "boundsIntInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckBoundsIntUint:
    """Bounds check over signed index and unsigned length cells."""

    bounds_int_uint: BoundsCheck
    kind: typing.Literal["boundsIntUint"] = "boundsIntUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckBoundsUintInt:
    """Bounds check over unsigned index and signed length cells."""

    bounds_uint_int: BoundsCheck
    kind: typing.Literal["boundsUintInt"] = "boundsUintInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckBoundsUintUint:
    """Bounds check over unsigned index and unsigned length cells."""

    bounds_uint_uint: BoundsCheck
    kind: typing.Literal["boundsUintUint"] = "boundsUintUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckNull:
    """Non-null check over one cell."""

    # the value cell offset
    value: int
    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckDivZeroInt:
    """Division-by-zero check over one signed cell."""

    divisor: int
    kind: typing.Literal["divZeroInt"] = "divZeroInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckDivZeroUint:
    """Division-by-zero check over one unsigned cell."""

    divisor: int
    kind: typing.Literal["divZeroUint"] = "divZeroUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckShiftRangeInt:
    """Shift range check over one signed shift amount cell."""

    shift_range_int: ShiftRangeCheck
    kind: typing.Literal["shiftRangeInt"] = "shiftRangeInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckShiftRangeUint:
    """Shift range check over one unsigned shift amount cell."""

    shift_range_uint: ShiftRangeCheck
    kind: typing.Literal["shiftRangeUint"] = "shiftRangeUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckNarrowInt:
    """Signed integer narrowing check over one cell."""

    narrow_int: NarrowCheck
    kind: typing.Literal["narrowInt"] = "narrowInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckNarrowUint:
    """Unsigned integer narrowing check over one cell."""

    narrow_uint: NarrowCheck
    kind: typing.Literal["narrowUint"] = "narrowUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowAddInt:
    """Signed add overflow check over two cells."""

    overflow_add_int: OverflowCheck
    kind: typing.Literal["overflowAddInt"] = "overflowAddInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowAddUint:
    """Unsigned add overflow check over two cells."""

    overflow_add_uint: OverflowCheck
    kind: typing.Literal["overflowAddUint"] = "overflowAddUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowSubInt:
    """Signed subtract overflow check over two cells."""

    overflow_sub_int: OverflowCheck
    kind: typing.Literal["overflowSubInt"] = "overflowSubInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowSubUint:
    """Unsigned subtract overflow check over two cells."""

    overflow_sub_uint: OverflowCheck
    kind: typing.Literal["overflowSubUint"] = "overflowSubUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowMulInt:
    """Signed multiply overflow check over two cells."""

    overflow_mul_int: OverflowCheck
    kind: typing.Literal["overflowMulInt"] = "overflowMulInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowMulUint:
    """Unsigned multiply overflow check over two cells."""

    overflow_mul_uint: OverflowCheck
    kind: typing.Literal["overflowMulUint"] = "overflowMulUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowDivInt:
    """Signed divide or remainder overflow check over two cells."""

    overflow_div_int: OverflowCheck
    kind: typing.Literal["overflowDivInt"] = "overflowDivInt"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckOverflowDivUint:
    """Unsigned divide or remainder overflow check over two cells."""

    overflow_div_uint: OverflowCheck
    kind: typing.Literal["overflowDivUint"] = "overflowDivUint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckType:
    """Runtime type descriptor check."""

    # the descriptor cell offset
    value: int
    # the expected type id
    expected: int
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


@dataclass(frozen=True, slots=True)
class CheckVariant:
    """Variant tag check."""

    variant: VariantCheck
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check(self)


"""One lowered runtime check."""
Check: typing.TypeAlias = (
    CheckBoundsIntInt
    | CheckBoundsIntUint
    | CheckBoundsUintInt
    | CheckBoundsUintUint
    | CheckNull
    | CheckDivZeroInt
    | CheckDivZeroUint
    | CheckShiftRangeInt
    | CheckShiftRangeUint
    | CheckNarrowInt
    | CheckNarrowUint
    | CheckOverflowAddInt
    | CheckOverflowAddUint
    | CheckOverflowSubInt
    | CheckOverflowSubUint
    | CheckOverflowMulInt
    | CheckOverflowMulUint
    | CheckOverflowDivInt
    | CheckOverflowDivUint
    | CheckType
    | CheckVariant
)


def encode_check(writer: BinaryWriter, value: Check) -> None:
    """Encode one Check."""
    if value.kind == "boundsIntInt":
        writer.write_unsigned(0)
        encode_bounds_check(writer, value.bounds_int_int)
    elif value.kind == "boundsIntUint":
        writer.write_unsigned(1)
        encode_bounds_check(writer, value.bounds_int_uint)
    elif value.kind == "boundsUintInt":
        writer.write_unsigned(2)
        encode_bounds_check(writer, value.bounds_uint_int)
    elif value.kind == "boundsUintUint":
        writer.write_unsigned(3)
        encode_bounds_check(writer, value.bounds_uint_uint)
    elif value.kind == "null":
        writer.write_unsigned(4)
        writer.write_unsigned(value.value)
    elif value.kind == "divZeroInt":
        writer.write_unsigned(5)
        writer.write_unsigned(value.divisor)
    elif value.kind == "divZeroUint":
        writer.write_unsigned(6)
        writer.write_unsigned(value.divisor)
    elif value.kind == "shiftRangeInt":
        writer.write_unsigned(7)
        encode_shift_range_check(writer, value.shift_range_int)
    elif value.kind == "shiftRangeUint":
        writer.write_unsigned(8)
        encode_shift_range_check(writer, value.shift_range_uint)
    elif value.kind == "narrowInt":
        writer.write_unsigned(9)
        encode_narrow_check(writer, value.narrow_int)
    elif value.kind == "narrowUint":
        writer.write_unsigned(10)
        encode_narrow_check(writer, value.narrow_uint)
    elif value.kind == "overflowAddInt":
        writer.write_unsigned(11)
        encode_overflow_check(writer, value.overflow_add_int)
    elif value.kind == "overflowAddUint":
        writer.write_unsigned(12)
        encode_overflow_check(writer, value.overflow_add_uint)
    elif value.kind == "overflowSubInt":
        writer.write_unsigned(13)
        encode_overflow_check(writer, value.overflow_sub_int)
    elif value.kind == "overflowSubUint":
        writer.write_unsigned(14)
        encode_overflow_check(writer, value.overflow_sub_uint)
    elif value.kind == "overflowMulInt":
        writer.write_unsigned(15)
        encode_overflow_check(writer, value.overflow_mul_int)
    elif value.kind == "overflowMulUint":
        writer.write_unsigned(16)
        encode_overflow_check(writer, value.overflow_mul_uint)
    elif value.kind == "overflowDivInt":
        writer.write_unsigned(17)
        encode_overflow_check(writer, value.overflow_div_int)
    elif value.kind == "overflowDivUint":
        writer.write_unsigned(18)
        encode_overflow_check(writer, value.overflow_div_uint)
    elif value.kind == "type":
        writer.write_unsigned(19)
        writer.write_unsigned(value.value)
        writer.write_unsigned(value.expected)
    elif value.kind == "variant":
        writer.write_unsigned(20)
        encode_variant_check(writer, value.variant)
    else:
        raise SerdeError("unknown enum variant")


def decode_check(reader: BinaryReader) -> Check:
    """Decode one Check."""
    variant = reader.read_number()

    if variant == 0:
        bounds_int_int = decode_bounds_check(reader)

        return CheckBoundsIntInt(bounds_int_int=bounds_int_int)
    elif variant == 1:
        bounds_int_uint = decode_bounds_check(reader)

        return CheckBoundsIntUint(bounds_int_uint=bounds_int_uint)
    elif variant == 2:
        bounds_uint_int = decode_bounds_check(reader)

        return CheckBoundsUintInt(bounds_uint_int=bounds_uint_int)
    elif variant == 3:
        bounds_uint_uint = decode_bounds_check(reader)

        return CheckBoundsUintUint(bounds_uint_uint=bounds_uint_uint)
    elif variant == 4:
        value_ = reader.read_number()

        return CheckNull(
            value=value_,
        )
    elif variant == 5:
        divisor = reader.read_number()

        return CheckDivZeroInt(
            divisor=divisor,
        )
    elif variant == 6:
        divisor = reader.read_number()

        return CheckDivZeroUint(
            divisor=divisor,
        )
    elif variant == 7:
        shift_range_int = decode_shift_range_check(reader)

        return CheckShiftRangeInt(shift_range_int=shift_range_int)
    elif variant == 8:
        shift_range_uint = decode_shift_range_check(reader)

        return CheckShiftRangeUint(shift_range_uint=shift_range_uint)
    elif variant == 9:
        narrow_int = decode_narrow_check(reader)

        return CheckNarrowInt(narrow_int=narrow_int)
    elif variant == 10:
        narrow_uint = decode_narrow_check(reader)

        return CheckNarrowUint(narrow_uint=narrow_uint)
    elif variant == 11:
        overflow_add_int = decode_overflow_check(reader)

        return CheckOverflowAddInt(overflow_add_int=overflow_add_int)
    elif variant == 12:
        overflow_add_uint = decode_overflow_check(reader)

        return CheckOverflowAddUint(overflow_add_uint=overflow_add_uint)
    elif variant == 13:
        overflow_sub_int = decode_overflow_check(reader)

        return CheckOverflowSubInt(overflow_sub_int=overflow_sub_int)
    elif variant == 14:
        overflow_sub_uint = decode_overflow_check(reader)

        return CheckOverflowSubUint(overflow_sub_uint=overflow_sub_uint)
    elif variant == 15:
        overflow_mul_int = decode_overflow_check(reader)

        return CheckOverflowMulInt(overflow_mul_int=overflow_mul_int)
    elif variant == 16:
        overflow_mul_uint = decode_overflow_check(reader)

        return CheckOverflowMulUint(overflow_mul_uint=overflow_mul_uint)
    elif variant == 17:
        overflow_div_int = decode_overflow_check(reader)

        return CheckOverflowDivInt(overflow_div_int=overflow_div_int)
    elif variant == 18:
        overflow_div_uint = decode_overflow_check(reader)

        return CheckOverflowDivUint(overflow_div_uint=overflow_div_uint)
    elif variant == 19:
        value_ = reader.read_number()
        expected = reader.read_number()

        return CheckType(
            value=value_,
            expected=expected,
        )
    elif variant == 20:
        variant = decode_variant_check(reader)

        return CheckVariant(variant=variant)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_check(value: Check) -> Json:
    """Return one JSON value for one Check."""
    if value.kind == "boundsIntInt":
        return {
            "kind": "boundsIntInt",
            "bounds_int_int": to_json_bounds_check(value.bounds_int_int),
        }
    elif value.kind == "boundsIntUint":
        return {
            "kind": "boundsIntUint",
            "bounds_int_uint": to_json_bounds_check(value.bounds_int_uint),
        }
    elif value.kind == "boundsUintInt":
        return {
            "kind": "boundsUintInt",
            "bounds_uint_int": to_json_bounds_check(value.bounds_uint_int),
        }
    elif value.kind == "boundsUintUint":
        return {
            "kind": "boundsUintUint",
            "bounds_uint_uint": to_json_bounds_check(value.bounds_uint_uint),
        }
    elif value.kind == "null":
        return {
            "kind": "null",
            "value": value.value,
        }
    elif value.kind == "divZeroInt":
        return {
            "kind": "divZeroInt",
            "divisor": value.divisor,
        }
    elif value.kind == "divZeroUint":
        return {
            "kind": "divZeroUint",
            "divisor": value.divisor,
        }
    elif value.kind == "shiftRangeInt":
        return {
            "kind": "shiftRangeInt",
            "shift_range_int": to_json_shift_range_check(value.shift_range_int),
        }
    elif value.kind == "shiftRangeUint":
        return {
            "kind": "shiftRangeUint",
            "shift_range_uint": to_json_shift_range_check(value.shift_range_uint),
        }
    elif value.kind == "narrowInt":
        return {
            "kind": "narrowInt",
            "narrow_int": to_json_narrow_check(value.narrow_int),
        }
    elif value.kind == "narrowUint":
        return {
            "kind": "narrowUint",
            "narrow_uint": to_json_narrow_check(value.narrow_uint),
        }
    elif value.kind == "overflowAddInt":
        return {
            "kind": "overflowAddInt",
            "overflow_add_int": to_json_overflow_check(value.overflow_add_int),
        }
    elif value.kind == "overflowAddUint":
        return {
            "kind": "overflowAddUint",
            "overflow_add_uint": to_json_overflow_check(value.overflow_add_uint),
        }
    elif value.kind == "overflowSubInt":
        return {
            "kind": "overflowSubInt",
            "overflow_sub_int": to_json_overflow_check(value.overflow_sub_int),
        }
    elif value.kind == "overflowSubUint":
        return {
            "kind": "overflowSubUint",
            "overflow_sub_uint": to_json_overflow_check(value.overflow_sub_uint),
        }
    elif value.kind == "overflowMulInt":
        return {
            "kind": "overflowMulInt",
            "overflow_mul_int": to_json_overflow_check(value.overflow_mul_int),
        }
    elif value.kind == "overflowMulUint":
        return {
            "kind": "overflowMulUint",
            "overflow_mul_uint": to_json_overflow_check(value.overflow_mul_uint),
        }
    elif value.kind == "overflowDivInt":
        return {
            "kind": "overflowDivInt",
            "overflow_div_int": to_json_overflow_check(value.overflow_div_int),
        }
    elif value.kind == "overflowDivUint":
        return {
            "kind": "overflowDivUint",
            "overflow_div_uint": to_json_overflow_check(value.overflow_div_uint),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "value": value.value,
            "expected": value.expected,
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_variant_check(value.variant),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_check(value: Json) -> Check:
    """Return one Check from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "boundsIntInt":
        return CheckBoundsIntInt(
            bounds_int_int=from_json_bounds_check(json_field(object_, "bounds_int_int"))
        )
    elif kind == "boundsIntUint":
        return CheckBoundsIntUint(
            bounds_int_uint=from_json_bounds_check(
                json_field(object_, "bounds_int_uint")
            )
        )
    elif kind == "boundsUintInt":
        return CheckBoundsUintInt(
            bounds_uint_int=from_json_bounds_check(
                json_field(object_, "bounds_uint_int")
            )
        )
    elif kind == "boundsUintUint":
        return CheckBoundsUintUint(
            bounds_uint_uint=from_json_bounds_check(
                json_field(object_, "bounds_uint_uint")
            )
        )
    elif kind == "null":
        return CheckNull(
            value=json_int(json_field(object_, "value")),
        )
    elif kind == "divZeroInt":
        return CheckDivZeroInt(
            divisor=json_int(json_field(object_, "divisor")),
        )
    elif kind == "divZeroUint":
        return CheckDivZeroUint(
            divisor=json_int(json_field(object_, "divisor")),
        )
    elif kind == "shiftRangeInt":
        return CheckShiftRangeInt(
            shift_range_int=from_json_shift_range_check(
                json_field(object_, "shift_range_int")
            )
        )
    elif kind == "shiftRangeUint":
        return CheckShiftRangeUint(
            shift_range_uint=from_json_shift_range_check(
                json_field(object_, "shift_range_uint")
            )
        )
    elif kind == "narrowInt":
        return CheckNarrowInt(
            narrow_int=from_json_narrow_check(json_field(object_, "narrow_int"))
        )
    elif kind == "narrowUint":
        return CheckNarrowUint(
            narrow_uint=from_json_narrow_check(json_field(object_, "narrow_uint"))
        )
    elif kind == "overflowAddInt":
        return CheckOverflowAddInt(
            overflow_add_int=from_json_overflow_check(
                json_field(object_, "overflow_add_int")
            )
        )
    elif kind == "overflowAddUint":
        return CheckOverflowAddUint(
            overflow_add_uint=from_json_overflow_check(
                json_field(object_, "overflow_add_uint")
            )
        )
    elif kind == "overflowSubInt":
        return CheckOverflowSubInt(
            overflow_sub_int=from_json_overflow_check(
                json_field(object_, "overflow_sub_int")
            )
        )
    elif kind == "overflowSubUint":
        return CheckOverflowSubUint(
            overflow_sub_uint=from_json_overflow_check(
                json_field(object_, "overflow_sub_uint")
            )
        )
    elif kind == "overflowMulInt":
        return CheckOverflowMulInt(
            overflow_mul_int=from_json_overflow_check(
                json_field(object_, "overflow_mul_int")
            )
        )
    elif kind == "overflowMulUint":
        return CheckOverflowMulUint(
            overflow_mul_uint=from_json_overflow_check(
                json_field(object_, "overflow_mul_uint")
            )
        )
    elif kind == "overflowDivInt":
        return CheckOverflowDivInt(
            overflow_div_int=from_json_overflow_check(
                json_field(object_, "overflow_div_int")
            )
        )
    elif kind == "overflowDivUint":
        return CheckOverflowDivUint(
            overflow_div_uint=from_json_overflow_check(
                json_field(object_, "overflow_div_uint")
            )
        )
    elif kind == "type":
        return CheckType(
            value=json_int(json_field(object_, "value")),
            expected=json_int(json_field(object_, "expected")),
        )
    elif kind == "variant":
        return CheckVariant(
            variant=from_json_variant_check(json_field(object_, "variant"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class BoundsCheck:
    """Bounds check over index and length cells."""

    # the index cell offset
    index: int
    # the length cell offset
    length: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_bounds_check(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BoundsCheck:
        """Decode one BoundsCheck."""
        return decode_bounds_check(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_bounds_check(self)

    @classmethod
    def from_json(cls, value: Json) -> BoundsCheck:
        """Return one BoundsCheck from one JSON value."""
        return from_json_bounds_check(value)


def encode_bounds_check(writer: BinaryWriter, value: BoundsCheck) -> None:
    """Encode one BoundsCheck."""
    writer.write_unsigned(value.index)
    writer.write_unsigned(value.length)


def decode_bounds_check(reader: BinaryReader) -> BoundsCheck:
    """Decode one BoundsCheck."""
    index = reader.read_number()
    length = reader.read_number()

    return BoundsCheck(
        index=index,
        length=length,
    )


def to_json_bounds_check(value: BoundsCheck) -> Json:
    """Return one JSON value for one BoundsCheck."""
    return {
        "index": value.index,
        "length": value.length,
    }


def from_json_bounds_check(value: Json) -> BoundsCheck:
    """Return one BoundsCheck from one JSON value."""
    object_ = json_object(value)

    return BoundsCheck(
        index=json_int(json_field(object_, "index")),
        length=json_int(json_field(object_, "length")),
    )


@dataclass(frozen=True, slots=True)
class ShiftRangeCheck:
    """Shift amount range check over one cell."""

    # the shift amount cell offset
    value: int
    # the shifted type bit width
    bit_width: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_shift_range_check(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ShiftRangeCheck:
        """Decode one ShiftRangeCheck."""
        return decode_shift_range_check(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_shift_range_check(self)

    @classmethod
    def from_json(cls, value: Json) -> ShiftRangeCheck:
        """Return one ShiftRangeCheck from one JSON value."""
        return from_json_shift_range_check(value)


def encode_shift_range_check(writer: BinaryWriter, value: ShiftRangeCheck) -> None:
    """Encode one ShiftRangeCheck."""
    writer.write_unsigned(value.value)
    writer.write_byte(value.bit_width)


def decode_shift_range_check(reader: BinaryReader) -> ShiftRangeCheck:
    """Decode one ShiftRangeCheck."""
    value_ = reader.read_number()
    bit_width = reader.read_byte()

    return ShiftRangeCheck(
        value=value_,
        bit_width=bit_width,
    )


def to_json_shift_range_check(value: ShiftRangeCheck) -> Json:
    """Return one JSON value for one ShiftRangeCheck."""
    return {
        "value": value.value,
        "bitWidth": value.bit_width,
    }


def from_json_shift_range_check(value: Json) -> ShiftRangeCheck:
    """Return one ShiftRangeCheck from one JSON value."""
    object_ = json_object(value)

    return ShiftRangeCheck(
        value=json_int(json_field(object_, "value")),
        bit_width=json_int(json_field(object_, "bitWidth")),
    )


@dataclass(frozen=True, slots=True)
class NarrowCheck:
    """Integer narrowing check over one cell."""

    # the value cell offset
    value: int
    # the target bit width
    to_width: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_narrow_check(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NarrowCheck:
        """Decode one NarrowCheck."""
        return decode_narrow_check(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_narrow_check(self)

    @classmethod
    def from_json(cls, value: Json) -> NarrowCheck:
        """Return one NarrowCheck from one JSON value."""
        return from_json_narrow_check(value)


def encode_narrow_check(writer: BinaryWriter, value: NarrowCheck) -> None:
    """Encode one NarrowCheck."""
    writer.write_unsigned(value.value)
    writer.write_byte(value.to_width)


def decode_narrow_check(reader: BinaryReader) -> NarrowCheck:
    """Decode one NarrowCheck."""
    value_ = reader.read_number()
    to_width = reader.read_byte()

    return NarrowCheck(
        value=value_,
        to_width=to_width,
    )


def to_json_narrow_check(value: NarrowCheck) -> Json:
    """Return one JSON value for one NarrowCheck."""
    return {
        "value": value.value,
        "toWidth": value.to_width,
    }


def from_json_narrow_check(value: Json) -> NarrowCheck:
    """Return one NarrowCheck from one JSON value."""
    object_ = json_object(value)

    return NarrowCheck(
        value=json_int(json_field(object_, "value")),
        to_width=json_int(json_field(object_, "toWidth")),
    )


@dataclass(frozen=True, slots=True)
class OverflowCheck:
    """Two cell inputs for one overflow check."""

    # the left input cell offset
    left: int
    # the right input cell offset
    right: int
    # the input bit width
    width: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_overflow_check(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OverflowCheck:
        """Decode one OverflowCheck."""
        return decode_overflow_check(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_overflow_check(self)

    @classmethod
    def from_json(cls, value: Json) -> OverflowCheck:
        """Return one OverflowCheck from one JSON value."""
        return from_json_overflow_check(value)


def encode_overflow_check(writer: BinaryWriter, value: OverflowCheck) -> None:
    """Encode one OverflowCheck."""
    writer.write_unsigned(value.left)
    writer.write_unsigned(value.right)
    writer.write_byte(value.width)


def decode_overflow_check(reader: BinaryReader) -> OverflowCheck:
    """Decode one OverflowCheck."""
    left = reader.read_number()
    right = reader.read_number()
    width = reader.read_byte()

    return OverflowCheck(
        left=left,
        right=right,
        width=width,
    )


def to_json_overflow_check(value: OverflowCheck) -> Json:
    """Return one JSON value for one OverflowCheck."""
    return {
        "left": value.left,
        "right": value.right,
        "width": value.width,
    }


def from_json_overflow_check(value: Json) -> OverflowCheck:
    """Return one OverflowCheck from one JSON value."""
    object_ = json_object(value)

    return OverflowCheck(
        left=json_int(json_field(object_, "left")),
        right=json_int(json_field(object_, "right")),
        width=json_int(json_field(object_, "width")),
    )


@dataclass(frozen=True, slots=True)
class VariantCheck:
    """Variant tag check over one cell."""

    # the tag cell offset
    value: int
    # the expected tag
    expected: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_check(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCheck:
        """Decode one VariantCheck."""
        return decode_variant_check(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_check(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantCheck:
        """Return one VariantCheck from one JSON value."""
        return from_json_variant_check(value)


def encode_variant_check(writer: BinaryWriter, value: VariantCheck) -> None:
    """Encode one VariantCheck."""
    writer.write_unsigned(value.value)
    writer.write_unsigned(value.expected)


def decode_variant_check(reader: BinaryReader) -> VariantCheck:
    """Decode one VariantCheck."""
    value_ = reader.read_number()
    expected = reader.read_number()

    return VariantCheck(
        value=value_,
        expected=expected,
    )


def to_json_variant_check(value: VariantCheck) -> Json:
    """Return one JSON value for one VariantCheck."""
    return {
        "value": value.value,
        "expected": value.expected,
    }


def from_json_variant_check(value: Json) -> VariantCheck:
    """Return one VariantCheck from one JSON value."""
    object_ = json_object(value)

    return VariantCheck(
        value=json_int(json_field(object_, "value")),
        expected=json_int(json_field(object_, "expected")),
    )


@dataclass(frozen=True, slots=True)
class SwitchTable:
    """One pooled dense switch table."""

    # the smallest value covered by the table
    min: int
    # the table entries
    cases: Sequence[destack._generated.program.vm.function.SwitchCase]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_switch_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchTable:
        """Decode one SwitchTable."""
        return decode_switch_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_switch_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SwitchTable:
        """Return one SwitchTable from one JSON value."""
        return from_json_switch_table(value)


def encode_switch_table(writer: BinaryWriter, value: SwitchTable) -> None:
    """Encode one SwitchTable."""
    writer.write_signed(value.min)
    writer.write_unsigned(len(value.cases))
    for item_value_cases_0 in value.cases:
        destack._generated.program.vm.function.encode_switch_case(
            writer, item_value_cases_0
        )


def decode_switch_table(reader: BinaryReader) -> SwitchTable:
    """Decode one SwitchTable."""
    min = reader.read_signed_number()
    cases = [
        destack._generated.program.vm.function.decode_switch_case(reader)
        for _ in range(reader.read_number())
    ]

    return SwitchTable(
        min=min,
        cases=cases,
    )


def to_json_switch_table(value: SwitchTable) -> Json:
    """Return one JSON value for one SwitchTable."""
    return {
        "min": value.min,
        "cases": [
            destack._generated.program.vm.function.to_json_switch_case(item_0)
            for item_0 in value.cases
        ],
    }


def from_json_switch_table(value: Json) -> SwitchTable:
    """Return one SwitchTable from one JSON value."""
    object_ = json_object(value)

    return SwitchTable(
        min=json_int(json_field(object_, "min")),
        cases=[
            destack._generated.program.vm.function.from_json_switch_case(item_0)
            for item_0 in json_array(json_field(object_, "cases"))
        ],
    )


__all__ = [
    "SideTable",
    "encode_side_table",
    "decode_side_table",
    "to_json_side_table",
    "from_json_side_table",
    "SideRecordTable",
    "encode_side_record_table",
    "decode_side_record_table",
    "to_json_side_record_table",
    "from_json_side_record_table",
    "AllocationSiteId",
    "encode_allocation_site_id",
    "decode_allocation_site_id",
    "to_json_allocation_site_id",
    "from_json_allocation_site_id",
    "Edge",
    "encode_edge",
    "decode_edge",
    "to_json_edge",
    "from_json_edge",
    "SliceProjectionId",
    "encode_slice_projection_id",
    "decode_slice_projection_id",
    "to_json_slice_projection_id",
    "from_json_slice_projection_id",
    "U32RangeId",
    "encode_u32_range_id",
    "decode_u32_range_id",
    "to_json_u32_range_id",
    "from_json_u32_range_id",
    "ProjectionId",
    "encode_projection_id",
    "decode_projection_id",
    "to_json_projection_id",
    "from_json_projection_id",
    "TensorLayoutId",
    "encode_tensor_layout_id",
    "decode_tensor_layout_id",
    "to_json_tensor_layout_id",
    "from_json_tensor_layout_id",
    "TensorDotId",
    "encode_tensor_dot_id",
    "decode_tensor_dot_id",
    "to_json_tensor_dot_id",
    "from_json_tensor_dot_id",
    "TensorConvolutionId",
    "encode_tensor_convolution_id",
    "decode_tensor_convolution_id",
    "to_json_tensor_convolution_id",
    "from_json_tensor_convolution_id",
    "TensorWindowId",
    "encode_tensor_window_id",
    "decode_tensor_window_id",
    "to_json_tensor_window_id",
    "from_json_tensor_window_id",
    "TensorGatherId",
    "encode_tensor_gather_id",
    "decode_tensor_gather_id",
    "to_json_tensor_gather_id",
    "from_json_tensor_gather_id",
    "TensorScatterId",
    "encode_tensor_scatter_id",
    "decode_tensor_scatter_id",
    "to_json_tensor_scatter_id",
    "from_json_tensor_scatter_id",
    "Check",
    "encode_check",
    "decode_check",
    "to_json_check",
    "from_json_check",
    "CheckBoundsIntInt",
    "CheckBoundsIntUint",
    "CheckBoundsUintInt",
    "CheckBoundsUintUint",
    "CheckNull",
    "CheckDivZeroInt",
    "CheckDivZeroUint",
    "CheckShiftRangeInt",
    "CheckShiftRangeUint",
    "CheckNarrowInt",
    "CheckNarrowUint",
    "CheckOverflowAddInt",
    "CheckOverflowAddUint",
    "CheckOverflowSubInt",
    "CheckOverflowSubUint",
    "CheckOverflowMulInt",
    "CheckOverflowMulUint",
    "CheckOverflowDivInt",
    "CheckOverflowDivUint",
    "CheckType",
    "CheckVariant",
    "BoundsCheck",
    "encode_bounds_check",
    "decode_bounds_check",
    "to_json_bounds_check",
    "from_json_bounds_check",
    "ShiftRangeCheck",
    "encode_shift_range_check",
    "decode_shift_range_check",
    "to_json_shift_range_check",
    "from_json_shift_range_check",
    "NarrowCheck",
    "encode_narrow_check",
    "decode_narrow_check",
    "to_json_narrow_check",
    "from_json_narrow_check",
    "OverflowCheck",
    "encode_overflow_check",
    "decode_overflow_check",
    "to_json_overflow_check",
    "from_json_overflow_check",
    "VariantCheck",
    "encode_variant_check",
    "decode_variant_check",
    "to_json_variant_check",
    "from_json_variant_check",
    "SwitchTable",
    "encode_switch_table",
    "decode_switch_table",
    "to_json_switch_table",
    "from_json_switch_table",
]
