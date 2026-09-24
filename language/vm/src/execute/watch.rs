use destack_bytecode::{
    Address, AtomicOperation, CodeOffset, Instruction, MemoryOperation, Scalar,
};
use destack_mir::{Space, Storage};
use destack_program::{
    GlobalLocation, MemoryAccess, MemoryRange, Outcome, Runtime, StopReason, Word,
};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Frame};

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Return the touched native byte range for one scalar memory operation.
    pub(crate) fn memory_address(
        &self,
        instruction: Instruction<'_>,
        operation: MemoryOperation,
        mode: Address,
        scalar: Scalar,
    ) -> Result<(usize, usize)> {
        let mut operands = self.operands(instruction);
        let address = if operation == MemoryOperation::Load {
            let _target = operands.register()?;

            operands.register()?
        } else {
            operands.register()?
        };
        let byte_len = scalar.bit_width() as usize / 8;
        let address = self.resolve_address(address.0, mode)?;

        Ok((address, byte_len))
    }

    /// Return the touched native byte range for one packed value memory operation.
    pub(crate) fn value_address(
        &self,
        instruction: Instruction<'_>,
        is_load: bool,
    ) -> Result<(usize, usize)> {
        let mut operands = self.operands(instruction);
        let address = if is_load {
            let _target = operands.span()?;

            operands.register()?
        } else {
            operands.register()?
        };
        if !is_load {
            let _value = operands.span()?;
        }
        let byte_len = operands.u32()? as usize;
        let mode = instruction
            .opcode()
            .memory_range_operation()
            .map(|(_, address, _)| address)
            .unwrap_or_else(|| unreachable!("value memory dispatch selects one address"));
        let address = self.resolve_address(address.0, mode)?;

        Ok((address, byte_len))
    }

    /// Return the touched native byte range for one atomic memory operation.
    pub(crate) fn atomic_address(
        &self,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        mode: Address,
        scalar: Scalar,
    ) -> Result<(usize, usize)> {
        let mut operands = self.operands(instruction);
        let register = if operation == AtomicOperation::Load {
            let _target = operands.register()?;

            operands.register()?
        } else if operation == AtomicOperation::Store {
            operands.register()?
        } else if operation.is_compare_exchange() {
            let _target = operands.register()?;
            let _status = operands.register()?;

            operands.register()?
        } else {
            let _target = operands.register()?;

            operands.register()?
        };
        let byte_len = scalar.bit_width() as usize / 8;
        let address = self.resolve_address(register.0, mode)?;

        Ok((address, byte_len))
    }

    /// Return memory accesses performed by one byte-range operation.
    pub(crate) fn byte_accesses(
        &self,
        instruction: Instruction<'_>,
    ) -> Result<[Option<(MemoryAccess, (usize, usize))>; 2]> {
        let opcode = instruction.opcode();
        let mut operands = self.operands(instruction);
        if let Some((_, target_mode, source_mode, is_immediate)) = opcode.transfer_operation() {
            let target = operands.register()?;
            let source = operands.register()?;
            let byte_len = if is_immediate {
                operands.u32()? as usize
            } else {
                let byte_len = operands.register()?;

                self.read(byte_len.0).bits() as usize
            };
            let source = self.resolve_address(source.0, source_mode)?;
            let target = self.resolve_address(target.0, target_mode)?;

            Ok([
                Some((MemoryAccess::Read, (source, byte_len))),
                Some((MemoryAccess::Write, (target, byte_len))),
            ])
        } else if let Some((target_mode, is_immediate)) = opcode.fill_operation() {
            let target = operands.register()?;
            let _byte = operands.register()?;
            let byte_len = if is_immediate {
                operands.u32()? as usize
            } else {
                let byte_len = operands.register()?;

                self.read(byte_len.0).bits() as usize
            };
            let target = self.resolve_address(target.0, target_mode)?;

            Ok([Some((MemoryAccess::Write, (target, byte_len))), None])
        } else if let Some((left_mode, right_mode, is_immediate)) = opcode.compare_operation() {
            let _target = operands.register()?;
            let left = operands.register()?;
            let right = operands.register()?;
            let byte_len = if is_immediate {
                operands.u32()? as usize
            } else {
                let byte_len = operands.register()?;

                self.read(byte_len.0).bits() as usize
            };
            let left = self.resolve_address(left.0, left_mode)?;
            let right = self.resolve_address(right.0, right_mode)?;

            Ok([
                Some((MemoryAccess::Read, (left, byte_len))),
                Some((MemoryAccess::Read, (right, byte_len))),
            ])
        } else if opcode.prefetch_operation().is_some() {
            Ok([None, None])
        } else {
            unreachable!("byte memory dispatch selects one byte range opcode")
        }
    }

    /// Stop after one memory access selected by an active watchpoint.
    pub(crate) fn watch_after(
        &mut self,
        frame: Frame,
        pc: CodeOffset,
        site_index: usize,
        access: MemoryAccess,
        address: Option<(usize, usize)>,
    ) -> Result<Option<Outcome<Vec<Word>>>> {
        let Some(watch_points) = self.watch_points else {
            return Ok(None);
        };
        let point = self.point(frame, pc)?;
        let sites = self
            .machine
            .program
            .sites()
            .memory(self.machine.program.sections(), point);
        let site = sites
            .get(site_index)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        if site.access != access {
            return Err(self.invalid_instruction());
        }

        // resolve the range only when one configured watchpoint requires it
        let range = if watch_points.requires_memory_range() {
            let (address, byte_len) = address.ok_or_else(Error::invalid_instruction)?;

            Some(self.executed_memory_range(site.storage.get(), address, byte_len)?)
        } else {
            None
        };
        let Some(watchpoint_id) = watch_points.watchpoint_at(site, access, range) else {
            return Ok(None);
        };

        // retain the stopped frames in place for later continuation
        self.retain_stop(frame.pc);
        let reason = StopReason::Watchpoint {
            watchpoint_id,
            point,
        };

        Ok(Some(Outcome::Stopped { reason }))
    }

    /// Project one native byte range into its program storage.
    pub(crate) fn executed_memory_range(
        &self,
        storage: Option<Storage>,
        address: usize,
        byte_len: usize,
    ) -> Result<MemoryRange> {
        let range = match storage {
            Some(Storage::Heap(Space::Local) | Storage::Heap(Space::Shared)) => {
                let offset = self
                    .activation
                    .memory
                    .heap_offset(address)
                    .ok_or_else(|| self.invalid_instruction())?;

                if storage == Some(Storage::Heap(Space::Local)) {
                    MemoryRange::local_heap(offset as u64, byte_len as u64)
                } else {
                    MemoryRange::shared_heap(offset as u64, byte_len as u64)
                }
            }
            Some(Storage::Frame) => {
                let offset = self
                    .fiber
                    .stack
                    .byte_offset(address)
                    .ok_or_else(|| self.invalid_instruction())?;

                MemoryRange::frame(offset as u64, byte_len as u64)
            }
            Some(
                storage @ (Storage::Static(Space::Constant)
                | Storage::Static(Space::Local)
                | Storage::Static(Space::Shared)),
            ) => self.global_range(storage, address, byte_len)?,
            Some(Storage::Heap(Space::Constant)) => return Err(self.invalid_instruction()),
            None => MemoryRange::address(address as u64, byte_len as u64),
        };

        Ok(range)
    }

    /// Resolve one native global byte range to its durable location.
    fn global_range(
        &self,
        storage: Storage,
        address: usize,
        byte_len: usize,
    ) -> Result<MemoryRange> {
        let location = match storage {
            Storage::Static(Space::Constant) => GlobalLocation::Constant,
            Storage::Static(Space::Local) => GlobalLocation::LocalStatic,
            Storage::Static(Space::Shared) => GlobalLocation::SharedStatic,
            Storage::Heap(_) | Storage::Frame => {
                return Err(self.invalid_instruction());
            }
        };

        // scan cold debugger metadata only when a range watchpoint is active
        for (global_id, global) in self.machine.program.globals(location) {
            let reference = self.activation.memory.reference(global);
            let reference = reference
                .offset()
                .ok_or_else(|| self.invalid_instruction())?;
            let base = self.activation.memory.base_address() + reference;
            let Some(offset) = address.checked_sub(base) else {
                continue;
            };
            if offset + byte_len <= global.byte_len() {
                return Ok(MemoryRange::global(
                    global_id,
                    location,
                    offset as u64,
                    byte_len as u64,
                ));
            }
        }

        Err(Error::invalid_instruction())
    }
}
