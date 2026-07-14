use destack_core::{
    EntryRange, EntryStore, SectionEntry, SectionImage, SectionPacker, SectionSlice,
};
use destack_heap::{AllocationPlan, SmallAllocationPlan};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{FunctionSignature, Signature, TypeId};

use super::{
    AggregateSelect, AllocationBranch, AtomicCompareExchange, Call, CallDynamic, CallVirtual,
    ConstValue, ConstValueBuilder, Drop, FunctionBind, IndirectCall, IndirectTailCall,
    IntrinsicCall, Invoke, InvokeDynamic, InvokeIndirect, InvokeVirtual, MoveRange, Projection,
    SliceAllocationBranch, SliceProjection, SwitchCase, TailCall, TailCallDynamic, TailCallVirtual,
    TensorBinary, TensorBroadcast, TensorConcat, TensorContiguousBinary, TensorContiguousUnary,
    TensorConvert, TensorConvolution, TensorConvolutionDimensions,
    TensorConvolutionDimensionsBuilder, TensorConvolutionWindow, TensorConvolutionWindowBuilder,
    TensorCopy, TensorDot, TensorDotDimensions, TensorDotDimensionsBuilder, TensorExtract,
    TensorFill, TensorGather, TensorGatherDimensions, TensorGatherDimensionsBuilder,
    TensorIndexReduce, TensorLayout, TensorLayoutBuilder, TensorLayoutView, TensorLoad, TensorPad,
    TensorReduce, TensorReshape, TensorScatter, TensorScatterDimensions,
    TensorScatterDimensionsBuilder, TensorSelect, TensorSlice, TensorStore, TensorTranspose,
    TensorUnary, TensorView, TensorViewCast, VectorBinary, VectorConvert, VectorExtract,
    VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat, VectorUnary,
};

/// Identifier for one pooled check constraint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CheckId(pub u32);

/// One lowered runtime check.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Check {
    /// Check kind.
    pub kind: CheckKind,
    /// Bounds check payload.
    pub bounds: BoundsCheck,
    /// Shift range check payload.
    pub shift: ShiftRangeCheck,
    /// Integer narrowing check payload.
    pub narrow: NarrowCheck,
    /// Overflow check payload.
    pub overflow: OverflowCheck,
    /// Variant tag check payload.
    pub variant: VariantCheck,
    /// Single value cell offset.
    pub value: u32,
    /// Expected type id or related scalar payload.
    pub expected: u32,
}

impl Check {
    /// Create a bounds check.
    pub const fn bounds(kind: CheckKind, bounds: BoundsCheck) -> Self {
        Self {
            kind,
            bounds,
            ..Self::empty()
        }
    }

    /// Create a null check.
    pub const fn null(value: u32) -> Self {
        Self {
            kind: CheckKind::Null,
            value,
            ..Self::empty()
        }
    }

    /// Create a division-by-zero check.
    pub const fn div_zero(kind: CheckKind, divisor: u32) -> Self {
        Self {
            kind,
            value: divisor,
            ..Self::empty()
        }
    }

    /// Create a shift range check.
    pub const fn shift(kind: CheckKind, shift: ShiftRangeCheck) -> Self {
        Self {
            kind,
            shift,
            ..Self::empty()
        }
    }

    /// Create a narrowing check.
    pub const fn narrow(kind: CheckKind, narrow: NarrowCheck) -> Self {
        Self {
            kind,
            narrow,
            ..Self::empty()
        }
    }

    /// Create an overflow check.
    pub const fn overflow(kind: CheckKind, overflow: OverflowCheck) -> Self {
        Self {
            kind,
            overflow,
            ..Self::empty()
        }
    }

    /// Create a runtime type check.
    pub const fn type_id(kind: CheckKind, value: u32, expected: u32) -> Self {
        Self {
            kind,
            value,
            expected,
            ..Self::empty()
        }
    }

    /// Create a variant tag check.
    pub const fn variant(variant: VariantCheck) -> Self {
        Self {
            kind: CheckKind::Variant,
            variant,
            ..Self::empty()
        }
    }

    /// Create an empty check payload.
    const fn empty() -> Self {
        Self {
            kind: CheckKind::BoundsIntInt,
            bounds: BoundsCheck::empty(),
            shift: ShiftRangeCheck::empty(),
            narrow: NarrowCheck::empty(),
            overflow: OverflowCheck::empty(),
            variant: VariantCheck::empty(),
            value: 0,
            expected: 0,
        }
    }
}

