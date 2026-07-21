use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    AtomicAccess, AtomicOperation, CompareExchangeAccess, ExecutionScope, FenceAccess, Opcode,
    Scalar, StorageSet, ValueType,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one atomic operation.
    pub(super) fn format_atomic(
        &mut self,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> FormatResult<()> {
        self.format_atomic_results(operation, scalar)?;

        // write the operation and target address
        self.write_token("atomic.")?;
        self.write_text(operation.name())?;
        self.write_token(".")?;
        self.write_text(scalar.name())?;
        self.write_token(" ")?;
        let address = self.register_id()?;
        self.write_register(address)?;

        // write the value operand for non-load operations
        if operation != AtomicOperation::Load {
            let value = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(value)?;
        }

        // write operation specific operands and memory access
        if operation.is_compare_exchange() {
            self.format_compare_exchange_access()
        } else {
            self.format_atomic_access(operation)
        }
    }

    /// Format the logical results of one atomic operation.
    fn format_atomic_results(
        &mut self,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> FormatResult<()> {
        if operation == AtomicOperation::Store {
            return Ok(());
        }

        self.result(operation.result_type(scalar))?;
        if operation.is_compare_exchange() {
            write!(self.formatter, [token(","), space()])?;
            self.result(ValueType::scalar(Scalar::Boolean))?;
        }
        write!(self.formatter, [space(), token("="), space()])
    }

    /// Format one regular atomic memory access.
    fn format_atomic_access(&mut self, operation: AtomicOperation) -> FormatResult<()> {
        // write the timeout carried only by timed waits
        if operation == AtomicOperation::WaitTimed {
            let timeout = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(timeout)?;
        }

        // decode the common memory access
        let access = AtomicAccess::from_bits(self.u16()?).ok_or(FormatError::SyntaxError {
            message: "atomic instruction has invalid access bits",
        })?;

        // write the common memory access
        write!(self.formatter, [token(","), space()])?;
        self.write_text(access.order.name())?;
        self.format_scope(access.scope)?;

        Ok(())
    }

    /// Format one compare exchange replacement and memory access.
    fn format_compare_exchange_access(&mut self) -> FormatResult<()> {
        let replacement = self.register_id()?;
        write!(self.formatter, [token(","), space()])?;
        self.write_register(replacement)?;

        // decode the paired success and failure memory orders
        let access =
            CompareExchangeAccess::from_bits(self.u16()?).ok_or(FormatError::SyntaxError {
                message: "atomic compare exchange has invalid access bits",
            })?;
        write!(self.formatter, [token(","), space()])?;
        self.write_text(access.success.name())?;
        write!(self.formatter, [token(","), space(), token("failure(")])?;
        self.write_text(access.failure.name())?;
        self.write_token(")")?;
        self.format_scope(access.scope)
    }

    /// Format one atomic fence.
    pub(super) fn format_fence(&mut self) -> FormatResult<()> {
        let access = FenceAccess::from_bits(self.u32()?).ok_or(FormatError::SyntaxError {
            message: "atomic fence has invalid access bits",
        })?;
        write!(self.formatter, [token("atomic.fence"), space()])?;
        self.write_text(access.order.name())?;
        self.format_scope(access.scope)?;
        write!(self.formatter, [token(","), space(), token("storage(")])?;
        self.format_storage(access.storage)?;
        self.write_token(")")?;

        Ok(())
    }

    /// Format one atomic wake operation.
    pub(super) fn format_wake(&mut self, opcode: Opcode) -> FormatResult<()> {
        self.result(ValueType::scalar(Scalar::Uint64))?;
        let address = self.register_id()?;
        let name = self.fixed_name(opcode)?;
        write!(self.formatter, [space(), token("="), space()])?;
        self.write_text(name)?;
        write!(self.formatter, [space()])?;
        self.write_register(address)?;

        // write the wake count for bounded wake operations
        if opcode == Opcode::ATOMIC_WAKE {
            let count = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(count)?;
        }
        let scope =
            ExecutionScope::from_code(self.u16()? as u8).ok_or(FormatError::SyntaxError {
                message: "atomic wake has an invalid execution scope",
            })?;
        self.format_scope(scope)?;

        Ok(())
    }

    /// Append one non-system accelerated execution scope.
    fn format_scope(&mut self, scope: ExecutionScope) -> FormatResult<()> {
        // omit the default system scope
        if scope != ExecutionScope::System {
            write!(self.formatter, [token(","), space(), token("scope(")])?;
            self.write_text(scope.name())?;
            self.write_token(")")?;
        }

        Ok(())
    }

    /// Append one atomic fence storage set.
    fn format_storage(&mut self, storage: StorageSet) -> FormatResult<()> {
        let selections = [
            (StorageSet::LOCAL, "local"),
            (StorageSet::SHARED, "shared"),
            (StorageSet::FRAME, "frame"),
            (StorageSet::STATIC, "static"),
            (StorageSet::DEVICE, "device"),
            (StorageSet::WORKGROUP, "workgroup"),
        ];
        let mut is_first = true;

        // write every selected storage class in canonical order
        for (selection, name) in selections {
            if storage.0 & selection.0 == 0 {
                continue;
            }
            if !is_first {
                write!(self.formatter, [token(","), space()])?;
            }
            self.write_text(name)?;
            is_first = false;
        }

        Ok(())
    }
}
