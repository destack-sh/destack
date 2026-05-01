use {destack_heap as heap, destack_mir as mir};

use super::{
    AllocationLayout, ElementAccess, FieldAccess, FrameAccess, PointeeAccess, SliceElementAccess,
    SwitchCase,
};

/// Identifier for one pooled check constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CheckId(u32);

/// Identifier for one pooled switch case table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwitchCasesId(u32);

/// Identifier for one pooled dense switch table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwitchTableId(u32);

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
pub(crate) struct AllocationLayoutId(u32);

/// Identifier for one pooled allocation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AllocationClassId(u32);

/// Identifier for one pooled reference map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReferenceMapId(u32);

/// Identifier for one pooled field access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FieldAccessId(u32);

/// Identifier for one pooled frame access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameAccessId(u32);

/// Identifier for one pooled element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementAccessId(u32);

/// Identifier for one pooled slice element access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SliceElementAccessId(u32);

/// Identifier for one pooled pointee access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointeeAccessId(u32);

/// Identifier for one pooled u32 slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct U32RangeId(u32);

/// Identifier for one pooled MIR type slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TypeRangeId(u32);

/// Identifier for one pooled tensor dot descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorDotId(u32);

/// Identifier for one pooled tensor convolution dimension descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorConvolutionId(u32);

/// Identifier for one pooled tensor convolution window descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorWindowId(u32);

/// Identifier for one pooled tensor gather descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorGatherId(u32);

/// Identifier for one pooled tensor scatter descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TensorScatterId(u32);

/// Immutable side table referenced by compact instruction operands.
#[derive(Clone, Debug, Default)]
pub(crate) struct OperandTable {
    /// Pooled allocation layouts.
    allocation_layout: Box<[AllocationLayout]>,
    /// Pooled allocation classes.
    allocation_class: Box<[heap::AllocationClass]>,
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
    /// Rare operand payloads.
    rare: Box<RareOperandTable>,
}

