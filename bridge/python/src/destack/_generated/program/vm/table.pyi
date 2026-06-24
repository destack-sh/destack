# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

"""Identifier for one pooled allocation site."""
AllocationSiteId: typing.TypeAlias = int

def encode_allocation_site_id(
    writer: BinaryWriter, value: AllocationSiteId
) -> None: ...
def decode_allocation_site_id(reader: BinaryReader) -> AllocationSiteId: ...
def to_json_allocation_site_id(value: AllocationSiteId) -> Json: ...
def from_json_allocation_site_id(value: Json) -> AllocationSiteId: ...

@dataclass(frozen=True, slots=True)
class Edge:
    """One lowered control-flow edge."""

    # the target block
    target: int
    # the block-parameter moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Edge: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Edge: ...

def encode_edge(writer: BinaryWriter, value: Edge) -> None: ...
def decode_edge(reader: BinaryReader) -> Edge: ...
def to_json_edge(value: Edge) -> Json: ...
def from_json_edge(value: Json) -> Edge: ...

"""Identifier for one pooled slice projection."""
SliceProjectionId: typing.TypeAlias = int

def encode_slice_projection_id(
    writer: BinaryWriter, value: SliceProjectionId
) -> None: ...
def decode_slice_projection_id(reader: BinaryReader) -> SliceProjectionId: ...
def to_json_slice_projection_id(value: SliceProjectionId) -> Json: ...
def from_json_slice_projection_id(value: Json) -> SliceProjectionId: ...

"""Identifier for one pooled u32 slice."""
U32RangeId: typing.TypeAlias = int

def encode_u32_range_id(writer: BinaryWriter, value: U32RangeId) -> None: ...
def decode_u32_range_id(reader: BinaryReader) -> U32RangeId: ...
def to_json_u32_range_id(value: U32RangeId) -> Json: ...
def from_json_u32_range_id(value: Json) -> U32RangeId: ...

"""Identifier for one pooled address projection."""
ProjectionId: typing.TypeAlias = int

def encode_projection_id(writer: BinaryWriter, value: ProjectionId) -> None: ...
def decode_projection_id(reader: BinaryReader) -> ProjectionId: ...
def to_json_projection_id(value: ProjectionId) -> Json: ...
def from_json_projection_id(value: Json) -> ProjectionId: ...

"""Identifier for one pooled tensor layout."""
TensorLayoutId: typing.TypeAlias = int

def encode_tensor_layout_id(writer: BinaryWriter, value: TensorLayoutId) -> None: ...
def decode_tensor_layout_id(reader: BinaryReader) -> TensorLayoutId: ...
def to_json_tensor_layout_id(value: TensorLayoutId) -> Json: ...
def from_json_tensor_layout_id(value: Json) -> TensorLayoutId: ...

"""Identifier for one pooled tensor dot descriptor."""
TensorDotId: typing.TypeAlias = int

def encode_tensor_dot_id(writer: BinaryWriter, value: TensorDotId) -> None: ...
def decode_tensor_dot_id(reader: BinaryReader) -> TensorDotId: ...
def to_json_tensor_dot_id(value: TensorDotId) -> Json: ...
def from_json_tensor_dot_id(value: Json) -> TensorDotId: ...

"""Identifier for one pooled tensor convolution dimension descriptor."""
TensorConvolutionId: typing.TypeAlias = int

def encode_tensor_convolution_id(
    writer: BinaryWriter, value: TensorConvolutionId
) -> None: ...
def decode_tensor_convolution_id(reader: BinaryReader) -> TensorConvolutionId: ...
def to_json_tensor_convolution_id(value: TensorConvolutionId) -> Json: ...
def from_json_tensor_convolution_id(value: Json) -> TensorConvolutionId: ...

"""Identifier for one pooled tensor convolution window descriptor."""
TensorWindowId: typing.TypeAlias = int

def encode_tensor_window_id(writer: BinaryWriter, value: TensorWindowId) -> None: ...
def decode_tensor_window_id(reader: BinaryReader) -> TensorWindowId: ...
def to_json_tensor_window_id(value: TensorWindowId) -> Json: ...
def from_json_tensor_window_id(value: Json) -> TensorWindowId: ...

