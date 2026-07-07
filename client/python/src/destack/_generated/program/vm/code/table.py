# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.core.section


@dataclass(frozen=True, slots=True)
class SideTable:
    """Immutable side table referenced by compact side records."""

    # pooled side records
    record: SideRecordTable
    # pooled allocation plans
    allocation_plan: destack._generated.core.section.SectionSlice
    # pooled small allocation plans
    small_allocation_plan: destack._generated.core.section.SectionSlice
    # pooled constants
    constant: destack._generated.core.section.SectionSlice
    # flattened constant bytes
    constant_bytes: destack._generated.core.section.SectionSlice
    # pooled address projections
    projection: destack._generated.core.section.SectionSlice
    # pooled slice projections
    slice_projection: destack._generated.core.section.SectionSlice
    # pooled check constraints
    check: destack._generated.core.section.SectionSlice
    # pooled switch case tables
    switch_cases: destack._generated.core.section.SectionSlice
    # flattened switch case entries
    switch_case_entries: destack._generated.core.section.SectionSlice
    # pooled dense switch tables
    switch_table: destack._generated.core.section.SectionSlice
    # pooled control edges
    edge: destack._generated.core.section.SectionSlice
    # pooled u32 slices
    u32_ranges: destack._generated.core.section.SectionSlice
    # flattened u32 entries
    u32_entries: destack._generated.core.section.SectionSlice
    # flattened tensor u64 entries
    tensor_u64_entries: destack._generated.core.section.SectionSlice
    # flattened tensor flag entries
    tensor_flag_entries: destack._generated.core.section.SectionSlice
    # pooled tensor dot descriptors
    tensor_dot: destack._generated.core.section.SectionSlice
    # pooled tensor convolution dimension descriptors
    tensor_convolution: destack._generated.core.section.SectionSlice
    # pooled tensor convolution window descriptors
    tensor_window: destack._generated.core.section.SectionSlice
    # pooled tensor gather descriptors
    tensor_gather: destack._generated.core.section.SectionSlice
    # pooled tensor scatter descriptors
    tensor_scatter: destack._generated.core.section.SectionSlice
    # pooled tensor layouts
    tensor_layout: destack._generated.core.section.SectionSlice
    # pooled callable signatures
    signature: destack._generated.core.section.SectionSlice
    # flattened callable signature parameters
    signature_parameters: destack._generated.core.section.SectionSlice

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
    destack._generated.core.section.encode_section_slice(writer, value.allocation_plan)
    destack._generated.core.section.encode_section_slice(
        writer, value.small_allocation_plan
    )
    destack._generated.core.section.encode_section_slice(writer, value.constant)
    destack._generated.core.section.encode_section_slice(writer, value.constant_bytes)
    destack._generated.core.section.encode_section_slice(writer, value.projection)
    destack._generated.core.section.encode_section_slice(writer, value.slice_projection)
    destack._generated.core.section.encode_section_slice(writer, value.check)
    destack._generated.core.section.encode_section_slice(writer, value.switch_cases)
    destack._generated.core.section.encode_section_slice(
        writer, value.switch_case_entries
    )
    destack._generated.core.section.encode_section_slice(writer, value.switch_table)
    destack._generated.core.section.encode_section_slice(writer, value.edge)
    destack._generated.core.section.encode_section_slice(writer, value.u32_ranges)
    destack._generated.core.section.encode_section_slice(writer, value.u32_entries)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_u64_entries
    )
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_flag_entries
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_dot)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_convolution
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_window)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_gather)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_scatter)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_layout)
    destack._generated.core.section.encode_section_slice(writer, value.signature)
    destack._generated.core.section.encode_section_slice(
        writer, value.signature_parameters
    )


