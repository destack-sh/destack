use {destack_heap as heap, destack_mir as mir};

use super::{
    AllocationLayout, Call, CallBranch, CallIndirect, CallIndirectBranch, CallInterface,
    CallInterfaceBranch, CallVirtual, CallVirtualBranch, ConstValue, ElementAccess, FieldAccess,
    FrameAccess, Intrinsic, MoveRange, PointeeAccess, SliceElementAccess, SwitchCase, TailCall,
    TailCallIndirect, TailCallInterface, TailCallVirtual, TensorBroadcast, TensorCompare,
    TensorConcat, TensorConvert, TensorConvolution, TensorCopy, TensorDot, TensorExtract,
    TensorFill, TensorGather, TensorLoad, TensorPad, TensorReduce, TensorReshape, TensorScatter,
    TensorSelect, TensorSlice, TensorStore, TensorTranspose, TensorView,
};

/// Identifier for one pooled check constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CheckId(pub(crate) u32);

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

/// Identifier for one pooled allocation layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationLayoutId(pub(crate) u32);

/// Identifier for one pooled constant value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConstValueId(pub(crate) u32);

/// Identifier for one pooled allocation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationClassId(pub(crate) u32);

/// Identifier for one pooled small allocation layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SmallAllocationLayoutId(pub(crate) u32);

/// Identifier for one pooled reference map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReferenceMapId(pub(crate) u32);

/// Identifier for one pooled field access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAccessId(pub(crate) u32);

/// Identifier for one pooled frame access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameAccessId(pub(crate) u32);

/// Identifier for one pooled element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementAccessId(pub(crate) u32);

/// Identifier for one pooled slice element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceElementAccessId(pub(crate) u32);

/// Identifier for one pooled pointee access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointeeAccessId(pub(crate) u32);

/// Identifier for one pooled u32 slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct U32RangeId(pub(crate) u32);

/// Identifier for one pooled MIR type slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TypeRangeId(pub(crate) u32);

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

/// Record stored outside the fixed instruction words.
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
    call: Call,
    call_branch: CallBranch,
    call_virtual: CallVirtual,
    call_virtual_branch: CallVirtualBranch,
    call_interface: CallInterface,
    call_interface_branch: CallInterfaceBranch,
    call_indirect: CallIndirect,
    call_indirect_branch: CallIndirectBranch,
    tensor_load: TensorLoad,
    tensor_extract: TensorExtract,
    tensor_store: TensorStore,
    tensor_fill: TensorFill,
    tensor_copy: TensorCopy,
    tensor_reshape: TensorReshape,
    tensor_broadcast: TensorBroadcast,
    tensor_transpose: TensorTranspose,
    tensor_slice: TensorSlice,
    tensor_pad: TensorPad,
    tensor_concat: TensorConcat,
    tensor_reduce: TensorReduce,
    tensor_dot: TensorDot,
    tensor_convolution: TensorConvolution,
    tensor_gather: TensorGather,
    tensor_scatter: TensorScatter,
    tensor_compare: TensorCompare,
    tensor_select: TensorSelect,
    tensor_convert: TensorConvert,
    tensor_view: TensorView,
    intrinsic: Intrinsic,
    tail_call: TailCall,
    tail_call_virtual: TailCallVirtual,
    tail_call_interface: TailCallInterface,
    tail_call_indirect: TailCallIndirect,
}

/// Immutable side table referenced by compact side records.
#[derive(Clone, Debug, Default)]
pub(crate) struct SideTable {
    /// Pooled side records.
    record: Box<SideRecordTable>,
    /// Pooled allocation layouts.
    allocation_layout: Box<[AllocationLayout]>,
    /// Pooled constants.
    constant: Box<[ConstValue]>,
    /// Pooled allocation classes.
    allocation_class: Box<[heap::AllocationClass]>,
    /// Pooled small allocation layouts.
    small_allocation_layout: Box<[heap::SmallAllocationLayout]>,
    /// Pooled reference maps.
    reference_map: Box<[mir::ReferenceMap]>,
    /// Pooled field accesses.
    field_access: Box<[FieldAccess]>,
    /// Pooled frame accesses.
    frame_access: Box<[FrameAccess]>,
    /// Pooled element accesses.
    element_access: Box<[ElementAccess]>,
    /// Pooled slice element accesses.
    slice_element_access: Box<[SliceElementAccess]>,
    /// Pooled pointee accesses.
    pointee_access: Box<[PointeeAccess]>,
    /// Pooled check constraints.
    check: Box<[mir::CheckConstraint]>,
    /// Pooled switch case tables.
    switch_cases: Box<[Box<[SwitchCase]>]>,
    /// Pooled dense switch tables.
    switch_table: Box<[SwitchTable]>,
    /// Pooled control edges.
    edge: Box<[Edge]>,
    /// Pooled u32 slices.
    u32_ranges: Box<[Box<[u32]>]>,
    /// Pooled MIR type slices.
    type_ranges: Box<[Box<[mir::LocalNodeId<mir::Type>]>]>,
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
}