"""Identifier for one pooled tensor gather descriptor."""
TensorGatherId: typing.TypeAlias = int

def encode_tensor_gather_id(writer: BinaryWriter, value: TensorGatherId) -> None: ...
def decode_tensor_gather_id(reader: BinaryReader) -> TensorGatherId: ...
def to_json_tensor_gather_id(value: TensorGatherId) -> Json: ...
def from_json_tensor_gather_id(value: Json) -> TensorGatherId: ...

"""Identifier for one pooled tensor scatter descriptor."""
TensorScatterId: typing.TypeAlias = int

def encode_tensor_scatter_id(writer: BinaryWriter, value: TensorScatterId) -> None: ...
def decode_tensor_scatter_id(reader: BinaryReader) -> TensorScatterId: ...
def to_json_tensor_scatter_id(value: TensorScatterId) -> Json: ...
def from_json_tensor_scatter_id(value: Json) -> TensorScatterId: ...

@dataclass(frozen=True, slots=True)
class CheckBoundsIntInt:
    """Bounds check over signed index and signed length cells."""

    bounds_int_int: BoundsCheck
    kind: typing.Literal["boundsIntInt"] = "boundsIntInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckBoundsIntUint:
    """Bounds check over signed index and unsigned length cells."""

    bounds_int_uint: BoundsCheck
    kind: typing.Literal["boundsIntUint"] = "boundsIntUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckBoundsUintInt:
    """Bounds check over unsigned index and signed length cells."""

    bounds_uint_int: BoundsCheck
    kind: typing.Literal["boundsUintInt"] = "boundsUintInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckBoundsUintUint:
    """Bounds check over unsigned index and unsigned length cells."""

    bounds_uint_uint: BoundsCheck
    kind: typing.Literal["boundsUintUint"] = "boundsUintUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckNull:
    """Non-null check over one cell."""

    # the value cell offset
    value: int
    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckDivZeroInt:
    """Division-by-zero check over one signed cell."""

    divisor: int
    kind: typing.Literal["divZeroInt"] = "divZeroInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckDivZeroUint:
    """Division-by-zero check over one unsigned cell."""

    divisor: int
    kind: typing.Literal["divZeroUint"] = "divZeroUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckShiftRangeInt:
    """Shift range check over one signed shift amount cell."""

    shift_range_int: ShiftRangeCheck
    kind: typing.Literal["shiftRangeInt"] = "shiftRangeInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckShiftRangeUint:
    """Shift range check over one unsigned shift amount cell."""

    shift_range_uint: ShiftRangeCheck
    kind: typing.Literal["shiftRangeUint"] = "shiftRangeUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckNarrowInt:
    """Signed integer narrowing check over one cell."""

    narrow_int: NarrowCheck
    kind: typing.Literal["narrowInt"] = "narrowInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckNarrowUint:
    """Unsigned integer narrowing check over one cell."""

    narrow_uint: NarrowCheck
    kind: typing.Literal["narrowUint"] = "narrowUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowAddInt:
    """Signed add overflow check over two cells."""

    overflow_add_int: OverflowCheck
    kind: typing.Literal["overflowAddInt"] = "overflowAddInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowAddUint:
    """Unsigned add overflow check over two cells."""

    overflow_add_uint: OverflowCheck
    kind: typing.Literal["overflowAddUint"] = "overflowAddUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowSubInt:
    """Signed subtract overflow check over two cells."""

    overflow_sub_int: OverflowCheck
    kind: typing.Literal["overflowSubInt"] = "overflowSubInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowSubUint:
    """Unsigned subtract overflow check over two cells."""

    overflow_sub_uint: OverflowCheck
    kind: typing.Literal["overflowSubUint"] = "overflowSubUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowMulInt:
    """Signed multiply overflow check over two cells."""

    overflow_mul_int: OverflowCheck
    kind: typing.Literal["overflowMulInt"] = "overflowMulInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowMulUint:
    """Unsigned multiply overflow check over two cells."""

    overflow_mul_uint: OverflowCheck
    kind: typing.Literal["overflowMulUint"] = "overflowMulUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowDivInt:
    """Signed divide or remainder overflow check over two cells."""

    overflow_div_int: OverflowCheck
    kind: typing.Literal["overflowDivInt"] = "overflowDivInt"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckOverflowDivUint:
    """Unsigned divide or remainder overflow check over two cells."""

    overflow_div_uint: OverflowCheck
    kind: typing.Literal["overflowDivUint"] = "overflowDivUint"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckType:
    """Runtime type descriptor check."""

    # the descriptor cell offset
    value: int
    # the expected type id
    expected: int
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CheckVariant:
    """Variant tag check."""

    variant: VariantCheck
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_check(writer: BinaryWriter, value: Check) -> None: ...
def decode_check(reader: BinaryReader) -> Check: ...
def to_json_check(value: Check) -> Json: ...
def from_json_check(value: Json) -> Check: ...

