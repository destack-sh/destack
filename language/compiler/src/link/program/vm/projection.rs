use destack_mir as mir;

use super::layout::StorageLayout;
use super::lower::BlockLowerer;
use super::value::Operand;
use crate::LinkResult;
use destack_program::CellLayout;
use destack_program::vm::{Projection, SliceProjection, SlotProjection};

impl<'a> BlockLowerer<'a> {
    /// Return one lowered layout by MIR type.
    pub(super) fn layout_for_type(
        &self,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<&StorageLayout> {
        self.layouts()
            .get(&value_type)
            .ok_or_else(|| self.invalid_input(format!("layout for type {value_type:?}")))
    }

    /// Return one lowered field count for one value.
    pub(super) fn field_count_for_value(&self, value: mir::Value) -> LinkResult<u32> {
        if let Some(count) = self
            .operand_map()
            .get(value)
            .and_then(|layout| self.field_count_for_layout(layout))
        {
            return Ok(count);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(self.invalid_instruction("field count value"));
        };

        self.field_count_for_type(value_type)
    }

    /// Return one lowered array length for one value.
    pub(super) fn array_length_for_value(&self, value: mir::Value) -> LinkResult<u64> {
        if let Some(length) = self
            .operand_map()
            .get(value)
            .and_then(|layout| self.sequence_element_count(layout))
        {
            return Ok(length);
        }

        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(self.invalid_instruction("array length value"));
        };

        self.array_length_for_type(value_type)
    }

    /// Return the type projected by one value.
    pub(super) fn projection_type_for_value(
        &self,
        value: mir::Value,
    ) -> LinkResult<Option<mir::LocalNodeId<mir::Type>>> {
        let pointee_type = self
            .operand_map()
            .heap_pointee_type(self.function.program, value)
            .or_else(|| {
                self.operand_map()
                    .raw_pointee_type(self.function.program, value)
            });

        if pointee_type.is_some() {
            return Ok(pointee_type);
        }

        Ok(Some(self.value_type_for_value(value)?))
    }

