use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one atomic load.
    pub(super) fn emit_atomic_load(
        &mut self,
        destination: mir::Value,
        pointer: mir::Value,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        let scalar = self.scalar_type(destination)?;
        let opcode = self.atomic_opcode(bytecode::AtomicOperation::Load, pointer, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(pointer)?);
        instruction.u16(Self::atomic_access(access).bits());
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one atomic store.
    pub(super) fn emit_atomic_store(
        &mut self,
        pointer: mir::Value,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        let scalar = self.scalar_type(value)?;
        let opcode = self.atomic_opcode(bytecode::AtomicOperation::Store, pointer, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(pointer)?);
        instruction.register(self.word(value)?);
        instruction.u16(Self::atomic_access(access).bits());

        self.encode(instruction, &[])
    }

    /// Emit one atomic compare exchange.
    pub(super) fn emit_atomic_compare_exchange(
        &mut self,
        destination: mir::Value,
        pointer: mir::Value,
        expected: mir::Value,
        new_value: mir::Value,
        is_weak: bool,
        access: mir::CompareExchangeAccess,
    ) -> Result<(), EmitError> {
        let scalar = self.scalar_type(expected)?;
        let operation = if is_weak {
            bytecode::AtomicOperation::CompareExchangeWeak
        } else {
            bytecode::AtomicOperation::CompareExchange
        };
        let opcode = self.atomic_opcode(operation, pointer, scalar)?;
        let old = self.scratch(bytecode::ValueType::scalar(scalar))?;
        let success = self.scratch(bytecode::ValueType::scalar(bytecode::Scalar::Boolean))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(pointer)?);
        instruction.register(self.word(expected)?);
        instruction.register(self.word(new_value)?);
        instruction.u16(Self::compare_exchange_access(access).bits());
        self.encode(instruction, &[old, success])?;

        self.emit_aggregate_registers(destination, &[old, success])
    }

    /// Emit one atomic read modify write operation.
    pub(super) fn emit_atomic_rmw(
        &mut self,
        destination: mir::Value,
        operator: mir::AtomicRmwOperator,
        pointer: mir::Value,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        let scalar = self.scalar_type(value)?;
        let operation = match operator {
            mir::AtomicRmwOperator::Exchange => bytecode::AtomicOperation::Exchange,
            mir::AtomicRmwOperator::Add | mir::AtomicRmwOperator::Fadd => {
                bytecode::AtomicOperation::FetchAdd
            }
            mir::AtomicRmwOperator::Sub => bytecode::AtomicOperation::FetchSubtract,
            mir::AtomicRmwOperator::And => bytecode::AtomicOperation::FetchAnd,
            mir::AtomicRmwOperator::Or => bytecode::AtomicOperation::FetchOr,
            mir::AtomicRmwOperator::Xor => bytecode::AtomicOperation::FetchXor,
            mir::AtomicRmwOperator::Min
            | mir::AtomicRmwOperator::Umin
            | mir::AtomicRmwOperator::Fmin => bytecode::AtomicOperation::FetchMinimum,
            mir::AtomicRmwOperator::Max
            | mir::AtomicRmwOperator::Umax
            | mir::AtomicRmwOperator::Fmax => bytecode::AtomicOperation::FetchMaximum,
        };
        let opcode = self.atomic_opcode(operation, pointer, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(pointer)?);
        instruction.register(self.word(value)?);
        instruction.u16(Self::atomic_access(access).bits());
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one atomic memory fence.
    pub(super) fn emit_atomic_fence(&mut self, access: mir::FenceAccess) -> Result<(), EmitError> {
        let access = bytecode::FenceAccess {
            order: Self::atomic_order(access.ordering),
            scope: Self::execution_scope(access.scope),
            storage: Self::storage_set(access.storage),
        };
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::ATOMIC_FENCE);
        instruction.u32(access.bits());

        self.encode(instruction, &[])
    }

    /// Return one scalar atomic opcode for a reference operand.
    fn atomic_opcode(
        &self,
        operation: bytecode::AtomicOperation,
        pointer: mir::Value,
        scalar: bytecode::Scalar,
    ) -> Result<bytecode::Opcode, EmitError> {
        let address = self.address(pointer)?;

        bytecode::Opcode::atomic(operation, address, scalar)
            .ok_or_else(|| self.internal("invalid atomic operation"))
    }

    /// Return one MIR value's scalar representation.
    fn scalar_type(&self, value: mir::Value) -> Result<bytecode::Scalar, EmitError> {
        self.register_type(value)?
            .scalar_type()
            .ok_or_else(|| self.internal("atomic operation requires a scalar value"))
    }

    /// Return one encoded atomic access.
    fn atomic_access(access: mir::AtomicAccess) -> bytecode::AtomicAccess {
        bytecode::AtomicAccess {
            order: Self::atomic_order(access.ordering),
            scope: Self::execution_scope(access.scope),
        }
    }

    /// Return one encoded compare exchange access.
    fn compare_exchange_access(
        access: mir::CompareExchangeAccess,
    ) -> bytecode::CompareExchangeAccess {
        bytecode::CompareExchangeAccess {
            success: Self::atomic_order(access.success.ordering),
            failure: Self::atomic_order(access.failure_ordering),
            scope: Self::execution_scope(access.success.scope),
        }
    }

    /// Return one bytecode memory order.
    fn atomic_order(order: mir::MemoryOrdering) -> bytecode::AtomicOrder {
        match order {
            mir::MemoryOrdering::Relaxed => bytecode::AtomicOrder::Relaxed,
            mir::MemoryOrdering::Acquire => bytecode::AtomicOrder::Acquire,
            mir::MemoryOrdering::Release => bytecode::AtomicOrder::Release,
            mir::MemoryOrdering::AcquireRelease => bytecode::AtomicOrder::AcquireRelease,
            mir::MemoryOrdering::SequentiallyConsistent => {
                bytecode::AtomicOrder::SequentiallyConsistent
            }
        }
    }

    /// Return one bytecode execution scope.
    fn execution_scope(scope: mir::ExecutionScope) -> bytecode::ExecutionScope {
        match scope {
            mir::ExecutionScope::Invocation => bytecode::ExecutionScope::Invocation,
            mir::ExecutionScope::Subgroup => bytecode::ExecutionScope::Subgroup,
            mir::ExecutionScope::Workgroup => bytecode::ExecutionScope::Workgroup,
            mir::ExecutionScope::Device => bytecode::ExecutionScope::Device,
            mir::ExecutionScope::System => bytecode::ExecutionScope::System,
        }
    }

    /// Return one bytecode storage set.
    fn storage_set(storage: mir::StorageSet) -> bytecode::StorageSet {
        let mut result = bytecode::StorageSet::NONE;
        for (source, target) in [
            (mir::StorageSet::LOCAL, bytecode::StorageSet::LOCAL),
            (mir::StorageSet::SHARED, bytecode::StorageSet::SHARED),
            (mir::StorageSet::FRAME, bytecode::StorageSet::FRAME),
            (mir::StorageSet::GLOBAL, bytecode::StorageSet::GLOBAL),
            (mir::StorageSet::DEVICE, bytecode::StorageSet::DEVICE),
            (mir::StorageSet::WORKGROUP, bytecode::StorageSet::WORKGROUP),
        ] {
            if storage.contains(source) {
                result.0 |= target.0;
            }
        }

        result
    }
}