/// Rare immutable operands referenced by uncommon instruction families.
#[derive(Clone, Debug, Default)]
struct RareOperandTable {
    /// Pooled check constraints.
    check: Box<[mir::CheckConstraint]>,
    /// Pooled switch case tables.
    switch_cases: Box<[Box<[SwitchCase]>]>,
    /// Pooled dense switch tables.
    switch_table: Box<[SwitchTable]>,
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

/// Mutable operand table used while lowering one program.
#[derive(Debug, Default)]
pub(crate) struct OperandTableBuilder {
    /// Pooled check constraints.
    check: Vec<mir::CheckConstraint>,
    /// Pooled switch case tables.
    switch_cases: Vec<Box<[SwitchCase]>>,
    /// Pooled dense switch tables.
    switch_table: Vec<SwitchTable>,
    /// Pooled allocation layouts.
    allocation_layout: Vec<AllocationLayout>,
    /// Pooled allocation classes.
    allocation_class: Vec<heap::AllocationClass>,
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

impl OperandTableBuilder {
    /// Finish the immutable operand table.
    pub(crate) fn finish(self) -> OperandTable {
        let Self {
            check,
            switch_cases,
            switch_table,
            allocation_layout,
            allocation_class,
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

        let rare = Box::new(RareOperandTable {
            check: check.into_boxed_slice(),
            switch_cases: switch_cases.into_boxed_slice(),
            switch_table: switch_table.into_boxed_slice(),
            u32_ranges: u32_ranges.into_boxed_slice(),
            type_ranges: type_ranges.into_boxed_slice(),
            tensor_dot: tensor_dot.into_boxed_slice(),
            tensor_convolution: tensor_convolution.into_boxed_slice(),
            tensor_window: tensor_window.into_boxed_slice(),
            tensor_gather: tensor_gather.into_boxed_slice(),
            tensor_scatter: tensor_scatter.into_boxed_slice(),
        });

        OperandTable {
            allocation_layout: allocation_layout.into_boxed_slice(),
            allocation_class: allocation_class.into_boxed_slice(),
            reference_map: reference_map.into_boxed_slice(),
            field_access: field_access.into_boxed_slice(),
            frame_access: frame_access.into_boxed_slice(),
            element_access: element_access.into_boxed_slice(),
            slice_element_access: slice_element_access.into_boxed_slice(),
            pointee_access: pointee_access.into_boxed_slice(),
            rare,
        }
    }

    /// Add one check constraint to the operand table.
    pub(crate) fn push_check(&mut self, check: mir::CheckConstraint) -> CheckId {
        let id = self.check.len() as u32;
        self.check.push(check);

        CheckId(id)
    }

    /// Add one switch case table to the operand table.
    pub(crate) fn push_switch_cases(&mut self, cases: Box<[SwitchCase]>) -> SwitchCasesId {
        let id = self.switch_cases.len() as u32;
        self.switch_cases.push(cases);

        SwitchCasesId(id)
    }

    /// Add one dense switch table to the operand table.
    pub(crate) fn push_switch_table(&mut self, table: SwitchTable) -> SwitchTableId {
        let id = self.switch_table.len() as u32;
        self.switch_table.push(table);

        SwitchTableId(id)
    }

    /// Add one allocation layout to the operand table.
    pub(crate) fn push_allocation_layout(
        &mut self,
        allocation: AllocationLayout,
    ) -> AllocationLayoutId {
        let id = self.allocation_layout.len() as u32;
        self.allocation_layout.push(allocation);

        AllocationLayoutId(id)
    }

    /// Add one allocation class to the operand table.
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

    /// Add one reference map to the operand table.
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

    /// Add one field access to the operand table.
    pub(crate) fn push_field_access(&mut self, access: FieldAccess) -> FieldAccessId {
        let id = self.field_access.len() as u32;
        self.field_access.push(access);

        FieldAccessId(id)
    }

    /// Add one frame access to the operand table.
    pub(crate) fn push_frame_access(&mut self, access: FrameAccess) -> FrameAccessId {
        let id = self.frame_access.len() as u32;
        self.frame_access.push(access);

        FrameAccessId(id)
    }

    /// Add one element access to the operand table.
    pub(crate) fn push_element_access(&mut self, access: ElementAccess) -> ElementAccessId {
        let id = self.element_access.len() as u32;
        self.element_access.push(access);

        ElementAccessId(id)
    }

    /// Add one slice element access to the operand table.
    pub(crate) fn push_slice_element_access(
        &mut self,
        access: SliceElementAccess,
    ) -> SliceElementAccessId {
        let id = self.slice_element_access.len() as u32;
        self.slice_element_access.push(access);

        SliceElementAccessId(id)
    }

    /// Add one pointee access to the operand table.
    pub(crate) fn push_pointee_access(&mut self, access: PointeeAccess) -> PointeeAccessId {
        let id = self.pointee_access.len() as u32;
        self.pointee_access.push(access);

        PointeeAccessId(id)
    }

    /// Add one u32 slice to the operand table.
    pub(crate) fn push_u32_range(&mut self, values: &[u32]) -> U32RangeId {
        let id = self.u32_ranges.len() as u32;
        self.u32_ranges.push(values.into());

        U32RangeId(id)
    }

    /// Add one MIR type slice to the operand table.
    pub(crate) fn push_type_range(
        &mut self,
        values: &[mir::LocalNodeId<mir::Type>],
    ) -> TypeRangeId {
        let id = self.type_ranges.len() as u32;
        self.type_ranges.push(values.into());

        TypeRangeId(id)
    }

    /// Add one tensor dot descriptor to the operand table.
    pub(crate) fn push_tensor_dot(
        &mut self,
        dimensions: mir::TensorDotDimensionNumbers,
    ) -> TensorDotId {
        let id = self.tensor_dot.len() as u32;
        self.tensor_dot.push(dimensions);

        TensorDotId(id)
    }

    /// Add one tensor convolution dimension descriptor to the operand table.
    pub(crate) fn push_tensor_convolution(
        &mut self,
        dimensions: mir::TensorConvolutionDimensionNumbers,
    ) -> TensorConvolutionId {
        let id = self.tensor_convolution.len() as u32;
        self.tensor_convolution.push(dimensions);

        TensorConvolutionId(id)
    }

    /// Add one tensor convolution window descriptor to the operand table.
    pub(crate) fn push_tensor_window(
        &mut self,
        window: mir::TensorConvolutionWindow,
    ) -> TensorWindowId {
        let id = self.tensor_window.len() as u32;
        self.tensor_window.push(window);

        TensorWindowId(id)
    }

    /// Add one tensor gather descriptor to the operand table.
    pub(crate) fn push_tensor_gather(
        &mut self,
        dimensions: mir::TensorGatherDimensionNumbers,
    ) -> TensorGatherId {
        let id = self.tensor_gather.len() as u32;
        self.tensor_gather.push(dimensions);

        TensorGatherId(id)
    }

    /// Add one tensor scatter descriptor to the operand table.
    pub(crate) fn push_tensor_scatter(
        &mut self,
        dimensions: mir::TensorScatterDimensionNumbers,
    ) -> TensorScatterId {
        let id = self.tensor_scatter.len() as u32;
        self.tensor_scatter.push(dimensions);

        TensorScatterId(id)
    }
}

impl OperandTable {
    /// Borrow one pooled u32 slice.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &[u32] {
        &self.rare.u32_ranges[id.0 as usize]
    }

    /// Borrow one pooled MIR type slice.
    #[inline(always)]
    pub(crate) fn type_range(&self, id: TypeRangeId) -> &[mir::LocalNodeId<mir::Type>] {
        &self.rare.type_ranges[id.0 as usize]
    }

    /// Borrow one tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> &mir::TensorDotDimensionNumbers {
        &self.rare.tensor_dot[id.0 as usize]
    }

    /// Borrow one tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> &mir::TensorConvolutionDimensionNumbers {
        &self.rare.tensor_convolution[id.0 as usize]
    }

    /// Borrow one tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(&self, id: TensorWindowId) -> &mir::TensorConvolutionWindow {
        &self.rare.tensor_window[id.0 as usize]
    }

    /// Borrow one tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(&self, id: TensorGatherId) -> &mir::TensorGatherDimensionNumbers {
        &self.rare.tensor_gather[id.0 as usize]
    }

    /// Borrow one tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(
        &self,
        id: TensorScatterId,
    ) -> &mir::TensorScatterDimensionNumbers {
        &self.rare.tensor_scatter[id.0 as usize]
    }

    /// Borrow one pooled allocation layout.
    #[inline(always)]
    pub(crate) fn allocation_layout(&self, id: AllocationLayoutId) -> &AllocationLayout {
        &self.allocation_layout[id.0 as usize]
    }

    /// Return one pooled allocation class.
    #[inline(always)]
    pub(crate) fn allocation_class(&self, id: AllocationClassId) -> heap::AllocationClass {
        self.allocation_class[id.0 as usize]
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
        &self.rare.check[id.0 as usize]
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &[SwitchCase] {
        &self.rare.switch_cases[id.0 as usize]
    }

    /// Borrow one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> &SwitchTable {
        &self.rare.switch_table[id.0 as usize]
    }
}