/// Lowered runtime check kind.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CheckKind {
    /// Bounds check over signed index and signed length cells.
    #[default]
    BoundsIntInt = 0,
    /// Bounds check over signed index and unsigned length cells.
    BoundsIntUint = 1,
    /// Bounds check over unsigned index and signed length cells.
    BoundsUintInt = 2,
    /// Bounds check over unsigned index and unsigned length cells.
    BoundsUintUint = 3,
    /// Non-null check over one cell.
    Null = 4,
    /// Division-by-zero check over one signed cell.
    DivZeroInt = 5,
    /// Division-by-zero check over one unsigned cell.
    DivZeroUint = 6,
    /// Shift range check over one signed shift amount cell.
    ShiftRangeInt = 7,
    /// Shift range check over one unsigned shift amount cell.
    ShiftRangeUint = 8,
    /// Signed integer narrowing check over one cell.
    NarrowInt = 9,
    /// Unsigned integer narrowing check over one cell.
    NarrowUint = 10,
    /// Signed add overflow check over two cells.
    OverflowAddInt = 11,
    /// Unsigned add overflow check over two cells.
    OverflowAddUint = 12,
    /// Signed subtract overflow check over two cells.
    OverflowSubInt = 13,
    /// Unsigned subtract overflow check over two cells.
    OverflowSubUint = 14,
    /// Signed multiply overflow check over two cells.
    OverflowMulInt = 15,
    /// Unsigned multiply overflow check over two cells.
    OverflowMulUint = 16,
    /// Signed divide or remainder overflow check over two cells.
    OverflowDivInt = 17,
    /// Unsigned divide or remainder overflow check over two cells.
    OverflowDivUint = 18,
    /// Exact runtime type-id check.
    TypeId = 19,
    /// Runtime subtype check over one type-id cell.
    SubtypeId = 20,
    /// Variant tag check.
    Variant = 21,
}

/// Bounds check over index and length cells.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BoundsCheck {
    /// The index cell offset.
    pub index: u32,
    /// The length cell offset.
    pub length: u32,
}

impl BoundsCheck {
    /// Return an empty bounds check payload.
    pub const fn empty() -> Self {
        Self {
            index: 0,
            length: 0,
        }
    }
}

/// Shift amount range check over one cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ShiftRangeCheck {
    /// The shift amount cell offset.
    pub value: u32,
    /// The shifted type bit width.
    pub bit_width: u8,
}

impl ShiftRangeCheck {
    /// Return an empty shift range check payload.
    pub const fn empty() -> Self {
        Self {
            value: 0,
            bit_width: 0,
        }
    }
}

/// Integer narrowing check over one cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct NarrowCheck {
    /// The value cell offset.
    pub value: u32,
    /// The target bit width.
    pub to_width: u8,
}

impl NarrowCheck {
    /// Return an empty narrowing check payload.
    pub const fn empty() -> Self {
        Self {
            value: 0,
            to_width: 0,
        }
    }
}

/// Variant tag check over one cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VariantCheck {
    /// The tag cell offset.
    pub value: u32,
    /// The expected tag.
    pub expected: u64,
}

impl VariantCheck {
    /// Return an empty variant check payload.
    pub const fn empty() -> Self {
        Self {
            value: 0,
            expected: 0,
        }
    }
}

/// Two cell inputs for one overflow check.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct OverflowCheck {
    /// The left input cell offset.
    pub left: u32,
    /// The right input cell offset.
    pub right: u32,
    /// The input bit width.
    pub width: u8,
}

impl OverflowCheck {
    /// Return an empty overflow check payload.
    pub const fn empty() -> Self {
        Self {
            left: 0,
            right: 0,
            width: 0,
        }
    }
}

/// Identifier for one pooled switch case table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SwitchCasesId(pub u32);

/// Identifier for one pooled dense switch table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SwitchTableId(pub u32);

/// Identifier for one pooled control edge.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct EdgeId(pub u32);

/// One lowered control-flow edge.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Edge {
    /// The target block.
    pub target: u32,
    /// The block-parameter moves.
    pub moves: MoveRange,
}

/// One pooled dense switch table.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SwitchTable {
    /// The smallest value covered by the table.
    pub min: i128,
    /// The table entries.
    pub cases: EntryRange<SwitchCase>,
}

/// Build-time dense switch table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SwitchTableBuilder {
    /// The smallest value covered by the table.
    pub min: i128,
    /// The table entries.
    pub cases: Box<[SwitchCase]>,
}

