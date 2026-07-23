use destack_bytecode::{AtomicOperation, CodeOffset, Instruction, MemoryOperation, Opcode, Scalar};
use destack_mir::Space;
use destack_program::{
    Continuation, GlobalAddress, GlobalLocation, MemoryAccess, MemoryRange, Outcome, StopReason,
    TypeId, Word,
};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Frame};

impl Activation<'_, '_> {
    /// Return the touched native byte range for one scalar memory operation.
    pub(crate) fn memory_address(
        &self,
        instruction: Instruction<'_>,
        operation: MemoryOperation,
        scalar: Scalar,
    ) -> Result<(usize, usize)> {
        let mut operands = instruction.operands();
        let address = if operation == MemoryOperation::Load {
            let _target = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        } else {
            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        };
        let address = self.read(address.0).bits() as usize;
        let byte_len = scalar.bit_width() as usize / 8;

        Ok((address, byte_len))
    }

    /// Return the touched native byte range for one packed value memory operation.
    pub(crate) fn value_address(
        &self,
        instruction: Instruction<'_>,
        is_load: bool,
    ) -> Result<(usize, usize)> {
        let mut operands = instruction.operands();
        let address = if is_load {
            let _target = operands.range().map_err(|_| self.invalid_instruction())?;

            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        } else {
            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        };
        if !is_load {
            let _value = operands.range().map_err(|_| self.invalid_instruction())?;
        }
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let byte_len = self
            .machine
            .program
            .layout(ty)
            .map(|layout| layout.byte_len())
            .ok_or_else(|| self.invalid_instruction())?;
        let address = self.read(address.0).bits() as usize;

        Ok((address, byte_len))
    }

    /// Return the touched native byte range for one atomic memory operation.
    pub(crate) fn atomic_address(
        &self,
        instruction: Instruction<'_>,
        operation: AtomicOperation,
        scalar: Scalar,
    ) -> Result<(usize, usize)> {
        let mut operands = instruction.operands();
        let pointer = if operation == AtomicOperation::Load {
            let _target = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        } else if operation == AtomicOperation::Store {
            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        } else if operation.is_compare_exchange() {
            let _target = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;
            let _status = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        } else {
            let _target = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            operands
                .register()
                .map_err(|_| self.invalid_instruction())?
        };
        let address = self.read(pointer.0).bits() as usize;
        let byte_len = scalar.bit_width() as usize / 8;

        Ok((address, byte_len))
    }

    /// Stop after one byte-range operation selected by an active watchpoint.
    pub(crate) fn watch_bytes_after(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let mut operands = instruction.operands();
        match instruction.opcode() {
            Opcode::COPY_BYTES | Opcode::MOVE_BYTES => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let source = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let source = (self.read(source.0).bits() as usize, byte_len);
                let target = (self.read(target.0).bits() as usize, byte_len);

                // report the source read before the target write
                if let Some(outcome) =
                    self.watch_after(frame, instruction_offset, MemoryAccess::Read, Some(source))?
                {
                    return Ok(Some(outcome));
                }

                self.watch_after(frame, instruction_offset, MemoryAccess::Write, Some(target))
            }
            Opcode::FILL_BYTES => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let _byte = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let target = (self.read(target.0).bits() as usize, byte_len);

                self.watch_after(frame, instruction_offset, MemoryAccess::Write, Some(target))
            }
            Opcode::COMPARE_BYTES => {
                let _target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let left = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let right = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let left = (self.read(left.0).bits() as usize, byte_len);
                let right = (self.read(right.0).bits() as usize, byte_len);

                // report both reads in operand order
                if let Some(outcome) =
                    self.watch_after(frame, instruction_offset, MemoryAccess::Read, Some(left))?
                {
                    return Ok(Some(outcome));
                }

                self.watch_after(frame, instruction_offset, MemoryAccess::Read, Some(right))
            }
            _ => Ok(None),
        }
    }

    /// Stop after one memory access selected by an active watchpoint.
    pub(crate) fn watch_after(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        access: MemoryAccess,
        address: Option<(usize, usize)>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let Some(watch_points) = self.watch_points else {
            return Ok(None);
        };
        let point = self.point(frame, instruction_offset)?;
        let sites = self
            .machine
            .program
            .sites()
            .memory(self.machine.program.sections(), point);

        // select the first matching site and watchpoint deterministically
        for &site in sites {
            let range = if watch_points.requires_memory_range() {
                let (address, byte_len) = address.ok_or_else(|| {
                    Error::invalid_instruction(frame.function, instruction_offset)
                })?;

                Some(self.memory_range(site.space, address, byte_len)?)
            } else {
                None
            };
            let Some(watchpoint_id) = watch_points.watchpoint_at(site, access, range) else {
                continue;
            };

            // retain the state after the completed memory operation
            let frame = self.frame();
            let next = self.point(frame, frame.code_offset)?;
            let frame_state = self
                .machine
                .program
                .frame_state_at(next)
                .ok_or_else(Error::invalid_continuation)?;
            let continuation = self.capture(frame_state)?;
            let reason = StopReason::Watchpoint {
                watchpoint_id,
                point,
            };

            return Ok(Some(Outcome::Stopped {
                continuation,
                reason,
            }));
        }

        Ok(None)
    }

    /// Project one native byte range into its program memory space.
    fn memory_range(&self, space: Space, address: usize, byte_len: usize) -> Result<MemoryRange> {
        let range = match space {
            Space::Local | Space::Shared => {
                let offset = self
                    .call
                    .storage
                    .heap_offset(space, address)
                    .ok_or_else(|| self.invalid_instruction())?;

                if space == Space::Local {
                    MemoryRange::local_heap(offset as u64, byte_len as u64)
                } else {
                    MemoryRange::shared_heap(offset as u64, byte_len as u64)
                }
            }
            Space::Frame => {
                let offset = self
                    .machine
                    .stack
                    .byte_offset(address)
                    .ok_or_else(Error::invalid_continuation)?;

                MemoryRange::frame(offset as u64, byte_len as u64)
            }
            Space::Static => self.static_range(address, byte_len)?,
        };

        Ok(range)
    }

    /// Resolve one native static byte range to its durable global coordinate.
    fn static_range(&self, address: usize, byte_len: usize) -> Result<MemoryRange> {
        let locations = [
            GlobalLocation::Constant,
            GlobalLocation::SharedStatic,
            GlobalLocation::LocalStatic,
        ];

        // scan cold debugger metadata only when a range watchpoint is active
        for location in locations {
            for (global_id, global) in self.machine.program.globals(location) {
                let global_address = GlobalAddress::new(global_id, 0);
                let base = match location {
                    GlobalLocation::Constant => self
                        .machine
                        .program
                        .constant_native_address(global_address, global.byte_len()),
                    GlobalLocation::SharedStatic => self.call.storage.shared_static.native_address(
                        global,
                        global_address,
                        global.byte_len(),
                    ),
                    GlobalLocation::LocalStatic => self.call.storage.local_static.native_address(
                        global,
                        global_address,
                        global.byte_len(),
                    ),
                };
                let Some(base) = base else {
                    continue;
                };
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
        }

        let frame = self.frame();

        Err(Error::invalid_instruction(
            frame.function,
            frame.code_offset,
        ))
    }
}
