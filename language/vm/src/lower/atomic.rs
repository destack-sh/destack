use destack_mir as mir;

use crate::program::{
    AtomicCompareExchange, AtomicFence, AtomicLoad, AtomicRmw, AtomicStore, Instruction, Op,
    WordLayout, word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::raw_pointee_type_for_value;

impl<'a> BlockLowerer<'a> {
    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load pointer".to_string(),
            })?;

        let (layout, byte_len) = require_atomic_layout(self.tree, self.value_type(), pointer)?;

        Ok(pool.instruction_with_side(
            Op::AtomicLoad,
            AtomicLoad {
                dest_offset: word_offset(self, destination)?,
                pointer_offset: word_offset(self, pointer)?,
                layout,
                byte_len,
                ordering,
                scope,
                memory_scope,
                semantics,
            },
        ))
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
        pool: &mut Pool<'_, '_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> Result<Instruction> {
        // require SSA values
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic store value".to_string(),
        })?;

        let (layout, byte_len) = require_atomic_layout(self.tree, self.value_type(), pointer)?;

        Ok(pool.instruction_with_side(
            Op::AtomicStore,
            AtomicStore {
                pointer_offset: word_offset(self, pointer)?,
                value_offset: word_offset(self, value)?,
                layout,
                byte_len,
                ordering,
                scope,
                memory_scope,
                semantics,
            },
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
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange pointer".to_string(),
            })?;
        let expected = expected
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange expected".to_string(),
            })?;
        let new_value = new_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange new value".to_string(),
            })?;

        let (layout, byte_len) = require_atomic_layout(self.tree, self.value_type(), pointer)?;

        Ok(pool.instruction_with_side(
            Op::AtomicCompareExchange,
            AtomicCompareExchange {
                dest: destination,
                pointer_offset: word_offset(self, pointer)?,
                expected_offset: word_offset(self, expected)?,
                new_value_offset: word_offset(self, new_value)?,
                layout,
                byte_len,
                is_weak,
                ordering,
                scope,
                memory_scope,
                semantics,
            },
        ))
    }

    /// Lower one atomic read-modify-write.
    pub(super) fn lower_atomic_rmw(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        operator: mir::AtomicRmwOperator,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic rmw value".to_string(),
        })?;

        let (layout, byte_len) = require_atomic_layout(self.tree, self.value_type(), pointer)?;

        Ok(pool.instruction_with_side(
            atomic_rmw_op(operator),
            AtomicRmw {
                dest_offset: word_offset(self, destination)?,
                pointer_offset: word_offset(self, pointer)?,
                value_offset: word_offset(self, value)?,
                layout,
                byte_len,
                ordering,
                scope,
                memory_scope,
                semantics,
            },
        ))
    }

    /// Lower one atomic fence.
    pub(super) fn lower_atomic_fence(
        &self,
        pool: &mut Pool<'_, '_>,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> Instruction {
        pool.instruction_with_side(
            Op::AtomicFence,
            AtomicFence {
                ordering,
                scope,
                memory_scope,
                semantics,
            },
        )
    }
}

/// Select one atomic read modify write operation.
fn atomic_rmw_op(operator: mir::AtomicRmwOperator) -> Op {
    match operator {
        mir::AtomicRmwOperator::Exchange => Op::AtomicExchange,
        mir::AtomicRmwOperator::Add => Op::AtomicAdd,
        mir::AtomicRmwOperator::Sub => Op::AtomicSub,
        mir::AtomicRmwOperator::And => Op::AtomicAnd,
        mir::AtomicRmwOperator::Or => Op::AtomicOr,
        mir::AtomicRmwOperator::Xor => Op::AtomicXor,
        mir::AtomicRmwOperator::Min => Op::AtomicMin,
        mir::AtomicRmwOperator::Max => Op::AtomicMax,
        mir::AtomicRmwOperator::Umin => Op::AtomicUmin,
        mir::AtomicRmwOperator::Umax => Op::AtomicUmax,
        mir::AtomicRmwOperator::Fadd => Op::AtomicFadd,
        mir::AtomicRmwOperator::Fmin => Op::AtomicFmin,
        mir::AtomicRmwOperator::Fmax => Op::AtomicFmax,
    }
}

/// Require an atomic pointer with a concrete word layout.
fn require_atomic_layout(
    tree: &mir::Tree,
    value_types: &[mir::LocalNodeId<mir::Type>],
    pointer: mir::Value,
) -> Result<(WordLayout, usize)> {
    // atomics operate over raw memory in the VM interpreter
    let pointee = raw_pointee_type_for_value(tree, value_types, pointer).ok_or_else(|| {
        Error::InvalidPointerType {
            actual: format!("{pointer:?}"),
        }
    })?;

    // compile the memory representation once
    let layout = word_layout_from_type(tree, pointee).ok_or_else(|| Error::TypeMismatch {
        expected: "word atomic pointee".to_string(),
        actual: format!("{:?}", tree.get(pointee)),
    })?;
    let byte_len = layout.byte_len(tree.pointer_bytes() as usize);

    Ok((layout, byte_len))
}