/// Identifier for one pooled allocation plan.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AllocationPlanId(pub u32);

/// Identifier for one pooled small allocation plan.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SmallAllocationPlanId(pub u32);

/// Identifier for one pooled constant value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ConstValueId(pub u32);

/// Identifier for one pooled address projection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProjectionId(pub u32);

/// Identifier for one pooled slice projection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceProjectionId(pub u32);

/// Identifier for one pooled u32 slice.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct U32RangeId(pub u32);

/// Identifier for one pooled tensor dot descriptor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorDotId(pub u32);

/// Identifier for one pooled tensor convolution dimension descriptor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorConvolutionId(pub u32);

/// Identifier for one pooled tensor convolution window descriptor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorWindowId(pub u32);

/// Identifier for one pooled tensor gather descriptor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorGatherId(pub u32);

/// Identifier for one pooled tensor scatter descriptor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorScatterId(pub u32);

/// Identifier for one pooled tensor layout.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TensorLayoutId(pub u32);

/// Identifier for one pooled callable signature.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SignatureId(pub u32);

/// Record stored outside the fixed instruction cells.
pub trait SideRecord: Copy + SectionEntry {
    /// Add one side record to the table.
    fn push(table: &mut SideTableBuilder, record: Self) -> u32;

    /// Borrow one side record from the table.
    fn get<'a>(table: &SideTable, sections: SectionImage<'a>, id: u32) -> &'a Self;
}

macro_rules! side_record_table {
    ($( $field:ident : $ty:ty ),+ $(,)?) => {
        /// Immutable side records referenced by instruction ids.
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
        struct SideRecordTable {
            $(
                $field: SectionSlice<$ty>,
            )+
        }

        /// Mutable side record table used while lowering.
        #[derive(Debug, Default, Serialize, Deserialize, Reflect)]
        struct SideRecordTableBuilder {
            $(
                $field: Vec<$ty>,
            )+
        }

        impl SideRecordTableBuilder {
            /// Pack the immutable table.
            fn pack(self, sections: &mut SectionPacker) -> SideRecordTable {
                SideRecordTable {
                    $(
                        $field: sections.insert(self.$field),
                    )+
                }
            }
        }

        $(
            impl SideRecord for $ty {
                #[inline]
                fn push(table: &mut SideTableBuilder, record: Self) -> u32 {
                    let id = table.record.$field.len() as u32;
                    table.record.$field.push(record);

                    id
                }

                #[inline(always)]
                fn get<'a>(table: &SideTable, sections: SectionImage<'a>, id: u32) -> &'a Self {
                    &sections.entries(table.record.$field)[id as usize]
                }
            }

            // SAFETY: side records are fixed-width VM entries.
            unsafe impl SectionEntry for $ty {}
        )+
    };
}

side_record_table! {
    aggregate_select: AggregateSelect,
    allocation_branch: AllocationBranch,
    slice_allocation_branch: SliceAllocationBranch,
    atomic_compare_exchange: AtomicCompareExchange,
    vector_splat: VectorSplat,
    vector_extract: VectorExtract,
    vector_binary: VectorBinary,
    vector_unary: VectorUnary,
    vector_insert: VectorInsert,
    vector_shuffle: VectorShuffle,
    vector_select: VectorSelect,
    vector_reduce: VectorReduce,
    vector_convert: VectorConvert,
    function_bind: FunctionBind,
    drop: Drop,
    call: Call,
    invoke: Invoke,
    call_virtual: CallVirtual,
    invoke_virtual: InvokeVirtual,
    call_dynamic: CallDynamic,
    invoke_dynamic: InvokeDynamic,
    indirect_call: IndirectCall,
    invoke_indirect: InvokeIndirect,
    tensor_load: TensorLoad,
    tensor_extract: TensorExtract,
    tensor_binary: TensorBinary,
    tensor_contiguous_binary: TensorContiguousBinary,
    tensor_unary: TensorUnary,
    tensor_contiguous_unary: TensorContiguousUnary,
    tensor_store: TensorStore,
    tensor_fill: TensorFill,
    tensor_copy: TensorCopy,
    tensor_view_cast: TensorViewCast,
    tensor_reshape: TensorReshape,
    tensor_broadcast: TensorBroadcast,
    tensor_transpose: TensorTranspose,
    tensor_slice: TensorSlice,
    tensor_pad: TensorPad,
    tensor_concat: TensorConcat,
    tensor_reduce: TensorReduce,
    tensor_index_reduce: TensorIndexReduce,
    tensor_dot: TensorDot,
    tensor_convolution: TensorConvolution,
    tensor_gather: TensorGather,
    tensor_scatter: TensorScatter,
    tensor_select: TensorSelect,
    tensor_convert: TensorConvert,
    tensor_view: TensorView,
    intrinsic: IntrinsicCall,
    tail_call: TailCall,
    tail_call_virtual: TailCallVirtual,
    tail_call_dynamic: TailCallDynamic,
    indirect_tail_call: IndirectTailCall,
}

