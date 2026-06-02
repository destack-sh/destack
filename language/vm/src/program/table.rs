use destack_mir as mir;

use super::{
    AllocationBranch, AllocationSite, AtomicCompareExchange, Call, CallBranch, CallDynamic,
    CallDynamicBranch, CallIndirect, CallIndirectBranch, CallVirtual, CallVirtualBranch,
    ClosureBind, ConstValue, FrameSelect, Intrinsic, MoveRange, Projection, SliceAllocationBranch,
    SliceProjection, SmallAllocationSite, SwitchCase, TailCall, TailCallDynamic, TailCallIndirect,
    TailCallVirtual, TensorBinary, TensorBroadcast, TensorConcat, TensorContiguousBinary,
    TensorContiguousUnary, TensorConvert, TensorConvolution, TensorCopy, TensorDot, TensorExtract,
    TensorFill, TensorGather, TensorIndexReduce, TensorLayout, TensorLoad, TensorPad, TensorReduce,
    TensorReshape, TensorScatter, TensorSelect, TensorSlice, TensorStore, TensorTranspose,
    TensorUnary, TensorView, TensorViewCast, VectorBinary, VectorConvert, VectorExtract,
    VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat, VectorUnary,
};

/// Identifier for one pooled check constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CheckId(pub(crate) u32);

/// One lowered runtime check.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Check {
    /// Bounds check over signed index and signed length cells.
    BoundsIntInt(BoundsCheck),
    /// Bounds check over signed index and unsigned length cells.
    BoundsIntUint(BoundsCheck),
    /// Bounds check over unsigned index and signed length cells.
    BoundsUintInt(BoundsCheck),
    /// Bounds check over unsigned index and unsigned length cells.
    BoundsUintUint(BoundsCheck),
    /// Non-null check over one cell.
    Null {
        /// The value cell offset.
        value: u32,
    },
    /// Division-by-zero check over one signed cell.
    DivZeroInt { divisor: u32 },
    /// Division-by-zero check over one unsigned cell.
    DivZeroUint { divisor: u32 },
    /// Shift range check over one signed shift amount cell.
    ShiftRangeInt(ShiftRangeCheck),
    /// Shift range check over one unsigned shift amount cell.
    ShiftRangeUint(ShiftRangeCheck),
    /// Signed integer narrowing check over one cell.
    NarrowInt(NarrowCheck),
    /// Unsigned integer narrowing check over one cell.
    NarrowUint(NarrowCheck),
    /// Signed add overflow check over two cells.
    OverflowAddInt(OverflowCheck),
    /// Unsigned add overflow check over two cells.
    OverflowAddUint(OverflowCheck),
    /// Signed subtract overflow check over two cells.
    OverflowSubInt(OverflowCheck),
    /// Unsigned subtract overflow check over two cells.
    OverflowSubUint(OverflowCheck),
    /// Signed multiply overflow check over two cells.
    OverflowMulInt(OverflowCheck),
    /// Unsigned multiply overflow check over two cells.
    OverflowMulUint(OverflowCheck),
    /// Signed divide or remainder overflow check over two cells.
    OverflowDivInt(OverflowCheck),
    /// Unsigned divide or remainder overflow check over two cells.
    OverflowDivUint(OverflowCheck),
    /// Runtime type descriptor check.
    Type {
        /// The descriptor cell offset.
        value: u32,
        /// The expected type id.
        expected: u32,
    },
    /// Variant tag check.
    Variant(VariantCheck),
}

/// Bounds check over index and length cells.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BoundsCheck {
    /// The index cell offset.
    pub(crate) index: u32,
    /// The length cell offset.
    pub(crate) length: u32,
}

/// Shift amount range check over one cell.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShiftRangeCheck {
    /// The shift amount cell offset.
    pub(crate) value: u32,
    /// The shifted type bit width.
    pub(crate) bit_width: u8,
}

/// Integer narrowing check over one cell.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NarrowCheck {
    /// The value cell offset.
    pub(crate) value: u32,
    /// The target bit width.
    pub(crate) to_width: u8,
}