def decode_side_table(reader: BinaryReader) -> SideTable:
    """Decode one SideTable."""
    record = decode_side_record_table(reader)
    allocation_plan = destack._generated.core.section.decode_section_slice(reader)
    small_allocation_plan = destack._generated.core.section.decode_section_slice(reader)
    constant = destack._generated.core.section.decode_section_slice(reader)
    constant_bytes = destack._generated.core.section.decode_section_slice(reader)
    projection = destack._generated.core.section.decode_section_slice(reader)
    slice_projection = destack._generated.core.section.decode_section_slice(reader)
    check = destack._generated.core.section.decode_section_slice(reader)
    switch_cases = destack._generated.core.section.decode_section_slice(reader)
    switch_case_entries = destack._generated.core.section.decode_section_slice(reader)
    switch_table = destack._generated.core.section.decode_section_slice(reader)
    edge = destack._generated.core.section.decode_section_slice(reader)
    u32_ranges = destack._generated.core.section.decode_section_slice(reader)
    u32_entries = destack._generated.core.section.decode_section_slice(reader)
    tensor_u64_entries = destack._generated.core.section.decode_section_slice(reader)
    tensor_flag_entries = destack._generated.core.section.decode_section_slice(reader)
    tensor_dot = destack._generated.core.section.decode_section_slice(reader)
    tensor_convolution = destack._generated.core.section.decode_section_slice(reader)
    tensor_window = destack._generated.core.section.decode_section_slice(reader)
    tensor_gather = destack._generated.core.section.decode_section_slice(reader)
    tensor_scatter = destack._generated.core.section.decode_section_slice(reader)
    tensor_layout = destack._generated.core.section.decode_section_slice(reader)
    signature = destack._generated.core.section.decode_section_slice(reader)
    signature_parameters = destack._generated.core.section.decode_section_slice(reader)

    return SideTable(
        record=record,
        allocation_plan=allocation_plan,
        small_allocation_plan=small_allocation_plan,
        constant=constant,
        constant_bytes=constant_bytes,
        projection=projection,
        slice_projection=slice_projection,
        check=check,
        switch_cases=switch_cases,
        switch_case_entries=switch_case_entries,
        switch_table=switch_table,
        edge=edge,
        u32_ranges=u32_ranges,
        u32_entries=u32_entries,
        tensor_u64_entries=tensor_u64_entries,
        tensor_flag_entries=tensor_flag_entries,
        tensor_dot=tensor_dot,
        tensor_convolution=tensor_convolution,
        tensor_window=tensor_window,
        tensor_gather=tensor_gather,
        tensor_scatter=tensor_scatter,
        tensor_layout=tensor_layout,
        signature=signature,
        signature_parameters=signature_parameters,
    )


def to_json_side_table(value: SideTable) -> Json:
    """Return one JSON value for one SideTable."""
    return {
        "record": to_json_side_record_table(value.record),
        "allocationPlan": destack._generated.core.section.to_json_section_slice(
            value.allocation_plan
        ),
        "smallAllocationPlan": destack._generated.core.section.to_json_section_slice(
            value.small_allocation_plan
        ),
        "constant": destack._generated.core.section.to_json_section_slice(
            value.constant
        ),
        "constantBytes": destack._generated.core.section.to_json_section_slice(
            value.constant_bytes
        ),
        "projection": destack._generated.core.section.to_json_section_slice(
            value.projection
        ),
        "sliceProjection": destack._generated.core.section.to_json_section_slice(
            value.slice_projection
        ),
        "check": destack._generated.core.section.to_json_section_slice(value.check),
        "switchCases": destack._generated.core.section.to_json_section_slice(
            value.switch_cases
        ),
        "switchCaseEntries": destack._generated.core.section.to_json_section_slice(
            value.switch_case_entries
        ),
        "switchTable": destack._generated.core.section.to_json_section_slice(
            value.switch_table
        ),
        "edge": destack._generated.core.section.to_json_section_slice(value.edge),
        "u32Ranges": destack._generated.core.section.to_json_section_slice(
            value.u32_ranges
        ),
        "u32Entries": destack._generated.core.section.to_json_section_slice(
            value.u32_entries
        ),
        "tensorU64Entries": destack._generated.core.section.to_json_section_slice(
            value.tensor_u64_entries
        ),
        "tensorFlagEntries": destack._generated.core.section.to_json_section_slice(
            value.tensor_flag_entries
        ),
        "tensorDot": destack._generated.core.section.to_json_section_slice(
            value.tensor_dot
        ),
        "tensorConvolution": destack._generated.core.section.to_json_section_slice(
            value.tensor_convolution
        ),
        "tensorWindow": destack._generated.core.section.to_json_section_slice(
            value.tensor_window
        ),
        "tensorGather": destack._generated.core.section.to_json_section_slice(
            value.tensor_gather
        ),
        "tensorScatter": destack._generated.core.section.to_json_section_slice(
            value.tensor_scatter
        ),
        "tensorLayout": destack._generated.core.section.to_json_section_slice(
            value.tensor_layout
        ),
        "signature": destack._generated.core.section.to_json_section_slice(
            value.signature
        ),
        "signatureParameters": destack._generated.core.section.to_json_section_slice(
            value.signature_parameters
        ),
    }