/// Immutable side table referenced by compact side records.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SideTable {
    /// Pooled side records.
    record: SideRecordTable,
    /// Pooled allocation plans.
    allocation_plan: SectionSlice<AllocationPlan>,
    /// Pooled small allocation plans.
    small_allocation_plan: SectionSlice<SmallAllocationPlan>,
    /// Pooled constants.
    constant: SectionSlice<ConstValue>,
    /// Flattened constant bytes.
    constant_bytes: SectionSlice<u8>,
    /// Pooled address projections.
    projection: SectionSlice<Projection>,
    /// Pooled slice projections.
    slice_projection: SectionSlice<SliceProjection>,
    /// Pooled check constraints.
    check: SectionSlice<Check>,
    /// Pooled switch case tables.
    switch_cases: SectionSlice<EntryRange<SwitchCase>>,
    /// Flattened switch case entries.
    switch_case_entries: SectionSlice<SwitchCase>,
    /// Pooled dense switch tables.
    switch_table: SectionSlice<SwitchTable>,
    /// Pooled control edges.
    edge: SectionSlice<Edge>,
    /// Pooled u32 slices.
    u32_ranges: SectionSlice<EntryRange<u32>>,
    /// Flattened u32 entries.
    u32_entries: SectionSlice<u32>,
    /// Flattened tensor u64 entries.
    tensor_u64_entries: SectionSlice<u64>,
    /// Flattened tensor flag entries.
    tensor_flag_entries: SectionSlice<u8>,
    /// Pooled tensor dot descriptors.
    tensor_dot: SectionSlice<TensorDotDimensions>,
    /// Pooled tensor convolution dimension descriptors.
    tensor_convolution: SectionSlice<TensorConvolutionDimensions>,
    /// Pooled tensor convolution window descriptors.
    tensor_window: SectionSlice<TensorConvolutionWindow>,
    /// Pooled tensor gather descriptors.
    tensor_gather: SectionSlice<TensorGatherDimensions>,
    /// Pooled tensor scatter descriptors.
    tensor_scatter: SectionSlice<TensorScatterDimensions>,
    /// Pooled tensor layouts.
    tensor_layout: SectionSlice<TensorLayout>,
    /// Pooled callable signatures.
    signature: SectionSlice<FunctionSignature>,
    /// Flattened callable signature parameters.
    signature_parameters: SectionSlice<TypeId>,
}

/// Mutable side table used while lowering one program.
#[derive(Debug, Default, Serialize, Deserialize, Reflect)]
pub struct SideTableBuilder {
    /// Pooled side records.
    record: SideRecordTableBuilder,
    /// Pooled check constraints.
    check: Vec<Check>,
    /// Pooled switch case tables.
    switch_cases: Vec<Box<[SwitchCase]>>,
    /// Pooled dense switch tables.
    switch_table: Vec<SwitchTableBuilder>,
    /// Pooled control edges.
    edge: Vec<Edge>,
    /// Pooled allocation plans.
    allocation_plan: Vec<AllocationPlan>,
    /// Pooled small allocation plans.
    small_allocation_plan: Vec<SmallAllocationPlan>,
    /// Pooled constants.
    constant: Vec<ConstValueBuilder>,
    /// Pooled address projections.
    projection: Vec<Projection>,
    /// Pooled slice projections.
    slice_projection: Vec<SliceProjection>,
    /// Pooled u32 slices.
    u32_ranges: Vec<Box<[u32]>>,
    /// Pooled tensor dot descriptors.
    tensor_dot: Vec<TensorDotDimensionsBuilder>,
    /// Pooled tensor convolution dimension descriptors.
    tensor_convolution: Vec<TensorConvolutionDimensionsBuilder>,
    /// Pooled tensor convolution window descriptors.
    tensor_window: Vec<TensorConvolutionWindowBuilder>,
    /// Pooled tensor gather descriptors.
    tensor_gather: Vec<TensorGatherDimensionsBuilder>,
    /// Pooled tensor scatter descriptors.
    tensor_scatter: Vec<TensorScatterDimensionsBuilder>,
    /// Pooled tensor layouts.
    tensor_layout: Vec<TensorLayoutBuilder>,
    /// Pooled callable signatures.
    signature: Vec<Signature>,
}