/// Mutable side table used while lowering one program.
#[derive(Debug, Default)]
pub(crate) struct SideTableBuilder {
    /// Pooled side records.
    record: SideRecordTableBuilder,
    /// Pooled check constraints.
    check: Vec<mir::CheckConstraint>,
    /// Pooled switch case tables.
    switch_cases: Vec<Box<[SwitchCase]>>,
    /// Pooled dense switch tables.
    switch_table: Vec<SwitchTable>,
    /// Pooled control edges.
    edge: Vec<Edge>,
    /// Pooled allocation layouts.
    allocation_layout: Vec<AllocationLayout>,
    /// Pooled constants.
    constant: Vec<ConstValue>,
    /// Pooled allocation classes.
    allocation_class: Vec<heap::AllocationClass>,
    /// Pooled small allocation layouts.
    small_allocation_layout: Vec<heap::SmallAllocationLayout>,
    /// Pooled reference maps.
    reference_map: Vec<mir::ReferenceMap>,
    /// Pooled field accesses.
    field_access: Vec<FieldAccess>,
    /// Pooled frame accesses.
    frame_access: Vec<FrameAccess>,
    /// Pooled element accesses.
    element_access: Vec<ElementAccess>,
    /// Pooled slice element accesses.
    slice_element_access: Vec<SliceElementAccess>,
    /// Pooled pointee accesses.
    pointee_access: Vec<PointeeAccess>,
    /// Pooled u32 slices.
    u32_ranges: Vec<Box<[u32]>>,
    /// Pooled MIR type slices.
    type_ranges: Vec<Box<[mir::LocalNodeId<mir::Type>]>>,
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
            allocation_layout,
            constant,
            allocation_class,
            small_allocation_layout,
            reference_map,
            field_access,
            frame_access,
            element_access,
            slice_element_access,
            pointee_access,
            u32_ranges,
            type_ranges,
            tensor_dot,
            tensor_convolution,
            tensor_window,
            tensor_gather,
            tensor_scatter,
        } = self;

        SideTable {
            record: Box::new(record.finish()),
            allocation_layout: allocation_layout.into_boxed_slice(),
            constant: constant.into_boxed_slice(),
            allocation_class: allocation_class.into_boxed_slice(),
            small_allocation_layout: small_allocation_layout.into_boxed_slice(),
            reference_map: reference_map.into_boxed_slice(),
            field_access: field_access.into_boxed_slice(),
            frame_access: frame_access.into_boxed_slice(),
            element_access: element_access.into_boxed_slice(),
            slice_element_access: slice_element_access.into_boxed_slice(),
            pointee_access: pointee_access.into_boxed_slice(),
            check: check.into_boxed_slice(),
            switch_cases: switch_cases.into_boxed_slice(),
            switch_table: switch_table.into_boxed_slice(),
            edge: edge.into_boxed_slice(),
            u32_ranges: u32_ranges.into_boxed_slice(),
            type_ranges: type_ranges.into_boxed_slice(),
            tensor_dot: tensor_dot.into_boxed_slice(),
            tensor_convolution: tensor_convolution.into_boxed_slice(),
            tensor_window: tensor_window.into_boxed_slice(),
            tensor_gather: tensor_gather.into_boxed_slice(),
            tensor_scatter: tensor_scatter.into_boxed_slice(),
        }
    }

    /// Add one check constraint to the side table.
    pub(crate) fn push_check(&mut self, check: mir::CheckConstraint) -> CheckId {
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

    /// Add one allocation layout to the side table.
    pub(crate) fn push_allocation_layout(
        &mut self,
        allocation: AllocationLayout,
    ) -> AllocationLayoutId {
        let id = self.allocation_layout.len() as u32;
        self.allocation_layout.push(allocation);

        AllocationLayoutId(id)
    }

    /// Add one constant to the side table.
    pub(crate) fn push_constant(&mut self, constant: ConstValue) -> ConstValueId {
        let id = self.constant.len() as u32;
        self.constant.push(constant);

        ConstValueId(id)
    }

    /// Add one allocation class to the side table.
    pub(crate) fn push_allocation_class(
        &mut self,
        allocation_class: heap::AllocationClass,
    ) -> AllocationClassId {
        if let Some(id) = self
            .allocation_class
            .iter()
            .position(|class| *class == allocation_class)
        {
            return AllocationClassId(id as u32);
        }

        let id = self.allocation_class.len() as u32;
        self.allocation_class.push(allocation_class);

        AllocationClassId(id)
    }

    /// Add one small allocation layout to the side table.
    pub(crate) fn push_small_allocation_layout(
        &mut self,
        small: heap::SmallAllocationLayout,
    ) -> SmallAllocationLayoutId {
        if let Some(id) = self
            .small_allocation_layout
            .iter()
            .position(|existing| *existing == small)
        {
            return SmallAllocationLayoutId(id as u32);
        }

        let id = self.small_allocation_layout.len() as u32;
        self.small_allocation_layout.push(small);

        SmallAllocationLayoutId(id)
    }

    /// Add one reference map to the side table.
    pub(crate) fn push_reference_map(
        &mut self,
        reference_map: mir::ReferenceMap,
    ) -> ReferenceMapId {
        if let Some(id) = self
            .reference_map
            .iter()
            .position(|existing| *existing == reference_map)
        {
            return ReferenceMapId(id as u32);
        }

        let id = self.reference_map.len() as u32;
        self.reference_map.push(reference_map);

        ReferenceMapId(id)
    }

    /// Add one field access to the side table.
    pub(crate) fn push_field_access(&mut self, access: FieldAccess) -> FieldAccessId {
        let id = self.field_access.len() as u32;
        self.field_access.push(access);

        FieldAccessId(id)
    }

    /// Add one frame access to the side table.
    pub(crate) fn push_frame_access(&mut self, access: FrameAccess) -> FrameAccessId {
        let id = self.frame_access.len() as u32;
        self.frame_access.push(access);

        FrameAccessId(id)
    }

    /// Add one element access to the side table.
    pub(crate) fn push_element_access(&mut self, access: ElementAccess) -> ElementAccessId {
        let id = self.element_access.len() as u32;
        self.element_access.push(access);

        ElementAccessId(id)
    }

    /// Add one slice element access to the side table.
    pub(crate) fn push_slice_element_access(
        &mut self,
        access: SliceElementAccess,
    ) -> SliceElementAccessId {
        let id = self.slice_element_access.len() as u32;
        self.slice_element_access.push(access);

        SliceElementAccessId(id)
    }

    /// Add one pointee access to the side table.
    pub(crate) fn push_pointee_access(&mut self, access: PointeeAccess) -> PointeeAccessId {
        let id = self.pointee_access.len() as u32;
        self.pointee_access.push(access);

        PointeeAccessId(id)
    }

    /// Add one u32 slice to the side table.
    pub(crate) fn push_u32_range(&mut self, values: &[u32]) -> U32RangeId {
        let id = self.u32_ranges.len() as u32;
        self.u32_ranges.push(values.into());

        U32RangeId(id)
    }

    /// Add one MIR type slice to the side table.
    pub(crate) fn push_type_range(
        &mut self,
        values: &[mir::LocalNodeId<mir::Type>],
    ) -> TypeRangeId {
        let id = self.type_ranges.len() as u32;
        self.type_ranges.push(values.into());

        TypeRangeId(id)
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
}

impl SideTable {
    /// Borrow one pooled u32 slice.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &[u32] {
        &self.u32_ranges[id.0 as usize]
    }

    /// Borrow one pooled MIR type slice.
    #[inline(always)]
    pub(crate) fn type_range(&self, id: TypeRangeId) -> &[mir::LocalNodeId<mir::Type>] {
        &self.type_ranges[id.0 as usize]
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

    /// Borrow one pooled allocation layout.
    #[inline(always)]
    pub(crate) fn allocation_layout(&self, id: AllocationLayoutId) -> &AllocationLayout {
        &self.allocation_layout[id.0 as usize]
    }

    /// Borrow one pooled constant.
    #[inline(always)]
    pub(crate) fn constant(&self, id: ConstValueId) -> &ConstValue {
        &self.constant[id.0 as usize]
    }

    /// Return one pooled allocation class.
    #[inline(always)]
    pub(crate) fn allocation_class(&self, id: AllocationClassId) -> heap::AllocationClass {
        self.allocation_class[id.0 as usize]
    }

    /// Return one pooled small allocation layout.
    #[inline(always)]
    pub(crate) fn small_allocation_layout(
        &self,
        id: SmallAllocationLayoutId,
    ) -> heap::SmallAllocationLayout {
        self.small_allocation_layout[id.0 as usize]
    }

    /// Borrow one pooled reference map.
    #[inline(always)]
    pub(crate) fn reference_map(&self, id: ReferenceMapId) -> &mir::ReferenceMap {
        &self.reference_map[id.0 as usize]
    }

    /// Borrow one pooled field access.
    #[inline(always)]
    pub(crate) fn field_access(&self, id: FieldAccessId) -> &FieldAccess {
        &self.field_access[id.0 as usize]
    }

    /// Borrow one pooled frame access.
    #[inline(always)]
    pub(crate) fn frame_access(&self, id: FrameAccessId) -> &FrameAccess {
        &self.frame_access[id.0 as usize]
    }

    /// Borrow one pooled element access.
    #[inline(always)]
    pub(crate) fn element_access(&self, id: ElementAccessId) -> &ElementAccess {
        &self.element_access[id.0 as usize]
    }

    /// Borrow one pooled slice element access.
    #[inline(always)]
    pub(crate) fn slice_element_access(&self, id: SliceElementAccessId) -> &SliceElementAccess {
        &self.slice_element_access[id.0 as usize]
    }

    /// Borrow one pooled pointee access.
    #[inline(always)]
    pub(crate) fn pointee_access(&self, id: PointeeAccessId) -> &PointeeAccess {
        &self.pointee_access[id.0 as usize]
    }

    /// Borrow one pooled check constraint.
    #[inline(always)]
    pub(crate) fn check(&self, id: CheckId) -> &mir::CheckConstraint {
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
