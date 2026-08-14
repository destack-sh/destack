use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{
    Address, AtomicAccess, AtomicOperation, CompareExchangeAccess, ExecutionScope, FenceAccess,
    Scalar, StorageSet,
};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Format one atomic operation.
    pub(super) fn format_atomic(
        &mut self,
        operation: AtomicOperation,
        address: Address,
        scalar: Scalar,
    ) -> FormatResult<()> {
        let name = format!("atomic.{}.{}", operation.name(), scalar.name());
        self.write_opcode(&name)?;
        self.write_atomic_results(operation)?;

        // write the target address
        let register = self.register_id()?;
        self.write_address_register(address, register)?;

        // write the value operand for non-load operations
        if operation != AtomicOperation::Load {
            let value = self.register_id()?;
            write!(self.formatter, [token(","), space()])?;
            self.write_register(value)?;
        }

        // write operation specific operands and memory access
        if operation.is_compare_exchange() {
            self.format_compare_exchange_access()?;
        } else {
            self.format_atomic_access()?;
        }

        Ok(())
    }

    /// Format the logical results of one atomic operation.
    fn write_atomic_results(&mut self, operation: AtomicOperation) -> FormatResult<()> {
        if operation == AtomicOperation::Store {
            return Ok(());
        }

        self.result()?;
        if operation.is_compare_exchange() {
            self.write_comma()?;
            self.result()?;
        }
        self.write_comma()
    }

    /// Format one regular atomic memory access.
    fn format_atomic_access(&mut self) -> FormatResult<()> {
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
            (StorageSet::GLOBAL, "global"),
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