impl SideTableBuilder {
    /// Pack the immutable side table.
    pub fn pack(self, sections: &mut SectionPacker) -> SideTable {
        let Self {
            record,
            check,
            switch_cases,
            switch_table,
            edge,
            allocation_plan,
            small_allocation_plan,
            constant,
            projection,
            slice_projection,
            u32_ranges,
            tensor_dot,
            tensor_convolution,
            tensor_window,
            tensor_gather,
            tensor_scatter,
            tensor_layout,
            signature,
        } = self;

        let mut constant_bytes = EntryStore::new();
        let mut switch_case_entries = EntryStore::new();
        let mut u32_entries = EntryStore::new();
        let mut tensor_u64_entries = EntryStore::new();
        let mut tensor_flag_entries = EntryStore::new();
        let mut signature_parameters = EntryStore::new();

        let constant = constant
            .into_iter()
            .map(|constant| constant.build(&mut constant_bytes))
            .collect::<Vec<_>>();
        let switch_cases = switch_cases
            .into_iter()
            .map(|cases| switch_case_entries.append(cases))
            .collect::<Vec<_>>();
        let switch_table = switch_table
            .into_iter()
            .map(|table| table.build(&mut switch_case_entries))
            .collect::<Vec<_>>();
        let u32_ranges = u32_ranges
            .into_iter()
            .map(|values| u32_entries.append(values))
            .collect::<Vec<_>>();
        let tensor_dot = tensor_dot
            .into_iter()
            .map(|dimensions| dimensions.build(&mut u32_entries))
            .collect::<Vec<_>>();
        let tensor_convolution = tensor_convolution
            .into_iter()
            .map(|dimensions| dimensions.build(&mut u32_entries))
            .collect::<Vec<_>>();
        let tensor_window = tensor_window
            .into_iter()
            .map(|window| window.build(&mut tensor_u64_entries, &mut tensor_flag_entries))
            .collect::<Vec<_>>();
        let tensor_gather = tensor_gather
            .into_iter()
            .map(|dimensions| dimensions.build(&mut u32_entries))
            .collect::<Vec<_>>();
        let tensor_scatter = tensor_scatter
            .into_iter()
            .map(|dimensions| dimensions.build(&mut u32_entries))
            .collect::<Vec<_>>();
        let tensor_layout = tensor_layout
            .into_iter()
            .map(|layout| layout.build(&mut tensor_u64_entries))
            .collect::<Vec<_>>();
        let signature = signature
            .into_iter()
            .map(|signature| FunctionSignature {
                parameters: signature_parameters.append(signature.parameters),
                result: signature.result,
            })
            .collect::<Vec<_>>();

        SideTable {
            record: record.pack(sections),
            allocation_plan: sections.insert(allocation_plan),
            small_allocation_plan: sections.insert(small_allocation_plan),
            constant: sections.insert(constant),
            constant_bytes: sections.insert(constant_bytes.into_entries()),
            projection: sections.insert(projection),
            slice_projection: sections.insert(slice_projection),
            check: sections.insert(check),
            switch_cases: sections.insert(switch_cases),
            switch_case_entries: sections.insert(switch_case_entries.into_entries()),
            switch_table: sections.insert(switch_table),
            edge: sections.insert(edge),
            u32_ranges: sections.insert(u32_ranges),
            u32_entries: sections.insert(u32_entries.into_entries()),
            tensor_u64_entries: sections.insert(tensor_u64_entries.into_entries()),
            tensor_flag_entries: sections.insert(tensor_flag_entries.into_entries()),
            tensor_dot: sections.insert(tensor_dot),
            tensor_convolution: sections.insert(tensor_convolution),
            tensor_window: sections.insert(tensor_window),
            tensor_gather: sections.insert(tensor_gather),
            tensor_scatter: sections.insert(tensor_scatter),
            tensor_layout: sections.insert(tensor_layout),
            signature: sections.insert(signature),
            signature_parameters: sections.insert(signature_parameters.into_entries()),
        }
    }