    /// Return one lowered field projection for a value.
    pub(super) fn field_projection_for_value(
        &self,
        value: mir::Value,
        index: u32,
    ) -> LinkResult<Projection> {
        let field_count = self.field_count_for_value(value)? as usize;
        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(self.invalid_field_access(index, field_count));
        };
        self.field_projection(value_type, index)
            .ok_or_else(|| self.invalid_field_access(index, field_count))
    }

    /// Return one lowered element projection for a value.
    pub(super) fn element_projection_for_value(&self, value: mir::Value) -> LinkResult<Projection> {
        let value_type = self.projection_type_for_value(value)?;
        let Some(value_type) = value_type else {
            return Err(self.invalid_instruction("element projection value"));
        };
        self.element_projection(value_type)
            .ok_or_else(|| self.invalid_instruction("element projection"))
    }

    /// Return one field count from a concrete type.
    fn field_count_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> LinkResult<u32> {
        match self.function.tree.get(value_type) {
            mir::Type::Struct { fields, copy: _ } => {
                u32::try_from(fields.len()).map_err(|_| self.layout_overflow("field count"))
            }
            mir::Type::Tuple { elements, copy: _ } => {
                u32::try_from(elements.len()).map_err(|_| self.layout_overflow("field count"))
            }
            _ => Err(self.invalid_instruction("field count type")),
        }
    }

    /// Return one array length from a concrete type.
    fn array_length_for_type(&self, value_type: mir::LocalNodeId<mir::Type>) -> LinkResult<u64> {
        match self.function.tree.get(value_type) {
            mir::Type::FixedArray { length, .. } => Ok(*length),
            _ => Err(self.invalid_instruction("array length type")),
        }
    }

    /// Return the MIR type for one value.
    pub(super) fn value_type_for_value(
        &self,
        value: mir::Value,
    ) -> LinkResult<mir::LocalNodeId<mir::Type>> {
        self.value_types()
            .get(value.0 as usize)
            .copied()
            .ok_or_else(|| self.invalid_input(format!("type for value {value:?}")))
    }

    /// Build one field projection from one storage layout.
    pub(super) fn field_projection(
        &self,
        pointee_type: mir::LocalNodeId<mir::Type>,
        index: u32,
    ) -> Option<Projection> {
        let layout = self.layouts().get(&pointee_type)?;

        // resolve named fields first
        if let Some(field) = layout.field(index) {
            let cell_layout = self.access_cell_layout(field.ty);

            return Some(Projection::fixed(
                self.function.program.type_id(field.ty),
                field.offset,
                field.byte_len(),
                cell_layout,
            ));
        }

        // resolve fixed array slots by static index
        let element = layout.element()?;
        let count = layout.element_count()?;
        if index as usize >= count {
            return None;
        }

        let cell_layout = self.access_cell_layout(element.ty);
        let offset = element.stride * index as usize;

        Some(Projection::fixed(
            self.function.program.type_id(element.ty),
            offset,
            element.byte_len(),
            cell_layout,
        ))
    }

    /// Build the vtable projection for one class receiver.
    pub(super) fn virtual_table_projection(
        &self,
        receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    ) -> Option<Projection> {
        let receiver_type = receiver_type?;
        let raw_layout = self.function.layout_for_type(receiver_type)?;
        let mir::LayoutShape::Object(_) = &raw_layout.shape else {
            return None;
        };

        self.field_projection_at_offset(receiver_type, 0)
    }

    /// Build the dispatch-table projection for one dynamic receiver.
    pub(super) fn dynamic_table_projection(
        &self,
        receiver_type: Option<mir::LocalNodeId<mir::Type>>,
    ) -> Option<Projection> {
        let receiver_type = receiver_type?;
        let raw_layout = self.function.layout_for_type(receiver_type)?;
        let dispatch_offset = raw_layout.dynamic_dispatch_offset()?;

        self.field_projection_at_offset(receiver_type, dispatch_offset as usize)
    }

    /// Build one element projection from one storage layout.
    pub(super) fn element_projection(
        &self,
        pointee_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<Projection> {
        let layout = self.layouts().get(&pointee_type)?;
        let element = layout.element()?;
        let length = layout.element_count()? as u64;

        // cache scalar layout for lowered memory ops
        let cell_layout = self.access_cell_layout(element.ty);

        Some(Projection::indexed(
            self.function.program.type_id(element.ty),
            length,
            element.stride,
            element.byte_len(),
            cell_layout,
        ))
    }

    /// Build one slice projection from one slice descriptor type.
    pub(super) fn slice_projection(
        &self,
        slice_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<SliceProjection> {
        let mir::Type::Slice {
            element,
            kind,
            space,
            ..
        } = self
            .function
            .tree
            .get(self.function.tree.repr_type(slice_type))
        else {
            return None;
        };
        let element_type = *element;
        let layout = self.layouts().get(&slice_type)?;
        if !layout.is_slice() {
            return None;
        }

        let element_layout = self.layouts().get(&element_type)?;
        let element_cell_layout = self.access_cell_layout(element_type);
        let pointer_cell_layout = self.function.reference_cell_layout(*space, *kind);
        let pointer_bytes = self.function.pointer_bytes() as usize;
        let pointer_byte_len = pointer_cell_layout.byte_len(pointer_bytes);
        let length_offset = pointer_byte_len.next_multiple_of(pointer_bytes);
        let length_cell_layout = CellLayout::Uint {
            width: usize::BITS as u8,
        };

        Some(SliceProjection {
            pointer: SlotProjection::fixed(0, pointer_byte_len, pointer_cell_layout),
            length: SlotProjection::fixed(length_offset, pointer_bytes, length_cell_layout),
            element: Projection::indexed(
                self.function.program.type_id(element_type),
                0,
                element_layout.stride(),
                element_layout.byte_len(),
                element_cell_layout,
            ),
        })
    }

    /// Resolve the backing pointer layout for one slice descriptor type.
    pub(super) fn slice_element_cell_layout(
        &self,
        slice_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<CellLayout> {
        let mir::Type::Slice { kind, space, .. } = self
            .function
            .tree
            .get(self.function.tree.repr_type(slice_type))
        else {
            return None;
        };

        Some(self.function.reference_cell_layout(*space, *kind))
    }

    /// Build one pointee projection from one storage layout.
    pub(super) fn pointee_projection(
        &self,
        pointee_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<Projection> {
        let layout = self.layouts().get(&pointee_type)?;
        let cell_layout = self.access_cell_layout(pointee_type);

        Some(Projection::fixed(
            self.function.program.type_id(pointee_type),
            0,
            layout.byte_len(),
            cell_layout,
        ))
    }

    /// Build one tensor element projection from one storage element layout.
    pub(super) fn tensor_element_projection(
        &self,
        element_type: mir::LocalNodeId<mir::Type>,
    ) -> Option<Projection> {
        let layout = self.layouts().get(&element_type)?;
        let cell_layout = self.access_cell_layout(element_type);

        Some(Projection::indexed(
            self.function.program.type_id(element_type),
            0,
            layout.stride(),
            layout.byte_len(),
            cell_layout,
        ))
    }

    /// Resolve the pointer layout for one tensor value type.
    pub(super) fn tensor_view_cell_layout(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<CellLayout> {
        match self.function.tree.get(self.function.tree.repr_type(ty)) {
            mir::Type::Tensor { .. } => Some(CellLayout::HeapReference),
            mir::Type::TensorView { kind, space, .. } => {
                Some(self.function.reference_cell_layout(*space, *kind))
            }
            _ => None,
        }
    }

    /// Resolve the storage space for one tensor value type.
    pub(super) fn tensor_view_space(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<mir::Space> {
        match self.function.tree.get(self.function.tree.repr_type(ty)) {
            mir::Type::Tensor { .. } => Some(mir::Space::Local),
            mir::Type::TensorView { space, .. } => Some(*space),
            _ => None,
        }
    }

    /// Resolve the tensor element type for one tensor value or reference type.
    pub(super) fn tensor_element_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        let ty = self.function.tree.repr_type(ty);

        match self.function.tree.get(ty) {
            mir::Type::Tensor { element, .. } | mir::Type::TensorView { element, .. } => {
                Some(*element)
            }
            _ => None,
        }
    }

    /// Build one field projection from a lowered byte offset.
    fn field_projection_at_offset(
        &self,
        pointee_type: mir::LocalNodeId<mir::Type>,
        byte_offset: usize,
    ) -> Option<Projection> {
        let layout = self.layouts().get(&pointee_type)?;
        let field = (0..layout.field_count()?)
            .filter_map(|index| layout.field(index as u32))
            .find(|field| field.offset == byte_offset)?;

        let cell_layout = self.access_cell_layout(field.ty);

        Some(Projection::fixed(
            self.function.program.type_id(field.ty),
            field.offset,
            field.byte_len(),
            cell_layout,
        ))
    }

    /// Report whether one storage layout stays scalar in lowered memory ops.
    fn access_cell_layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<CellLayout> {
        if !self.layouts().get(&ty).is_some_and(StorageLayout::is_cell) {
            return None;
        }

        self.function.cell_layout_for_type(ty)
    }

    /// Resolve the field count for a struct or tuple layout.
    fn field_count_for_layout(&self, operand: Operand) -> Option<u32> {
        match operand {
            Operand::Aggregate { ty } => {
                match self
                    .function
                    .tree
                    .get(self.function.program.type_by_id(ty)?)
                {
                    mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
                    mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
                    _ => None,
                }
            }
            Operand::Reference { pointee, .. } => {
                match self
                    .function
                    .tree
                    .get(self.function.program.type_by_id(pointee)?)
                {
                    mir::Type::Struct { fields, copy: _ } => u32::try_from(fields.len()).ok(),
                    mir::Type::Tuple { elements, copy: _ } => u32::try_from(elements.len()).ok(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Resolve the element count for a sequence operand.
    fn sequence_element_count(&self, operand: Operand) -> Option<u64> {
        match operand {
            Operand::Sequence { length, .. } => Some(length),
            Operand::Aggregate { ty } => {
                match self
                    .function
                    .tree
                    .get(self.function.program.type_by_id(ty)?)
                {
                    mir::Type::FixedArray { length, .. } => Some(*length),
                    mir::Type::Vector { lanes, .. } => Some(u64::from(*lanes)),
                    _ => None,
                }
            }
            Operand::Reference { pointee, .. } => {
                match self
                    .function
                    .tree
                    .get(self.function.program.type_by_id(pointee)?)
                {
                    mir::Type::FixedArray { length, .. } => Some(*length),
                    mir::Type::Vector { lanes, .. } => Some(u64::from(*lanes)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
