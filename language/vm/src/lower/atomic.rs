use destack_mir as mir;

use crate::program::{
    AddressSpace, AtomicAddress, AtomicCompareExchange, AtomicOrder, AtomicReadModifyWriteOperator,
    AtomicReadModifyWriteShape, AtomicShape, AtomicWidth, CellLayout, Instruction, Op,
    address_space_from_reference, cell_layout_from_type, repr_type,
};
use crate::{Error, Result};

use super::frame::cell_offset;
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
        _pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
        access: mir::AtomicAccess,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = atomic_value(destination, "atomic load destination")?;
        let pointer = atomic_value(pointer, "atomic load pointer")?;

        // derive the concrete atomic shape
        let (layout, address) = require_atomic_pointer(self.tree, self.value_type(), pointer)?;
        let pointer_bytes = self.tree.pointer_bytes() as usize;
        let shape = atomic_shape(access, layout, address, pointer_bytes)?;

        // encode the operation directly into the instruction
        Ok(Instruction::new(
            Op::AtomicLoad,
            cell_offset(self, destination)?,
            cell_offset(self, pointer)?,
            0,
            shape.encode(),
        ))
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
        _pool: &mut Pool<'_, '_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        access: mir::AtomicAccess,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let pointer = atomic_value(pointer, "atomic store pointer")?;
        let value = atomic_value(value, "atomic store value")?;

        // derive the concrete atomic shape
        let (layout, address) = require_atomic_pointer(self.tree, self.value_type(), pointer)?;
        let pointer_bytes = self.tree.pointer_bytes() as usize;
        let shape = atomic_shape(access, layout, address, pointer_bytes)?;

        // encode the operation directly into the instruction
        Ok(Instruction::new(
            Op::AtomicStore,
            cell_offset(self, pointer)?,
            cell_offset(self, value)?,
            0,
            shape.encode(),
        ))
    }

    /// Lower one atomic compare exchange.
    pub(super) fn lower_atomic_compare_exchange(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
        expected: mir::ValueReference,
        new_value: mir::ValueReference,
        is_weak: bool,
        access: mir::CompareExchangeAccess,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = atomic_value(destination, "atomic compare exchange destination")?;
        let pointer = atomic_value(pointer, "atomic compare exchange pointer")?;
        let expected = atomic_value(expected, "atomic compare exchange expected")?;
        let new_value = atomic_value(new_value, "atomic compare exchange new value")?;

        // derive the concrete atomic shape
        let (layout, address) = require_atomic_pointer(self.tree, self.value_type(), pointer)?;
        let failure_order = AtomicOrder::from_mir(access.failure_ordering);
        let pointer_bytes = self.tree.pointer_bytes() as usize;
        let width = atomic_width(layout, pointer_bytes)?;
        let shape = atomic_shape_from_parts(access.success, layout, address, width);

        // store aggregate destination metadata out of line
        Ok(pool.instruction_with_side(
            Op::AtomicCompareExchange,
            AtomicCompareExchange {
                destination,
                pointer_offset: cell_offset(self, pointer)?,
                expected_offset: cell_offset(self, expected)?,
                new_value_offset: cell_offset(self, new_value)?,
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
        destination: mir::ValueReference,
        operator: mir::AtomicRmwOperator,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        access: mir::AtomicAccess,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = atomic_value(destination, "atomic read modify write destination")?;
        let pointer = atomic_value(pointer, "atomic read modify write pointer")?;
        let value = atomic_value(value, "atomic read modify write value")?;

        // derive the concrete atomic shape
        let (layout, address) = require_atomic_pointer(self.tree, self.value_type(), pointer)?;
        let pointer_bytes = self.tree.pointer_bytes() as usize;
        let shape = atomic_shape(access, layout, address, pointer_bytes)?;

        // exchange has its own opcode because it maps to a single native atomic primitive
        if operator == mir::AtomicRmwOperator::Exchange {
            return Ok(Instruction::new(
                Op::AtomicExchange,
                cell_offset(self, destination)?,
                cell_offset(self, pointer)?,
                cell_offset(self, value)?,
                shape.encode(),
            ));
        }

        // encode the remaining update operator into the compact shape
        let operator = atomic_read_modify_write_operator(operator, layout)?;
        let shape = AtomicReadModifyWriteShape::new(shape, operator);

        Ok(Instruction::new(
            Op::AtomicReadModifyWrite,
            cell_offset(self, destination)?,
            cell_offset(self, pointer)?,
            cell_offset(self, value)?,
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
        let order = AtomicOrder::from_mir(access.ordering);

        Instruction::new(Op::AtomicFence, 0, 0, 0, order.encode())
    }
}

/// Return the value carried by one atomic operand reference.
fn atomic_value(reference: mir::ValueReference, context: &str) -> Result<mir::Value> {
    reference
        .value()
        .ok_or_else(|| Error::invalid_program(context))
}

/// Build the compact shape for one atomic memory operation.
fn atomic_shape(
    access: mir::AtomicAccess,
    layout: CellLayout,
    address: AtomicAddress,
    pointer_bytes: usize,
) -> Result<AtomicShape> {
    let width = atomic_width(layout, pointer_bytes)?;

    Ok(atomic_shape_from_parts(access, layout, address, width))
}

/// Map one MIR read-modify-write operator to a VM update operation.
fn atomic_read_modify_write_operator(
    operator: mir::AtomicRmwOperator,
    layout: CellLayout,
) -> Result<AtomicReadModifyWriteOperator> {
    use mir::AtomicRmwOperator as MirOperator;

    Ok(match operator {
        MirOperator::Add if is_integer(layout) => AtomicReadModifyWriteOperator::Add,
        MirOperator::Sub if is_integer(layout) => AtomicReadModifyWriteOperator::Sub,
        MirOperator::And if is_bits(layout) => AtomicReadModifyWriteOperator::And,
        MirOperator::Or if is_bits(layout) => AtomicReadModifyWriteOperator::Or,
        MirOperator::Xor if is_bits(layout) => AtomicReadModifyWriteOperator::Xor,
        MirOperator::Min if is_signed_integer(layout) => AtomicReadModifyWriteOperator::Min,
        MirOperator::Max if is_signed_integer(layout) => AtomicReadModifyWriteOperator::Max,
        MirOperator::Umin if is_unsigned_integer(layout) => AtomicReadModifyWriteOperator::Umin,
        MirOperator::Umax if is_unsigned_integer(layout) => AtomicReadModifyWriteOperator::Umax,
        MirOperator::Fadd if is_float(layout) => AtomicReadModifyWriteOperator::Fadd,
        MirOperator::Fmin if is_float(layout) => AtomicReadModifyWriteOperator::Fmin,
        MirOperator::Fmax if is_float(layout) => AtomicReadModifyWriteOperator::Fmax,
        _ => return Err(Error::invalid_instruction()),
    })
}

/// Return whether this layout supports integer arithmetic atomics.
fn is_integer(layout: CellLayout) -> bool {
    matches!(layout, CellLayout::Int { .. } | CellLayout::Uint { .. })
}

/// Return whether this layout supports bitwise atomics.
fn is_bits(layout: CellLayout) -> bool {
    matches!(
        layout,
        CellLayout::Bool | CellLayout::Int { .. } | CellLayout::Uint { .. }
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

/// Return the atomic payload width for one cell layout.
fn atomic_width(layout: CellLayout, pointer_bytes: usize) -> Result<AtomicWidth> {
    let byte_len = layout.byte_len(pointer_bytes);

    Ok(match byte_len {
        1 => AtomicWidth::Width8,
        2 => AtomicWidth::Width16,
        4 => AtomicWidth::Width32,
        8 => AtomicWidth::Width64,
        _ => return Err(Error::invalid_instruction()),
    })
}

/// Build the VM atomic shape for one MIR atomic instruction.
fn atomic_shape_from_parts(
    access: mir::AtomicAccess,
    layout: CellLayout,
    address: AtomicAddress,
    width: AtomicWidth,
) -> AtomicShape {
    let order = AtomicOrder::from_mir(access.ordering);
    let is_signed = matches!(layout, CellLayout::Int { .. });

    AtomicShape::new(address, width, order, is_signed)
}

/// Require an atomic pointer with a concrete cell layout and address class.
fn require_atomic_pointer(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    pointer: mir::Value,
) -> Result<(CellLayout, AtomicAddress)> {
    // resolve the pointer value type
    let pointer_type = value_type_for_atomic_pointer(tree, value_types, pointer)?;
    let pointer_type = repr_type(tree, pointer_type);

    // require a concrete reference type
    let mir::Type::Reference {
        kind,
        space,
        pointee,
        ..
    } = tree.get(pointer_type)
    else {
        return Err(Error::invalid_pointer_type(format!("{pointer:?}")));
    };

    let address_space = address_space_from_reference(space.clone(), *kind);
    let address = atomic_address(address_space)?;

    // require atomic storage
    let pointee = pointee
        .ty()
        .ok_or_else(|| Error::invalid_pointer_type(format!("{pointer:?}")))?;
    let pointee = repr_type(tree, pointee);
    let mir::Type::Atomic { value } = tree.get(pointee) else {
        return Err(Error::invalid_pointer_type(format!(
            "{:?}",
            tree.get(pointee)
        )));
    };

    // require a cell-sized atomic payload
    let value = value
        .ty()
        .ok_or_else(|| Error::invalid_pointer_type(format!("{:?}", tree.get(pointee))))?;
    let layout = cell_layout_from_type(tree, value).ok_or_else(|| {
        Error::type_mismatch("cell atomic pointee", format!("{:?}", tree.get(value)))
    })?;

    Ok((layout, address))
}

/// Return the MIR type for one atomic pointer value.
fn value_type_for_atomic_pointer(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    pointer: mir::Value,
) -> Result<mir::LocalNodeId<mir::Type>> {
    value_types
        .get(pointer.0 as usize)
        .copied()
        .ok_or_else(|| Error::invalid_pointer_type(format!("{pointer:?}")))
        .map(|ty| repr_type(tree, ty))
}

/// Select the VM atomic address representation.
fn atomic_address(address_space: AddressSpace) -> Result<AtomicAddress> {
    match address_space {
        AddressSpace::Local => Ok(AtomicAddress::Heap),
        AddressSpace::Shared => Ok(AtomicAddress::SharedHeap),
        AddressSpace::Raw => Ok(AtomicAddress::Address),
        AddressSpace::Stack => Ok(AtomicAddress::Stack),
        AddressSpace::Frame => Ok(AtomicAddress::Frame),
        AddressSpace::Static => Ok(AtomicAddress::Static),
    }
}
