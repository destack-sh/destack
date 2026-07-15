use destack_mir as mir;

use crate::LinkResult;

use destack_program::CellLayout;
use destack_program::vm::{
    AtomicAddress, AtomicCompareExchange, AtomicOrder, AtomicReadModifyWriteOperator,
    AtomicReadModifyWriteShape, AtomicShape, AtomicWidth, Instruction, Op,
};

use super::super::TypeLinker;
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
        _pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        pointer: mir::Value,
        access: mir::AtomicAccess,
    ) -> LinkResult<Instruction> {
        // derive the concrete atomic shape
        let (layout, address) = self.require_atomic_pointer(
            &self.function.type_linker(),
            self.function.tree,
            self.value_types(),
            pointer,
        )?;
        let pointer_bytes = self.function.pointer_bytes() as usize;
        let shape = self.atomic_shape(access, layout, address, pointer_bytes)?;

        // encode the operation directly into the instruction
        Ok(Instruction::new(
            Op::AtomicLoad,
            self.cell_offset(destination)?,
            self.cell_offset(pointer)?,
            0,
            shape.encode(),
        ))
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
        _pool: &mut Pool<'_, '_>,
        pointer: mir::Value,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> LinkResult<Instruction> {
        // derive the concrete atomic shape
        let (layout, address) = self.require_atomic_pointer(
            &self.function.type_linker(),
            self.function.tree,
            self.value_types(),
            pointer,
        )?;
        let pointer_bytes = self.function.pointer_bytes() as usize;
        let shape = self.atomic_shape(access, layout, address, pointer_bytes)?;

        // encode the operation directly into the instruction
        Ok(Instruction::new(
            Op::AtomicStore,
            self.cell_offset(pointer)?,
            self.cell_offset(value)?,
            0,
            shape.encode(),
        ))
    }

    /// Lower one atomic compare exchange.
    pub(super) fn lower_atomic_compare_exchange(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        pointer: mir::Value,
        expected: mir::Value,
        new_value: mir::Value,
        is_weak: bool,
        access: mir::CompareExchangeAccess,
    ) -> LinkResult<Instruction> {
        // derive the concrete atomic shape
        let (layout, address) = self.require_atomic_pointer(
            &self.function.type_linker(),
            self.function.tree,
            self.value_types(),
            pointer,
        )?;
        let failure_order = AtomicOrder::from(access.failure_ordering);
        let pointer_bytes = self.function.pointer_bytes() as usize;
        let width = self.atomic_width(layout, pointer_bytes)?;
        let shape = atomic_shape_from_parts(access.success, layout, address, width);

        // store aggregate destination tables out of line
        Ok(pool.instruction_with_side(
            Op::AtomicCompareExchange,
            AtomicCompareExchange {
                destination: self.move_slot(destination)?,
                pointer_offset: self.cell_offset(pointer)?,
                expected_offset: self.cell_offset(expected)?,
                new_value_offset: self.cell_offset(new_value)?,
                shape,
                failure_order,
                is_weak,
            },
        ))
    }

    /// Lower one atomic read-modify-write.
    pub(super) fn lower_atomic_rmw(
        &self,
        _pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        operator: mir::AtomicRmwOperator,
        pointer: mir::Value,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> LinkResult<Instruction> {
        // derive the concrete atomic shape
        let (layout, address) = self.require_atomic_pointer(
            &self.function.type_linker(),
            self.function.tree,
            self.value_types(),
            pointer,
        )?;
        let pointer_bytes = self.function.pointer_bytes() as usize;
        let shape = self.atomic_shape(access, layout, address, pointer_bytes)?;

        // exchange has its own opcode because it maps to a single native atomic primitive
        if operator == mir::AtomicRmwOperator::Exchange {
            return Ok(Instruction::new(
                Op::AtomicExchange,
                self.cell_offset(destination)?,
                self.cell_offset(pointer)?,
                self.cell_offset(value)?,
                shape.encode(),
            ));
        }

        // encode the remaining update operator into the compact shape
        let operator = self.atomic_read_modify_write_operator(operator, layout)?;
        let shape = AtomicReadModifyWriteShape::new(shape, operator);

        Ok(Instruction::new(
            Op::AtomicReadModifyWrite,
            self.cell_offset(destination)?,
            self.cell_offset(pointer)?,
            self.cell_offset(value)?,
            shape.encode(),
        ))
    }

    /// Lower one atomic fence.
    pub(super) fn lower_atomic_fence(
        &self,
        _pool: &mut Pool<'_, '_>,
        access: mir::FenceAccess,
    ) -> Instruction {
        // fences only need their ordering at execution
        let order = AtomicOrder::from(access.ordering);

        Instruction::new(Op::AtomicFence, 0, 0, 0, order.encode())
    }
}

impl BlockLowerer<'_> {
    /// Build the compact shape for one atomic memory operation.
    fn atomic_shape(
        &self,
        access: mir::AtomicAccess,
        layout: CellLayout,
        address: AtomicAddress,
        pointer_bytes: usize,
    ) -> LinkResult<AtomicShape> {
        let width = self.atomic_width(layout, pointer_bytes)?;

        Ok(atomic_shape_from_parts(access, layout, address, width))
    }

    /// Map one MIR read-modify-write operator to a VM update operation.
    fn atomic_read_modify_write_operator(
        &self,
        operator: mir::AtomicRmwOperator,
        layout: CellLayout,
    ) -> LinkResult<AtomicReadModifyWriteOperator> {
        use mir::AtomicRmwOperator::*;

        Ok(match operator {
            Add if is_integer(layout) => AtomicReadModifyWriteOperator::Add,
            Sub if is_integer(layout) => AtomicReadModifyWriteOperator::Sub,
            And if is_bits(layout) => AtomicReadModifyWriteOperator::And,
            Or if is_bits(layout) => AtomicReadModifyWriteOperator::Or,
            Xor if is_bits(layout) => AtomicReadModifyWriteOperator::Xor,
            Min if is_signed_integer(layout) => AtomicReadModifyWriteOperator::Min,
            Max if is_signed_integer(layout) => AtomicReadModifyWriteOperator::Max,
            Umin if is_unsigned_integer(layout) => AtomicReadModifyWriteOperator::Umin,
            Umax if is_unsigned_integer(layout) => AtomicReadModifyWriteOperator::Umax,
            Fadd if is_float(layout) => AtomicReadModifyWriteOperator::Fadd,
            Fmin if is_float(layout) => AtomicReadModifyWriteOperator::Fmin,
            Fmax if is_float(layout) => AtomicReadModifyWriteOperator::Fmax,
            _ => return Err(self.invalid_instruction("atomic read modify write operator")),
        })
    }

    /// Return the atomic payload width for one cell layout.
    fn atomic_width(&self, layout: CellLayout, pointer_bytes: usize) -> LinkResult<AtomicWidth> {
        let byte_len = layout.byte_len(pointer_bytes);

        Ok(match byte_len {
            1 => AtomicWidth::Width8,
            2 => AtomicWidth::Width16,
            4 => AtomicWidth::Width32,
            8 => AtomicWidth::Width64,
            _ => return Err(self.invalid_instruction("atomic width")),
        })
    }

    /// Require an atomic pointer with a concrete cell layout and address class.
    fn require_atomic_pointer(
        &self,
        type_linker: &TypeLinker<'_>,
        tree: &mir::Tree,
        value_types: &[mir::LocalNodeId<mir::Type>],
        pointer: mir::Value,
    ) -> LinkResult<(CellLayout, AtomicAddress)> {
        // resolve the pointer value type
        let pointer_type = self.value_type_for_atomic_pointer(tree, value_types, pointer)?;
        let pointer_type = tree.repr_type(pointer_type);

        // require a concrete reference type
        let mir::Type::Reference {
            kind,
            space,
            pointee,
            ..
        } = tree.get(pointer_type)
        else {
            return Err(self.invalid_pointer_type(format!("{pointer:?}")));
        };

        let pointer = type_linker.reference_cell_layout(*space, *kind);
        let address = atomic_address(pointer)
            .ok_or_else(|| self.invalid_pointer_type(format!("{pointer:?}")))?;

        // require atomic storage
        let pointee = *pointee;
        let pointee = tree.repr_type(pointee);
        let mir::Type::Atomic { value } = tree.get(pointee) else {
            return Err(self.invalid_pointer_type(format!("{:?}", tree.get(pointee))));
        };

        // require a cell-sized atomic payload
        let value = *value;
        let layout = type_linker.cell_layout(value).ok_or_else(|| {
            self.type_mismatch("cell atomic pointee", format!("{:?}", tree.get(value)))
        })?;

        Ok((layout, address))
    }

    /// Return the MIR type for one atomic pointer value.
    fn value_type_for_atomic_pointer(
        &self,
        tree: &mir::Tree,
        value_types: &[mir::LocalNodeId<mir::Type>],
        pointer: mir::Value,
    ) -> LinkResult<mir::LocalNodeId<mir::Type>> {
        value_types
            .get(pointer.0 as usize)
            .copied()
            .ok_or_else(|| self.invalid_pointer_type(format!("{pointer:?}")))
            .map(|ty| tree.repr_type(ty))
    }
}