    /// Add one check constraint to the side table.
    pub fn push_check(&mut self, check: Check) -> CheckId {
        let id = self.check.len() as u32;
        self.check.push(check);

        CheckId(id)
    }

    /// Add one switch case table to the side table.
    pub fn push_switch_cases(&mut self, cases: Box<[SwitchCase]>) -> SwitchCasesId {
        let id = self.switch_cases.len() as u32;
        self.switch_cases.push(cases);

        SwitchCasesId(id)
    }

    /// Add one dense switch table to the side table.
    pub fn push_switch_table(&mut self, table: SwitchTableBuilder) -> SwitchTableId {
        let id = self.switch_table.len() as u32;
        self.switch_table.push(table);

        SwitchTableId(id)
    }

    /// Add one control edge to the side table.
    pub fn push_edge(&mut self, edge: Edge) -> EdgeId {
        let id = self.edge.len() as u32;
        self.edge.push(edge);

        EdgeId(id)
    }

    /// Add one allocation plan to the side table.
    pub fn push_allocation_plan(&mut self, allocation: AllocationPlan) -> AllocationPlanId {
        let id = self.allocation_plan.len() as u32;
        self.allocation_plan.push(allocation);

        AllocationPlanId(id)
    }

    /// Add one small allocation plan to the side table.
    pub fn push_small_allocation_plan(
        &mut self,
        allocation: SmallAllocationPlan,
    ) -> SmallAllocationPlanId {
        let id = self.small_allocation_plan.len() as u32;
        self.small_allocation_plan.push(allocation);

        SmallAllocationPlanId(id)
    }

    /// Add one constant to the side table.
    pub fn push_constant(&mut self, constant: ConstValueBuilder) -> ConstValueId {
        let id = self.constant.len() as u32;
        self.constant.push(constant);

        ConstValueId(id)
    }

    /// Add one address projection to the side table.
    pub fn push_projection(&mut self, projection: Projection) -> ProjectionId {
        let id = self.projection.len() as u32;
        self.projection.push(projection);

        ProjectionId(id)
    }

    /// Add one slice projection to the side table.
    pub fn push_slice_projection(&mut self, access: SliceProjection) -> SliceProjectionId {
        let id = self.slice_projection.len() as u32;
        self.slice_projection.push(access);

        SliceProjectionId(id)
    }

    /// Add one u32 slice to the side table.
    pub fn push_u32_range(&mut self, values: &[u32]) -> U32RangeId {
        let id = self.u32_ranges.len() as u32;
        self.u32_ranges.push(values.into());

        U32RangeId(id)
    }

    /// Add one tensor dot descriptor to the side table.
    pub fn push_tensor_dot(&mut self, dimensions: TensorDotDimensionsBuilder) -> TensorDotId {
        let id = self.tensor_dot.len() as u32;
        self.tensor_dot.push(dimensions);

        TensorDotId(id)
    }

    /// Add one tensor convolution dimension descriptor to the side table.
    pub fn push_tensor_convolution(
        &mut self,
        dimensions: TensorConvolutionDimensionsBuilder,
    ) -> TensorConvolutionId {
        let id = self.tensor_convolution.len() as u32;
        self.tensor_convolution.push(dimensions);

        TensorConvolutionId(id)
    }

    /// Add one tensor convolution window descriptor to the side table.
    pub fn push_tensor_window(&mut self, window: TensorConvolutionWindowBuilder) -> TensorWindowId {
        let id = self.tensor_window.len() as u32;
        self.tensor_window.push(window);

        TensorWindowId(id)
    }

    /// Add one tensor gather descriptor to the side table.
    pub fn push_tensor_gather(
        &mut self,
        dimensions: TensorGatherDimensionsBuilder,
    ) -> TensorGatherId {
        let id = self.tensor_gather.len() as u32;
        self.tensor_gather.push(dimensions);

        TensorGatherId(id)
    }

    /// Add one tensor scatter descriptor to the side table.
    pub fn push_tensor_scatter(
        &mut self,
        dimensions: TensorScatterDimensionsBuilder,
    ) -> TensorScatterId {
        let id = self.tensor_scatter.len() as u32;
        self.tensor_scatter.push(dimensions);

        TensorScatterId(id)
    }

