use destack_mir as mir;

use crate::program::{
    AtomicAddress, AtomicCompareExchange, AtomicOrder, AtomicReadModifyWriteOperator,
    AtomicReadModifyWriteShape, AtomicShape, AtomicWidth, Instruction, Op, PointerClass,
    WordLayout, pointer_class_from_reference, repr_type, word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::word_offset;
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
            word_offset(self, destination)?,
            word_offset(self, pointer)?,
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
            word_offset(self, pointer)?,
            word_offset(self, value)?,
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
                pointer_offset: word_offset(self, pointer)?,
                expected_offset: word_offset(self, expected)?,
                new_value_offset: word_offset(self, new_value)?,
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
                word_offset(self, destination)?,
                word_offset(self, pointer)?,
                word_offset(self, value)?,
                shape.encode(),
            ));
        }

        // encode the remaining update operator into the compact shape
        let operator = atomic_read_modify_write_operator(operator, layout)?;
        let shape = AtomicReadModifyWriteShape::new(shape, operator);

        Ok(Instruction::new(
            Op::AtomicReadModifyWrite,
            word_offset(self, destination)?,
            word_offset(self, pointer)?,
            word_offset(self, value)?,
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
        .ok_or_else(|| Error::MissingRepresentation {
            context: context.to_string(),
        })
}

/// Build the compact shape for one atomic memory operation.
fn atomic_shape(
    access: mir::AtomicAccess,
    layout: WordLayout,
    address: AtomicAddress,
    pointer_bytes: usize,
) -> Result<AtomicShape> {
    let width = atomic_width(layout, pointer_bytes)?;

    Ok(atomic_shape_from_parts(access, layout, address, width))
}

/// Map one MIR read-modify-write operator to a VM update operation.
fn atomic_read_modify_write_operator(
    operator: mir::AtomicRmwOperator,
    layout: WordLayout,
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
        _ => return Err(Error::InvalidInstruction),
    })
}

/// Return whether this layout supports integer arithmetic atomics.
fn is_integer(layout: WordLayout) -> bool {
    matches!(layout, WordLayout::Int { .. } | WordLayout::Uint { .. })
}

/// Return whether this layout supports bitwise atomics.
fn is_bits(layout: WordLayout) -> bool {
    matches!(
        layout,
        WordLayout::Bool | WordLayout::Int { .. } | WordLayout::Uint { .. }
    )
}

/// Return whether this layout supports signed min/max atomics.
fn is_signed_integer(layout: WordLayout) -> bool {
    matches!(layout, WordLayout::Int { .. })
}

/// Return whether this layout supports unsigned min/max atomics.
fn is_unsigned_integer(layout: WordLayout) -> bool {
    matches!(layout, WordLayout::Uint { .. })
}

/// Return whether this layout supports floating CAS-loop atomics.
fn is_float(layout: WordLayout) -> bool {
    matches!(layout, WordLayout::Float32 | WordLayout::Float64)
}

/// Return the atomic payload width for one word layout.
fn atomic_width(layout: WordLayout, pointer_bytes: usize) -> Result<AtomicWidth> {
    let byte_len = layout.byte_len(pointer_bytes);

    Ok(match byte_len {
        1 => AtomicWidth::Width8,
        2 => AtomicWidth::Width16,
        4 => AtomicWidth::Width32,
        8 => AtomicWidth::Width64,
        _ => return Err(Error::InvalidInstruction),
    })
}

/// Build the VM atomic shape for one MIR atomic instruction.
fn atomic_shape_from_parts(
    access: mir::AtomicAccess,
    layout: WordLayout,
    address: AtomicAddress,
    width: AtomicWidth,
) -> AtomicShape {
    let order = AtomicOrder::from_mir(access.ordering);
    let is_signed = matches!(layout, WordLayout::Int { .. });

    AtomicShape::new(address, width, order, is_signed)
}

/// Require an atomic pointer with a concrete word layout and address class.
fn require_atomic_pointer(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    pointer: mir::Value,
) -> Result<(WordLayout, AtomicAddress)> {
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
        return Err(Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        });
    };

    let pointer_class = pointer_class_from_reference(space.clone(), *kind);
    let address = atomic_address(pointer_class)?;

    // require atomic storage
    let pointee = pointee.ty().ok_or_else(|| Error::InvalidPointerType {
        actual: format!("{pointer:?}"),
    })?;
    let pointee = repr_type(tree, pointee);
    let mir::Type::Atomic { value } = tree.get(pointee) else {
        return Err(Error::InvalidPointerType {
            actual: format!("{:?}", tree.get(pointee)),
        });
    };

    // require a word-sized atomic payload
    let value = value.ty().ok_or_else(|| Error::InvalidPointerType {
        actual: format!("{:?}", tree.get(pointee)),
    })?;
    let layout = word_layout_from_type(tree, value).ok_or_else(|| Error::TypeMismatch {
        expected: "word atomic pointee".to_string(),
        actual: format!("{:?}", tree.get(value)),
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
        .ok_or_else(|| Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        })
        .map(|ty| repr_type(tree, ty))
}

/// Select the VM atomic address representation.
fn atomic_address(pointer_class: PointerClass) -> Result<AtomicAddress> {
    match pointer_class {
        PointerClass::Heap | PointerClass::HeapAddress => Ok(AtomicAddress::Heap),
        PointerClass::SharedHeap | PointerClass::SharedHeapAddress => Ok(AtomicAddress::SharedHeap),
        PointerClass::Raw => Ok(AtomicAddress::Raw),
        PointerClass::SharedRaw => Ok(AtomicAddress::SharedRaw),
        PointerClass::Stack => Ok(AtomicAddress::Stack),
        PointerClass::Frame => Ok(AtomicAddress::Frame),
        PointerClass::Static => Ok(AtomicAddress::Static),
        _ => Err(Error::InvalidInstruction),
    }
}