def from_json_side_table(value: Json) -> SideTable:
    """Return one SideTable from one JSON value."""
    object_ = json_object(value)

    return SideTable(
        record=from_json_side_record_table(json_field(object_, "record")),
        allocation_plan=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "allocationPlan")
        ),
        small_allocation_plan=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "smallAllocationPlan")
        ),
        constant=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "constant")
        ),
        constant_bytes=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "constantBytes")
        ),
        projection=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "projection")
        ),
        slice_projection=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "sliceProjection")
        ),
        check=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "check")
        ),
        switch_cases=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "switchCases")
        ),
        switch_case_entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "switchCaseEntries")
        ),
        switch_table=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "switchTable")
        ),
        edge=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "edge")
        ),
        u32_ranges=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "u32Ranges")
        ),
        u32_entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "u32Entries")
        ),
        tensor_u64_entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorU64Entries")
        ),
        tensor_flag_entries=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorFlagEntries")
        ),
        tensor_dot=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorDot")
        ),
        tensor_convolution=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorConvolution")
        ),
        tensor_window=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorWindow")
        ),
        tensor_gather=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorGather")
        ),
        tensor_scatter=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorScatter")
        ),
        tensor_layout=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorLayout")
        ),
        signature=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "signature")
        ),
        signature_parameters=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "signatureParameters")
        ),
    )