    /// Add one tensor layout to the side table.
    pub fn push_tensor_layout(&mut self, layout: TensorLayoutBuilder) -> TensorLayoutId {
        if let Some(id) = self
            .tensor_layout
            .iter()
            .position(|existing| *existing == layout)
        {
            return TensorLayoutId(id as u32);
        }

        let id = self.tensor_layout.len() as u32;
        self.tensor_layout.push(layout);

        TensorLayoutId(id)
    }

    /// Add one callable signature to the side table.
    pub fn push_signature(&mut self, signature: Signature) -> SignatureId {
        if let Some(id) = self
            .signature
            .iter()
            .position(|existing| *existing == signature)
        {
            return SignatureId(id as u32);
        }

        let id = self.signature.len() as u32;
        self.signature.push(signature);

        SignatureId(id)
    }
}

impl SideTable {
    /// Borrow one pooled u32 slice.
    #[inline(always)]
    pub fn u32_range<'a>(&self, sections: SectionImage<'a>, id: U32RangeId) -> &'a [u32] {
        let range = sections.entries(self.u32_ranges)[id.0 as usize];

        sections.range(self.u32_entries, range)
    }

    /// Borrow one tensor u32 entry range.
    #[inline(always)]
    pub fn tensor_u32_range<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<u32>,
    ) -> &'a [u32] {
        sections.range(self.u32_entries, range)
    }

    /// Borrow one tensor u64 entry range.
    #[inline(always)]
    pub fn tensor_u64_range<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<u64>,
    ) -> &'a [u64] {
        sections.range(self.tensor_u64_entries, range)
    }

    /// Borrow one tensor flag entry range.
    #[inline(always)]
    pub fn tensor_flag_range<'a>(
        &self,
        sections: SectionImage<'a>,
        range: EntryRange<u8>,
    ) -> &'a [u8] {
        sections.range(self.tensor_flag_entries, range)
    }

    /// Return one tensor dot descriptor.
    #[inline(always)]
    pub fn tensor_dot(&self, sections: SectionImage<'_>, id: TensorDotId) -> TensorDotDimensions {
        sections.entries(self.tensor_dot)[id.0 as usize]
    }

    /// Return one tensor convolution dimension descriptor.
    #[inline(always)]
    pub fn tensor_convolution(
        &self,
        sections: SectionImage<'_>,
        id: TensorConvolutionId,
    ) -> TensorConvolutionDimensions {
        sections.entries(self.tensor_convolution)[id.0 as usize]
    }

    /// Return one tensor convolution window descriptor.
    #[inline(always)]
    pub fn tensor_window(
        &self,
        sections: SectionImage<'_>,
        id: TensorWindowId,
    ) -> TensorConvolutionWindow {
        sections.entries(self.tensor_window)[id.0 as usize]
    }

    /// Return one tensor gather descriptor.
    #[inline(always)]
    pub fn tensor_gather(
        &self,
        sections: SectionImage<'_>,
        id: TensorGatherId,
    ) -> TensorGatherDimensions {
        sections.entries(self.tensor_gather)[id.0 as usize]
    }

    /// Return one tensor scatter descriptor.
    #[inline(always)]
    pub fn tensor_scatter(
        &self,
        sections: SectionImage<'_>,
        id: TensorScatterId,
    ) -> TensorScatterDimensions {
        sections.entries(self.tensor_scatter)[id.0 as usize]
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub fn tensor_layout<'a>(
        &self,
        sections: SectionImage<'a>,
        id: TensorLayoutId,
    ) -> TensorLayoutView<'a> {
        let layout = sections.entries(self.tensor_layout)[id.0 as usize];

        TensorLayoutView {
            entry: layout,
            shape: sections.range(self.tensor_u64_entries, layout.shape),
            strides: sections.range(self.tensor_u64_entries, layout.strides),
        }
    }

    /// Return one pooled callable signature.
    #[inline(always)]
    pub fn signature(&self, sections: SectionImage<'_>, id: SignatureId) -> FunctionSignature {
        sections.entries(self.signature)[id.0 as usize]
    }

    /// Borrow one pooled callable signature parameter slice.
    #[inline(always)]
    pub fn signature_parameters<'a>(
        &self,
        sections: SectionImage<'a>,
        signature: FunctionSignature,
    ) -> &'a [TypeId] {
        sections.range(self.signature_parameters, signature.parameters)
    }

    /// Return one pooled allocation plan.
    #[inline(always)]
    pub fn allocation_plan(
        &self,
        sections: SectionImage<'_>,
        id: AllocationPlanId,
    ) -> AllocationPlan {
        sections.entries(self.allocation_plan)[id.0 as usize]
    }

    /// Return one pooled small allocation plan.
    #[inline(always)]
    pub fn small_allocation_plan(
        &self,
        sections: SectionImage<'_>,
        id: SmallAllocationPlanId,
    ) -> SmallAllocationPlan {
        sections.entries(self.small_allocation_plan)[id.0 as usize]
    }

    /// Return one pooled constant.
    #[inline(always)]
    pub fn constant(&self, sections: SectionImage<'_>, id: ConstValueId) -> ConstValue {
        sections.entries(self.constant)[id.0 as usize]
    }

    /// Borrow one pooled constant byte range.
    #[inline(always)]
    pub fn constant_bytes<'a>(&self, sections: SectionImage<'a>, constant: ConstValue) -> &'a [u8] {
        sections.range(self.constant_bytes, constant.bytes)
    }

    /// Return one pooled address projection.
    #[inline(always)]
    pub fn projection(&self, sections: SectionImage<'_>, id: ProjectionId) -> Projection {
        sections.entries(self.projection)[id.0 as usize]
    }

    /// Return one pooled slice projection.
    #[inline(always)]
    pub fn slice_projection(
        &self,
        sections: SectionImage<'_>,
        id: SliceProjectionId,
    ) -> SliceProjection {
        sections.entries(self.slice_projection)[id.0 as usize]
    }

    /// Return one pooled check constraint.
    #[inline(always)]
    pub fn check(&self, sections: SectionImage<'_>, id: CheckId) -> Check {
        sections.entries(self.check)[id.0 as usize]
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub fn switch_cases<'a>(
        &self,
        sections: SectionImage<'a>,
        id: SwitchCasesId,
    ) -> &'a [SwitchCase] {
        let range = sections.entries(self.switch_cases)[id.0 as usize];

        sections.range(self.switch_case_entries, range)
    }

    /// Return one pooled dense switch table.
    #[inline(always)]
    pub fn switch_table(&self, sections: SectionImage<'_>, id: SwitchTableId) -> SwitchTable {
        sections.entries(self.switch_table)[id.0 as usize]
    }

    /// Borrow one pooled dense switch table's cases.
    #[inline(always)]
    pub fn switch_table_cases<'a>(
        &self,
        sections: SectionImage<'a>,
        table: SwitchTable,
    ) -> &'a [SwitchCase] {
        sections.range(self.switch_case_entries, table.cases)
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub fn edge(&self, sections: SectionImage<'_>, id: EdgeId) -> Edge {
        sections.entries(self.edge)[id.0 as usize]
    }
}