@dataclass(frozen=True, slots=True)
class BoundsCheck:
    """Bounds check over index and length cells."""

    # the index cell offset
    index: int
    # the length cell offset
    length: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BoundsCheck: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BoundsCheck: ...

def encode_bounds_check(writer: BinaryWriter, value: BoundsCheck) -> None: ...
def decode_bounds_check(reader: BinaryReader) -> BoundsCheck: ...
def to_json_bounds_check(value: BoundsCheck) -> Json: ...
def from_json_bounds_check(value: Json) -> BoundsCheck: ...

@dataclass(frozen=True, slots=True)
class ShiftRangeCheck:
    """Shift amount range check over one cell."""

    # the shift amount cell offset
    value: int
    # the shifted type bit width
    bit_width: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ShiftRangeCheck: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ShiftRangeCheck: ...

def encode_shift_range_check(writer: BinaryWriter, value: ShiftRangeCheck) -> None: ...
def decode_shift_range_check(reader: BinaryReader) -> ShiftRangeCheck: ...
def to_json_shift_range_check(value: ShiftRangeCheck) -> Json: ...
def from_json_shift_range_check(value: Json) -> ShiftRangeCheck: ...

@dataclass(frozen=True, slots=True)
class NarrowCheck:
    """Integer narrowing check over one cell."""

    # the value cell offset
    value: int
    # the target bit width
    to_width: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NarrowCheck: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NarrowCheck: ...

def encode_narrow_check(writer: BinaryWriter, value: NarrowCheck) -> None: ...
def decode_narrow_check(reader: BinaryReader) -> NarrowCheck: ...
def to_json_narrow_check(value: NarrowCheck) -> Json: ...
def from_json_narrow_check(value: Json) -> NarrowCheck: ...

@dataclass(frozen=True, slots=True)
class OverflowCheck:
    """Two cell inputs for one overflow check."""

    # the left input cell offset
    left: int
    # the right input cell offset
    right: int
    # the input bit width
    width: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> OverflowCheck: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> OverflowCheck: ...

def encode_overflow_check(writer: BinaryWriter, value: OverflowCheck) -> None: ...
def decode_overflow_check(reader: BinaryReader) -> OverflowCheck: ...
def to_json_overflow_check(value: OverflowCheck) -> Json: ...
def from_json_overflow_check(value: Json) -> OverflowCheck: ...

@dataclass(frozen=True, slots=True)
class VariantCheck:
    """Variant tag check over one cell."""

    # the tag cell offset
    value: int
    # the expected tag
    expected: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCheck: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantCheck: ...

def encode_variant_check(writer: BinaryWriter, value: VariantCheck) -> None: ...
def decode_variant_check(reader: BinaryReader) -> VariantCheck: ...
def to_json_variant_check(value: VariantCheck) -> Json: ...
def from_json_variant_check(value: Json) -> VariantCheck: ...

@dataclass(frozen=True, slots=True)
class SwitchTable:
    """One pooled dense switch table."""

    # the smallest value covered by the table
    min: int
    # the table entries
    cases: Sequence[destack._generated.program.vm.function.SwitchCase]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SwitchTable: ...

def encode_switch_table(writer: BinaryWriter, value: SwitchTable) -> None: ...
def decode_switch_table(reader: BinaryReader) -> SwitchTable: ...
def to_json_switch_table(value: SwitchTable) -> Json: ...
def from_json_switch_table(value: Json) -> SwitchTable: ...

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