@dataclass(frozen=True, slots=True)
class SideRecordTable:
    """Immutable side records referenced by instruction ids."""

    aggregate_select: destack._generated.core.section.SectionSlice
    allocation_branch: destack._generated.core.section.SectionSlice
    slice_allocation_branch: destack._generated.core.section.SectionSlice
    atomic_compare_exchange: destack._generated.core.section.SectionSlice
    vector_splat: destack._generated.core.section.SectionSlice
    vector_extract: destack._generated.core.section.SectionSlice
    vector_binary: destack._generated.core.section.SectionSlice
    vector_unary: destack._generated.core.section.SectionSlice
    vector_insert: destack._generated.core.section.SectionSlice
    vector_shuffle: destack._generated.core.section.SectionSlice
    vector_select: destack._generated.core.section.SectionSlice
    vector_reduce: destack._generated.core.section.SectionSlice
    vector_convert: destack._generated.core.section.SectionSlice
    function_bind: destack._generated.core.section.SectionSlice
    call: destack._generated.core.section.SectionSlice
    call_branch: destack._generated.core.section.SectionSlice
    call_virtual: destack._generated.core.section.SectionSlice
    call_virtual_branch: destack._generated.core.section.SectionSlice
    call_dynamic: destack._generated.core.section.SectionSlice
    call_dynamic_branch: destack._generated.core.section.SectionSlice
    indirect_call: destack._generated.core.section.SectionSlice
    indirect_call_branch: destack._generated.core.section.SectionSlice
    tensor_load: destack._generated.core.section.SectionSlice
    tensor_extract: destack._generated.core.section.SectionSlice
    tensor_binary: destack._generated.core.section.SectionSlice
    tensor_contiguous_binary: destack._generated.core.section.SectionSlice
    tensor_unary: destack._generated.core.section.SectionSlice
    tensor_contiguous_unary: destack._generated.core.section.SectionSlice
    tensor_store: destack._generated.core.section.SectionSlice
    tensor_fill: destack._generated.core.section.SectionSlice
    tensor_copy: destack._generated.core.section.SectionSlice
    tensor_view_cast: destack._generated.core.section.SectionSlice
    tensor_reshape: destack._generated.core.section.SectionSlice
    tensor_broadcast: destack._generated.core.section.SectionSlice
    tensor_transpose: destack._generated.core.section.SectionSlice
    tensor_slice: destack._generated.core.section.SectionSlice
    tensor_pad: destack._generated.core.section.SectionSlice
    tensor_concat: destack._generated.core.section.SectionSlice
    tensor_reduce: destack._generated.core.section.SectionSlice
    tensor_index_reduce: destack._generated.core.section.SectionSlice
    tensor_dot: destack._generated.core.section.SectionSlice
    tensor_convolution: destack._generated.core.section.SectionSlice
    tensor_gather: destack._generated.core.section.SectionSlice
    tensor_scatter: destack._generated.core.section.SectionSlice
    tensor_select: destack._generated.core.section.SectionSlice
    tensor_convert: destack._generated.core.section.SectionSlice
    tensor_view: destack._generated.core.section.SectionSlice
    intrinsic: destack._generated.core.section.SectionSlice
    tail_call: destack._generated.core.section.SectionSlice
    tail_call_virtual: destack._generated.core.section.SectionSlice
    tail_call_dynamic: destack._generated.core.section.SectionSlice
    indirect_tail_call: destack._generated.core.section.SectionSlice

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
    destack._generated.core.section.encode_section_slice(writer, value.aggregate_select)
    destack._generated.core.section.encode_section_slice(
        writer, value.allocation_branch
    )
    destack._generated.core.section.encode_section_slice(
        writer, value.slice_allocation_branch
    )
    destack._generated.core.section.encode_section_slice(
        writer, value.atomic_compare_exchange
    )
    destack._generated.core.section.encode_section_slice(writer, value.vector_splat)
    destack._generated.core.section.encode_section_slice(writer, value.vector_extract)
    destack._generated.core.section.encode_section_slice(writer, value.vector_binary)
    destack._generated.core.section.encode_section_slice(writer, value.vector_unary)
    destack._generated.core.section.encode_section_slice(writer, value.vector_insert)
    destack._generated.core.section.encode_section_slice(writer, value.vector_shuffle)
    destack._generated.core.section.encode_section_slice(writer, value.vector_select)
    destack._generated.core.section.encode_section_slice(writer, value.vector_reduce)
    destack._generated.core.section.encode_section_slice(writer, value.vector_convert)
    destack._generated.core.section.encode_section_slice(writer, value.function_bind)
    destack._generated.core.section.encode_section_slice(writer, value.call)
    destack._generated.core.section.encode_section_slice(writer, value.call_branch)
    destack._generated.core.section.encode_section_slice(writer, value.call_virtual)
    destack._generated.core.section.encode_section_slice(
        writer, value.call_virtual_branch
    )
    destack._generated.core.section.encode_section_slice(writer, value.call_dynamic)
    destack._generated.core.section.encode_section_slice(
        writer, value.call_dynamic_branch
    )
    destack._generated.core.section.encode_section_slice(writer, value.indirect_call)
    destack._generated.core.section.encode_section_slice(
        writer, value.indirect_call_branch
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_load)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_extract)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_binary)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_contiguous_binary
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_unary)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_contiguous_unary
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_store)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_fill)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_copy)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_view_cast)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_reshape)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_broadcast)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_transpose)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_slice)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_pad)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_concat)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_reduce)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_index_reduce
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_dot)
    destack._generated.core.section.encode_section_slice(
        writer, value.tensor_convolution
    )
    destack._generated.core.section.encode_section_slice(writer, value.tensor_gather)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_scatter)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_select)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_convert)
    destack._generated.core.section.encode_section_slice(writer, value.tensor_view)
    destack._generated.core.section.encode_section_slice(writer, value.intrinsic)
    destack._generated.core.section.encode_section_slice(writer, value.tail_call)
    destack._generated.core.section.encode_section_slice(
        writer, value.tail_call_virtual
    )
    destack._generated.core.section.encode_section_slice(
        writer, value.tail_call_dynamic
    )
    destack._generated.core.section.encode_section_slice(
        writer, value.indirect_tail_call
    )