/// Variant tag check over one cell.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VariantCheck {
    /// The tag cell offset.
    pub(crate) value: u32,
    /// The expected tag.
    pub(crate) expected: u64,
}

/// Two cell inputs for one overflow check.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OverflowCheck {
    /// The left input cell offset.
    pub(crate) left: u32,
    /// The right input cell offset.
    pub(crate) right: u32,
    /// The input bit width.
    pub(crate) width: u8,
}

/// Identifier for one pooled switch case table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwitchCasesId(pub(crate) u32);

/// Identifier for one pooled dense switch table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwitchTableId(pub(crate) u32);

/// Identifier for one pooled control edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgeId(pub(crate) u32);

/// One lowered control-flow edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Edge {
    /// The target block.
    pub target: u32,
    /// The block-parameter moves.
    pub moves: MoveRange,
}

/// One pooled dense switch table.
#[derive(Clone, Debug)]
pub(crate) struct SwitchTable {
    /// The smallest value covered by the table.
    pub min: i128,
    /// The table entries.
    pub cases: Box<[SwitchCase]>,
}

/// Identifier for one pooled allocation site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationSiteId(pub(crate) u32);

/// Identifier for one pooled small allocation site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SmallAllocationSiteId(pub(crate) u32);

/// Identifier for one pooled constant value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConstValueId(pub(crate) u32);

/// Identifier for one pooled address projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectionId(pub(crate) u32);

/// Identifier for one pooled slice projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceProjectionId(pub(crate) u32);

/// Identifier for one pooled u32 slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct U32RangeId(pub(crate) u32);

/// Identifier for one pooled tensor dot descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorDotId(pub(crate) u32);

/// Identifier for one pooled tensor convolution dimension descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorConvolutionId(pub(crate) u32);

/// Identifier for one pooled tensor convolution window descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorWindowId(pub(crate) u32);

/// Identifier for one pooled tensor gather descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorGatherId(pub(crate) u32);

/// Identifier for one pooled tensor scatter descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorScatterId(pub(crate) u32);

/// Identifier for one pooled tensor layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorLayoutId(pub(crate) u32);

/// Record stored outside the fixed instruction cells.
pub(crate) trait SideRecord: Copy {
    /// Add one side record to the table.
    fn push(table: &mut SideTableBuilder, record: Self) -> u32;

    /// Borrow one side record from the table.
    fn get(table: &SideTable, id: u32) -> &Self;
}