/// Return whether this layout supports integer arithmetic atomics.
fn is_integer(layout: CellLayout) -> bool {
    matches!(layout, CellLayout::Int { .. } | CellLayout::Uint { .. })
}

/// Return whether this layout supports bitwise atomics.
fn is_bits(layout: CellLayout) -> bool {
    matches!(
        layout,
        CellLayout::Boolean | CellLayout::Int { .. } | CellLayout::Uint { .. }
    )
}

/// Return whether this layout supports signed min/max atomics.
fn is_signed_integer(layout: CellLayout) -> bool {
    matches!(layout, CellLayout::Int { .. })
}

/// Return whether this layout supports unsigned min/max atomics.
fn is_unsigned_integer(layout: CellLayout) -> bool {
    matches!(layout, CellLayout::Uint { .. })
}

/// Return whether this layout supports floating CAS-loop atomics.
fn is_float(layout: CellLayout) -> bool {
    matches!(layout, CellLayout::Float32 | CellLayout::Float64)
}

/// Build the VM atomic shape for one MIR atomic instruction.
fn atomic_shape_from_parts(
    access: mir::AtomicAccess,
    layout: CellLayout,
    address: AtomicAddress,
    width: AtomicWidth,
) -> AtomicShape {
    let order = AtomicOrder::from(access.ordering);
    let is_signed = matches!(layout, CellLayout::Int { .. });

    AtomicShape::new(address, width, order, is_signed)
}

/// Select the VM atomic address representation.
fn atomic_address(pointer: CellLayout) -> Option<AtomicAddress> {
    Some(match pointer {
        CellLayout::HeapReference => AtomicAddress::Heap,
        CellLayout::SharedHeapReference => AtomicAddress::SharedHeap,
        CellLayout::Address => AtomicAddress::Address,
        CellLayout::StackPointer => AtomicAddress::Stack,
        CellLayout::FramePointer => AtomicAddress::Frame,
        CellLayout::GlobalAddress => AtomicAddress::Static,
        _ => return None,
    })
}