def decode_side_record_table(reader: BinaryReader) -> SideRecordTable:
    """Decode one SideRecordTable."""
    aggregate_select = destack._generated.core.section.decode_section_slice(reader)
    allocation_branch = destack._generated.core.section.decode_section_slice(reader)
    slice_allocation_branch = destack._generated.core.section.decode_section_slice(
        reader
    )
    atomic_compare_exchange = destack._generated.core.section.decode_section_slice(
        reader
    )
    vector_splat = destack._generated.core.section.decode_section_slice(reader)
    vector_extract = destack._generated.core.section.decode_section_slice(reader)
    vector_binary = destack._generated.core.section.decode_section_slice(reader)
    vector_unary = destack._generated.core.section.decode_section_slice(reader)
    vector_insert = destack._generated.core.section.decode_section_slice(reader)
    vector_shuffle = destack._generated.core.section.decode_section_slice(reader)
    vector_select = destack._generated.core.section.decode_section_slice(reader)
    vector_reduce = destack._generated.core.section.decode_section_slice(reader)
    vector_convert = destack._generated.core.section.decode_section_slice(reader)
    function_bind = destack._generated.core.section.decode_section_slice(reader)
    call = destack._generated.core.section.decode_section_slice(reader)
    call_branch = destack._generated.core.section.decode_section_slice(reader)
    call_virtual = destack._generated.core.section.decode_section_slice(reader)
    call_virtual_branch = destack._generated.core.section.decode_section_slice(reader)
    call_dynamic = destack._generated.core.section.decode_section_slice(reader)
    call_dynamic_branch = destack._generated.core.section.decode_section_slice(reader)
    indirect_call = destack._generated.core.section.decode_section_slice(reader)
    indirect_call_branch = destack._generated.core.section.decode_section_slice(reader)
    tensor_load = destack._generated.core.section.decode_section_slice(reader)
    tensor_extract = destack._generated.core.section.decode_section_slice(reader)
    tensor_binary = destack._generated.core.section.decode_section_slice(reader)
    tensor_contiguous_binary = destack._generated.core.section.decode_section_slice(
        reader
    )
    tensor_unary = destack._generated.core.section.decode_section_slice(reader)
    tensor_contiguous_unary = destack._generated.core.section.decode_section_slice(
        reader
    )
    tensor_store = destack._generated.core.section.decode_section_slice(reader)
    tensor_fill = destack._generated.core.section.decode_section_slice(reader)
    tensor_copy = destack._generated.core.section.decode_section_slice(reader)
    tensor_view_cast = destack._generated.core.section.decode_section_slice(reader)
    tensor_reshape = destack._generated.core.section.decode_section_slice(reader)
    tensor_broadcast = destack._generated.core.section.decode_section_slice(reader)
    tensor_transpose = destack._generated.core.section.decode_section_slice(reader)
    tensor_slice = destack._generated.core.section.decode_section_slice(reader)
    tensor_pad = destack._generated.core.section.decode_section_slice(reader)
    tensor_concat = destack._generated.core.section.decode_section_slice(reader)
    tensor_reduce = destack._generated.core.section.decode_section_slice(reader)
    tensor_index_reduce = destack._generated.core.section.decode_section_slice(reader)
    tensor_dot = destack._generated.core.section.decode_section_slice(reader)
    tensor_convolution = destack._generated.core.section.decode_section_slice(reader)
    tensor_gather = destack._generated.core.section.decode_section_slice(reader)
    tensor_scatter = destack._generated.core.section.decode_section_slice(reader)
    tensor_select = destack._generated.core.section.decode_section_slice(reader)
    tensor_convert = destack._generated.core.section.decode_section_slice(reader)
    tensor_view = destack._generated.core.section.decode_section_slice(reader)
    intrinsic = destack._generated.core.section.decode_section_slice(reader)
    tail_call = destack._generated.core.section.decode_section_slice(reader)
    tail_call_virtual = destack._generated.core.section.decode_section_slice(reader)
    tail_call_dynamic = destack._generated.core.section.decode_section_slice(reader)
    indirect_tail_call = destack._generated.core.section.decode_section_slice(reader)

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
        "aggregateSelect": destack._generated.core.section.to_json_section_slice(
            value.aggregate_select
        ),
        "allocationBranch": destack._generated.core.section.to_json_section_slice(
            value.allocation_branch
        ),
        "sliceAllocationBranch": destack._generated.core.section.to_json_section_slice(
            value.slice_allocation_branch
        ),
        "atomicCompareExchange": destack._generated.core.section.to_json_section_slice(
            value.atomic_compare_exchange
        ),
        "vectorSplat": destack._generated.core.section.to_json_section_slice(
            value.vector_splat
        ),
        "vectorExtract": destack._generated.core.section.to_json_section_slice(
            value.vector_extract
        ),
        "vectorBinary": destack._generated.core.section.to_json_section_slice(
            value.vector_binary
        ),
        "vectorUnary": destack._generated.core.section.to_json_section_slice(
            value.vector_unary
        ),
        "vectorInsert": destack._generated.core.section.to_json_section_slice(
            value.vector_insert
        ),
        "vectorShuffle": destack._generated.core.section.to_json_section_slice(
            value.vector_shuffle
        ),
        "vectorSelect": destack._generated.core.section.to_json_section_slice(
            value.vector_select
        ),
        "vectorReduce": destack._generated.core.section.to_json_section_slice(
            value.vector_reduce
        ),
        "vectorConvert": destack._generated.core.section.to_json_section_slice(
            value.vector_convert
        ),
        "functionBind": destack._generated.core.section.to_json_section_slice(
            value.function_bind
        ),
        "call": destack._generated.core.section.to_json_section_slice(value.call),
        "callBranch": destack._generated.core.section.to_json_section_slice(
            value.call_branch
        ),
        "callVirtual": destack._generated.core.section.to_json_section_slice(
            value.call_virtual
        ),
        "callVirtualBranch": destack._generated.core.section.to_json_section_slice(
            value.call_virtual_branch
        ),
        "callDynamic": destack._generated.core.section.to_json_section_slice(
            value.call_dynamic
        ),
        "callDynamicBranch": destack._generated.core.section.to_json_section_slice(
            value.call_dynamic_branch
        ),
        "indirectCall": destack._generated.core.section.to_json_section_slice(
            value.indirect_call
        ),
        "indirectCallBranch": destack._generated.core.section.to_json_section_slice(
            value.indirect_call_branch
        ),
        "tensorLoad": destack._generated.core.section.to_json_section_slice(
            value.tensor_load
        ),
        "tensorExtract": destack._generated.core.section.to_json_section_slice(
            value.tensor_extract
        ),
        "tensorBinary": destack._generated.core.section.to_json_section_slice(
            value.tensor_binary
        ),
        "tensorContiguousBinary": destack._generated.core.section.to_json_section_slice(
            value.tensor_contiguous_binary
        ),
        "tensorUnary": destack._generated.core.section.to_json_section_slice(
            value.tensor_unary
        ),
        "tensorContiguousUnary": destack._generated.core.section.to_json_section_slice(
            value.tensor_contiguous_unary
        ),
        "tensorStore": destack._generated.core.section.to_json_section_slice(
            value.tensor_store
        ),
        "tensorFill": destack._generated.core.section.to_json_section_slice(
            value.tensor_fill
        ),
        "tensorCopy": destack._generated.core.section.to_json_section_slice(
            value.tensor_copy
        ),
        "tensorViewCast": destack._generated.core.section.to_json_section_slice(
            value.tensor_view_cast
        ),
        "tensorReshape": destack._generated.core.section.to_json_section_slice(
            value.tensor_reshape
        ),
        "tensorBroadcast": destack._generated.core.section.to_json_section_slice(
            value.tensor_broadcast
        ),
        "tensorTranspose": destack._generated.core.section.to_json_section_slice(
            value.tensor_transpose
        ),
        "tensorSlice": destack._generated.core.section.to_json_section_slice(
            value.tensor_slice
        ),
        "tensorPad": destack._generated.core.section.to_json_section_slice(
            value.tensor_pad
        ),
        "tensorConcat": destack._generated.core.section.to_json_section_slice(
            value.tensor_concat
        ),
        "tensorReduce": destack._generated.core.section.to_json_section_slice(
            value.tensor_reduce
        ),
        "tensorIndexReduce": destack._generated.core.section.to_json_section_slice(
            value.tensor_index_reduce
        ),
        "tensorDot": destack._generated.core.section.to_json_section_slice(
            value.tensor_dot
        ),
        "tensorConvolution": destack._generated.core.section.to_json_section_slice(
            value.tensor_convolution
        ),
        "tensorGather": destack._generated.core.section.to_json_section_slice(
            value.tensor_gather
        ),
        "tensorScatter": destack._generated.core.section.to_json_section_slice(
            value.tensor_scatter
        ),
        "tensorSelect": destack._generated.core.section.to_json_section_slice(
            value.tensor_select
        ),
        "tensorConvert": destack._generated.core.section.to_json_section_slice(
            value.tensor_convert
        ),
        "tensorView": destack._generated.core.section.to_json_section_slice(
            value.tensor_view
        ),
        "intrinsic": destack._generated.core.section.to_json_section_slice(
            value.intrinsic
        ),
        "tailCall": destack._generated.core.section.to_json_section_slice(
            value.tail_call
        ),
        "tailCallVirtual": destack._generated.core.section.to_json_section_slice(
            value.tail_call_virtual
        ),
        "tailCallDynamic": destack._generated.core.section.to_json_section_slice(
            value.tail_call_dynamic
        ),
        "indirectTailCall": destack._generated.core.section.to_json_section_slice(
            value.indirect_tail_call
        ),
    }