macro_rules! side_record_table {
    ($( $field:ident : $ty:ty ),+ $(,)?) => {
        /// Immutable side records referenced by instruction ids.
        #[derive(Clone, Debug, Default)]
        struct SideRecordTable {
            $(
                $field: Box<[$ty]>,
            )+
        }

        /// Mutable side record table used while lowering.
        #[derive(Debug, Default)]
        struct SideRecordTableBuilder {
            $(
                $field: Vec<$ty>,
            )+
        }

        impl SideRecordTableBuilder {
            /// Finish the immutable table.
            fn finish(self) -> SideRecordTable {
                SideRecordTable {
                    $(
                        $field: self.$field.into_boxed_slice(),
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
                fn get(table: &SideTable, id: u32) -> &Self {
                    &table.record.$field[id as usize]
                }
            }
        )+
    };
}

side_record_table! {
    frame_select: FrameSelect,
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
    closure_bind: ClosureBind,
    call: Call,
    call_branch: CallBranch,
    call_virtual: CallVirtual,
    call_virtual_branch: CallVirtualBranch,
    call_dynamic: CallDynamic,
    call_dynamic_branch: CallDynamicBranch,
    call_indirect: CallIndirect,
    call_indirect_branch: CallIndirectBranch,
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
    intrinsic: Intrinsic,
    tail_call: TailCall,
    tail_call_virtual: TailCallVirtual,
    tail_call_dynamic: TailCallDynamic,
    tail_call_indirect: TailCallIndirect,
}

/// Immutable side table referenced by compact side records.
#[derive(Clone, Debug, Default)]
pub(crate) struct SideTable {
    /// Pooled side records.
    record: Box<SideRecordTable>,
    /// Pooled allocation sites.
    allocation_site: Box<[AllocationSite]>,
    /// Pooled small allocation sites.
    small_allocation_site: Box<[SmallAllocationSite]>,
    /// Pooled constants.
    constant: Box<[ConstValue]>,
    /// Pooled address projections.
    projection: Box<[Projection]>,
    /// Pooled slice projectiones.
    slice_projection: Box<[SliceProjection]>,
    /// Pooled check constraints.
    check: Box<[Check]>,
    /// Pooled switch case tables.
    switch_cases: Box<[Box<[SwitchCase]>]>,
    /// Pooled dense switch tables.
    switch_table: Box<[SwitchTable]>,
    /// Pooled control edges.
    edge: Box<[Edge]>,
    /// Pooled u32 slices.
    u32_ranges: Box<[Box<[u32]>]>,
    /// Pooled tensor dot descriptors.
    tensor_dot: Box<[mir::TensorDotDimensionNumbers]>,
    /// Pooled tensor convolution dimension descriptors.
    tensor_convolution: Box<[mir::TensorConvolutionDimensionNumbers]>,
    /// Pooled tensor convolution window descriptors.
    tensor_window: Box<[mir::TensorConvolutionWindow]>,
    /// Pooled tensor gather descriptors.
    tensor_gather: Box<[mir::TensorGatherDimensionNumbers]>,
    /// Pooled tensor scatter descriptors.
    tensor_scatter: Box<[mir::TensorScatterDimensionNumbers]>,
    /// Pooled tensor layouts.
    tensor_layout: Box<[TensorLayout]>,
}

/// Mutable side table used while lowering one program.
#[derive(Debug, Default)]
pub(crate) struct SideTableBuilder {
    /// Pooled side records.
    record: SideRecordTableBuilder,
    /// Pooled check constraints.
    check: Vec<Check>,
    /// Pooled switch case tables.
    switch_cases: Vec<Box<[SwitchCase]>>,
    /// Pooled dense switch tables.
    switch_table: Vec<SwitchTable>,
    /// Pooled control edges.
    edge: Vec<Edge>,
    /// Pooled allocation sites.
    allocation_site: Vec<AllocationSite>,
    /// Pooled small allocation sites.
    small_allocation_site: Vec<SmallAllocationSite>,
    /// Pooled constants.
    constant: Vec<ConstValue>,
    /// Pooled address projections.
    projection: Vec<Projection>,
    /// Pooled slice projectiones.
    slice_projection: Vec<SliceProjection>,
    /// Pooled u32 slices.
    u32_ranges: Vec<Box<[u32]>>,
    /// Pooled tensor dot descriptors.
    tensor_dot: Vec<mir::TensorDotDimensionNumbers>,
    /// Pooled tensor convolution dimension descriptors.
    tensor_convolution: Vec<mir::TensorConvolutionDimensionNumbers>,
    /// Pooled tensor convolution window descriptors.
    tensor_window: Vec<mir::TensorConvolutionWindow>,
    /// Pooled tensor gather descriptors.
    tensor_gather: Vec<mir::TensorGatherDimensionNumbers>,
    /// Pooled tensor scatter descriptors.
    tensor_scatter: Vec<mir::TensorScatterDimensionNumbers>,
    /// Pooled tensor layouts.
    tensor_layout: Vec<TensorLayout>,
}

impl SideTableBuilder {
    /// Finish the immutable side table.
    pub(crate) fn finish(self) -> SideTable {
        let Self {
            record,
            check,
            switch_cases,
            switch_table,
            edge,
            allocation_site,
            small_allocation_site,
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
        } = self;

        SideTable {
            record: Box::new(record.finish()),
            allocation_site: allocation_site.into_boxed_slice(),
            small_allocation_site: small_allocation_site.into_boxed_slice(),
            constant: constant.into_boxed_slice(),
            projection: projection.into_boxed_slice(),
            slice_projection: slice_projection.into_boxed_slice(),
            check: check.into_boxed_slice(),
            switch_cases: switch_cases.into_boxed_slice(),
            switch_table: switch_table.into_boxed_slice(),
            edge: edge.into_boxed_slice(),
            u32_ranges: u32_ranges.into_boxed_slice(),
            tensor_dot: tensor_dot.into_boxed_slice(),
            tensor_convolution: tensor_convolution.into_boxed_slice(),
            tensor_window: tensor_window.into_boxed_slice(),
            tensor_gather: tensor_gather.into_boxed_slice(),
            tensor_scatter: tensor_scatter.into_boxed_slice(),
            tensor_layout: tensor_layout.into_boxed_slice(),
        }
    }

    /// Add one check constraint to the side table.
    pub(crate) fn push_check(&mut self, check: Check) -> CheckId {
        let id = self.check.len() as u32;
        self.check.push(check);

        CheckId(id)
    }

    /// Add one switch case table to the side table.
    pub(crate) fn push_switch_cases(&mut self, cases: Box<[SwitchCase]>) -> SwitchCasesId {
        let id = self.switch_cases.len() as u32;
        self.switch_cases.push(cases);

        SwitchCasesId(id)
    }

    /// Add one dense switch table to the side table.
    pub(crate) fn push_switch_table(&mut self, table: SwitchTable) -> SwitchTableId {
        let id = self.switch_table.len() as u32;
        self.switch_table.push(table);

        SwitchTableId(id)
    }

    /// Add one control edge to the side table.
    pub(crate) fn push_edge(&mut self, edge: Edge) -> EdgeId {
        let id = self.edge.len() as u32;
        self.edge.push(edge);

        EdgeId(id)
    }

    /// Add one allocation site to the side table.
    pub(crate) fn push_allocation_site(&mut self, allocation: AllocationSite) -> AllocationSiteId {
        let id = self.allocation_site.len() as u32;
        self.allocation_site.push(allocation);

        AllocationSiteId(id)
    }

    /// Add one small allocation site to the side table.
    pub(crate) fn push_small_allocation_site(
        &mut self,
        allocation: SmallAllocationSite,
    ) -> SmallAllocationSiteId {
        let id = self.small_allocation_site.len() as u32;
        self.small_allocation_site.push(allocation);

        SmallAllocationSiteId(id)
    }

    /// Add one constant to the side table.
    pub(crate) fn push_constant(&mut self, constant: ConstValue) -> ConstValueId {
        let id = self.constant.len() as u32;
        self.constant.push(constant);

        ConstValueId(id)
    }

    /// Add one address projection to the side table.
    pub(crate) fn push_projection(&mut self, projection: Projection) -> ProjectionId {
        let id = self.projection.len() as u32;
        self.projection.push(projection);

        ProjectionId(id)
    }

    /// Add one slice projection to the side table.
    pub(crate) fn push_slice_projection(&mut self, access: SliceProjection) -> SliceProjectionId {
        let id = self.slice_projection.len() as u32;
        self.slice_projection.push(access);

        SliceProjectionId(id)
    }

    /// Add one u32 slice to the side table.
    pub(crate) fn push_u32_range(&mut self, values: &[u32]) -> U32RangeId {
        let id = self.u32_ranges.len() as u32;
        self.u32_ranges.push(values.into());

        U32RangeId(id)
    }

    /// Add one tensor dot descriptor to the side table.
    pub(crate) fn push_tensor_dot(
        &mut self,
        dimensions: mir::TensorDotDimensionNumbers,
    ) -> TensorDotId {
        let id = self.tensor_dot.len() as u32;
        self.tensor_dot.push(dimensions);

        TensorDotId(id)
    }

    /// Add one tensor convolution dimension descriptor to the side table.
    pub(crate) fn push_tensor_convolution(
        &mut self,
        dimensions: mir::TensorConvolutionDimensionNumbers,
    ) -> TensorConvolutionId {
        let id = self.tensor_convolution.len() as u32;
        self.tensor_convolution.push(dimensions);

        TensorConvolutionId(id)
    }

    /// Add one tensor convolution window descriptor to the side table.
    pub(crate) fn push_tensor_window(
        &mut self,
        window: mir::TensorConvolutionWindow,
    ) -> TensorWindowId {
        let id = self.tensor_window.len() as u32;
        self.tensor_window.push(window);

        TensorWindowId(id)
    }

    /// Add one tensor gather descriptor to the side table.
    pub(crate) fn push_tensor_gather(
        &mut self,
        dimensions: mir::TensorGatherDimensionNumbers,
    ) -> TensorGatherId {
        let id = self.tensor_gather.len() as u32;
        self.tensor_gather.push(dimensions);

        TensorGatherId(id)
    }

    /// Add one tensor scatter descriptor to the side table.
    pub(crate) fn push_tensor_scatter(
        &mut self,
        dimensions: mir::TensorScatterDimensionNumbers,
    ) -> TensorScatterId {
        let id = self.tensor_scatter.len() as u32;
        self.tensor_scatter.push(dimensions);

        TensorScatterId(id)
    }

    /// Add one tensor layout to the side table.
    pub(crate) fn push_tensor_layout(&mut self, layout: TensorLayout) -> TensorLayoutId {
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
}

impl SideTable {
    /// Borrow one pooled u32 slice.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &[u32] {
        &self.u32_ranges[id.0 as usize]
    }

    /// Borrow one tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> &mir::TensorDotDimensionNumbers {
        &self.tensor_dot[id.0 as usize]
    }

    /// Borrow one tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> &mir::TensorConvolutionDimensionNumbers {
        &self.tensor_convolution[id.0 as usize]
    }

    /// Borrow one tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(&self, id: TensorWindowId) -> &mir::TensorConvolutionWindow {
        &self.tensor_window[id.0 as usize]
    }

    /// Borrow one tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(&self, id: TensorGatherId) -> &mir::TensorGatherDimensionNumbers {
        &self.tensor_gather[id.0 as usize]
    }

    /// Borrow one tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(
        &self,
        id: TensorScatterId,
    ) -> &mir::TensorScatterDimensionNumbers {
        &self.tensor_scatter[id.0 as usize]
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub(crate) fn tensor_layout(&self, id: TensorLayoutId) -> &TensorLayout {
        &self.tensor_layout[id.0 as usize]
    }

    /// Borrow one pooled allocation site.
    #[inline(always)]
    pub(crate) fn allocation_site(&self, id: AllocationSiteId) -> &AllocationSite {
        &self.allocation_site[id.0 as usize]
    }

    /// Borrow one pooled small allocation site.
    #[inline(always)]
    pub(crate) fn small_allocation_site(&self, id: SmallAllocationSiteId) -> &SmallAllocationSite {
        &self.small_allocation_site[id.0 as usize]
    }

    /// Borrow one pooled constant.
    #[inline(always)]
    pub(crate) fn constant(&self, id: ConstValueId) -> &ConstValue {
        &self.constant[id.0 as usize]
    }

    /// Borrow one pooled address projection.
    #[inline(always)]
    pub(crate) fn projection(&self, id: ProjectionId) -> &Projection {
        &self.projection[id.0 as usize]
    }

    /// Borrow one pooled slice projection.
    #[inline(always)]
    pub(crate) fn slice_projection(&self, id: SliceProjectionId) -> &SliceProjection {
        &self.slice_projection[id.0 as usize]
    }

    /// Borrow one pooled check constraint.
    #[inline(always)]
    pub(crate) fn check(&self, id: CheckId) -> &Check {
        &self.check[id.0 as usize]
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &[SwitchCase] {
        &self.switch_cases[id.0 as usize]
    }

    /// Borrow one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> &SwitchTable {
        &self.switch_table[id.0 as usize]
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub(crate) fn edge(&self, id: EdgeId) -> Edge {
        self.edge[id.0 as usize]
    }
}