impl SwitchTableBuilder {
    /// Build this switch table into one section entry.
    fn build(self, cases: &mut EntryStore<SwitchCase>) -> SwitchTable {
        SwitchTable {
            min: self.min,
            cases: cases.append(self.cases),
        }
    }
}

// SAFETY: side-table ids are fixed-width VM entry scalars.
unsafe impl SectionEntry for CheckId {}
unsafe impl SectionEntry for SwitchCasesId {}
unsafe impl SectionEntry for SwitchTableId {}
unsafe impl SectionEntry for EdgeId {}
unsafe impl SectionEntry for AllocationPlanId {}
unsafe impl SectionEntry for SmallAllocationPlanId {}
unsafe impl SectionEntry for ConstValueId {}
unsafe impl SectionEntry for ProjectionId {}
unsafe impl SectionEntry for SliceProjectionId {}
unsafe impl SectionEntry for U32RangeId {}
unsafe impl SectionEntry for TensorDotId {}
unsafe impl SectionEntry for TensorConvolutionId {}
unsafe impl SectionEntry for TensorWindowId {}
unsafe impl SectionEntry for TensorGatherId {}
unsafe impl SectionEntry for TensorScatterId {}
unsafe impl SectionEntry for TensorLayoutId {}
unsafe impl SectionEntry for SignatureId {}

// SAFETY: side-table entries contain only fixed-width VM entry values.
unsafe impl SectionEntry for Check {}
unsafe impl SectionEntry for CheckKind {}
unsafe impl SectionEntry for BoundsCheck {}
unsafe impl SectionEntry for ShiftRangeCheck {}
unsafe impl SectionEntry for NarrowCheck {}
unsafe impl SectionEntry for VariantCheck {}
unsafe impl SectionEntry for OverflowCheck {}
unsafe impl SectionEntry for Edge {}
unsafe impl SectionEntry for SwitchTable {}