def from_json_side_record_table(value: Json) -> SideRecordTable:
    """Return one SideRecordTable from one JSON value."""
    object_ = json_object(value)

    return SideRecordTable(
        aggregate_select=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "aggregateSelect")
        ),
        allocation_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "allocationBranch")
        ),
        slice_allocation_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "sliceAllocationBranch")
        ),
        atomic_compare_exchange=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "atomicCompareExchange")
        ),
        vector_splat=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorSplat")
        ),
        vector_extract=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorExtract")
        ),
        vector_binary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorBinary")
        ),
        vector_unary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorUnary")
        ),
        vector_insert=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorInsert")
        ),
        vector_shuffle=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorShuffle")
        ),
        vector_select=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorSelect")
        ),
        vector_reduce=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorReduce")
        ),
        vector_convert=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "vectorConvert")
        ),
        function_bind=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "functionBind")
        ),
        call=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "call")
        ),
        call_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callBranch")
        ),
        call_virtual=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callVirtual")
        ),
        call_virtual_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callVirtualBranch")
        ),
        call_dynamic=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callDynamic")
        ),
        call_dynamic_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "callDynamicBranch")
        ),
        indirect_call=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "indirectCall")
        ),
        indirect_call_branch=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "indirectCallBranch")
        ),
        tensor_load=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorLoad")
        ),
        tensor_extract=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorExtract")
        ),
        tensor_binary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorBinary")
        ),
        tensor_contiguous_binary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorContiguousBinary")
        ),
        tensor_unary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorUnary")
        ),
        tensor_contiguous_unary=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorContiguousUnary")
        ),
        tensor_store=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorStore")
        ),
        tensor_fill=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorFill")
        ),
        tensor_copy=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorCopy")
        ),
        tensor_view_cast=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorViewCast")
        ),
        tensor_reshape=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorReshape")
        ),
        tensor_broadcast=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorBroadcast")
        ),
        tensor_transpose=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorTranspose")
        ),
        tensor_slice=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorSlice")
        ),
        tensor_pad=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorPad")
        ),
        tensor_concat=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorConcat")
        ),
        tensor_reduce=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorReduce")
        ),
        tensor_index_reduce=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorIndexReduce")
        ),
        tensor_dot=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorDot")
        ),
        tensor_convolution=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorConvolution")
        ),
        tensor_gather=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorGather")
        ),
        tensor_scatter=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorScatter")
        ),
        tensor_select=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorSelect")
        ),
        tensor_convert=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorConvert")
        ),
        tensor_view=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tensorView")
        ),
        intrinsic=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "intrinsic")
        ),
        tail_call=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tailCall")
        ),
        tail_call_virtual=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tailCallVirtual")
        ),
        tail_call_dynamic=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "tailCallDynamic")
        ),
        indirect_tail_call=destack._generated.core.section.from_json_section_slice(
            json_field(object_, "indirectTailCall")
        ),
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
]
