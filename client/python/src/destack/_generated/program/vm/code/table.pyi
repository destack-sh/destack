# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SideTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SideTable: ...

def encode_side_table(writer: BinaryWriter, value: SideTable) -> None: ...
def decode_side_table(reader: BinaryReader) -> SideTable: ...
def to_json_side_table(value: SideTable) -> Json: ...
def from_json_side_table(value: Json) -> SideTable: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SideRecordTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SideRecordTable: ...

def encode_side_record_table(writer: BinaryWriter, value: SideRecordTable) -> None: ...
def decode_side_record_table(reader: BinaryReader) -> SideRecordTable: ...
def to_json_side_record_table(value: SideRecordTable) -> Json: ...
def from_json_side_record_table(value: Json) -> SideRecordTable: ...

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
