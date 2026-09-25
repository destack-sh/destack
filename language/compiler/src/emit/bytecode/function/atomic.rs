use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one atomic load.
    pub(super) fn emit_atomic_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        let selected = self.emit_place(place, None)?;
        let scalar = self.scalar_type(destination)?;
        let opcode = self.atomic_opcode(bytecode::AtomicOperation::Load, selected.kind, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(selected.address);
        instruction.u16(self.atomic_access(access)?.bits());
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one atomic store.
    pub(super) fn emit_atomic_store(
        &mut self,
        place: &mir::Place,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        let selected = self.emit_place(place, None)?;
        let scalar = self.scalar_type(value)?;
        let opcode = self.atomic_opcode(bytecode::AtomicOperation::Store, selected.kind, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(selected.address);
        instruction.register(self.word(value)?);
        instruction.u16(self.atomic_access(access)?.bits());

        self.encode(instruction, &[])
    }

    /// Emit one atomic compare exchange.
    pub(super) fn emit_atomic_compare_exchange(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        expected: mir::Value,
        new_value: mir::Value,
        is_weak: bool,
        access: mir::CompareExchangeAccess,
    ) -> Result<(), EmitError> {
        // select the opcode for the requested exchange strength
        let selected = self.emit_place(place, None)?;
        let scalar = self.scalar_type(expected)?;
        let operation = if is_weak {
            bytecode::AtomicOperation::CompareExchangeWeak
        } else {
            bytecode::AtomicOperation::CompareExchange
        };
        let opcode = self.atomic_opcode(operation, selected.kind, scalar)?;

        // take a scratch register for each half of the result
        let old = self.scratch(bytecode::ValueType::scalar(scalar))?;
        let success = self.scratch(bytecode::ValueType::scalar(bytecode::Scalar::Boolean))?;

        // encode the exchange into both scratch registers
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(selected.address);
        instruction.register(self.word(expected)?);
        instruction.register(self.word(new_value)?);
        instruction.u16(self.compare_exchange_access(access)?.bits());
        self.encode(instruction, &[old, success])?;

        self.emit_aggregate_registers(destination, &[old, success])
    }

    /// Emit one atomic read modify write operation.
    pub(super) fn emit_atomic_rmw(
        &mut self,
        destination: mir::Value,
        operator: mir::AtomicRmwOperator,
        place: &mir::Place,
        value: mir::Value,
        access: mir::AtomicAccess,
    ) -> Result<(), EmitError> {
        // select the opcode for the requested operator
        let selected = self.emit_place(place, None)?;
        let scalar = self.scalar_type(value)?;
        let operation = match operator {
            mir::AtomicRmwOperator::Exchange => bytecode::AtomicOperation::Exchange,
            mir::AtomicRmwOperator::Add => bytecode::AtomicOperation::FetchAdd,
            mir::AtomicRmwOperator::Subtract => bytecode::AtomicOperation::FetchSubtract,
            mir::AtomicRmwOperator::And => bytecode::AtomicOperation::FetchAnd,
            mir::AtomicRmwOperator::Or => bytecode::AtomicOperation::FetchOr,
            mir::AtomicRmwOperator::Xor => bytecode::AtomicOperation::FetchXor,
            mir::AtomicRmwOperator::Min => bytecode::AtomicOperation::FetchMinimum,
            mir::AtomicRmwOperator::Max => bytecode::AtomicOperation::FetchMaximum,
        };

        // encode the operands and the access into one instruction
        let opcode = self.atomic_opcode(operation, selected.kind, scalar)?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(selected.address);
        instruction.register(self.word(value)?);
        instruction.u16(self.atomic_access(access)?.bits());
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one atomic memory fence.
    pub(super) fn emit_atomic_fence(&mut self, access: mir::FenceAccess) -> Result<(), EmitError> {
        let access = bytecode::FenceAccess {
            order: self.atomic_order(access.ordering)?,
            scope: Self::execution_scope(access.scope),
            storage: Self::storage_set(access.storage),
        };
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::ATOMIC_FENCE);
        instruction.u32(access.bits());

        self.encode(instruction, &[])
    }

    /// Return one scalar atomic opcode for an address operand.
    fn atomic_opcode(
        &self,
        operation: bytecode::AtomicOperation,
        address: bytecode::Address,
        scalar: bytecode::Scalar,
    ) -> Result<bytecode::Opcode, EmitError> {
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
    fn atomic_access(
        &self,
        access: mir::AtomicAccess,
    ) -> Result<bytecode::AtomicAccess, EmitError> {
        Ok(bytecode::AtomicAccess {
            order: self.atomic_order(access.ordering)?,
            scope: Self::execution_scope(access.scope),
        })
    }

    /// Return one encoded compare exchange access.
    fn compare_exchange_access(
        &self,
        access: mir::CompareExchangeAccess,
    ) -> Result<bytecode::CompareExchangeAccess, EmitError> {
        let failure = access.failure_ordering().map_err(|_| {
            self.internal("an atomic failure ordering parameter outside its template")
        })?;

        Ok(bytecode::CompareExchangeAccess {
            success: self.atomic_order(access.success.ordering)?,
            failure: self.atomic_order(failure)?,
            scope: Self::execution_scope(access.success.scope),
        })
    }

    /// Return one bytecode memory order.
    fn atomic_order(&self, order: mir::MemoryOrdering) -> Result<bytecode::AtomicOrder, EmitError> {
        let order = order.closed().map_err(|_| {
            self.internal("an atomic ordering parameter reached bytecode emit before instantiation")
        })?;

        Ok(match order {
            mir::MemoryOrdering::Relaxed => bytecode::AtomicOrder::Relaxed,
            mir::MemoryOrdering::Acquire => bytecode::AtomicOrder::Acquire,
            mir::MemoryOrdering::Release => bytecode::AtomicOrder::Release,
            mir::MemoryOrdering::AcquireRelease => bytecode::AtomicOrder::AcquireRelease,
            mir::MemoryOrdering::SequentiallyConsistent => {
                bytecode::AtomicOrder::SequentiallyConsistent
            }
            mir::MemoryOrdering::Parameter(_) => {
                unreachable!("a closed ordering names no parameter")
            }
        })
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
        // carry each storage flag across to its bytecode counterpart
        let mut result = bytecode::StorageSet::NONE;
        for (source, target) in [
            (mir::StorageSet::LOCAL, bytecode::StorageSet::LOCAL),
            (mir::StorageSet::SHARED, bytecode::StorageSet::SHARED),
            (mir::StorageSet::FRAME, bytecode::StorageSet::FRAME),
            (mir::StorageSet::GLOBAL, bytecode::StorageSet::GLOBAL),
        ] {
            if storage.contains(source) {
                result.0 |= target.0;
            }
        }

        result
    }
}
