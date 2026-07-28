use destack_bytecode::{AtomicOperation, CodeOffset, Instruction, MemoryOperation, Opcode, Scalar};
use destack_mir::Space;
use destack_program::{
    GlobalAddress, GlobalLocation, MemoryAccess, MemoryRange, Outcome, StopReason, Word,
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
        let mut operands = self.operands(instruction);
        let address = if operation == MemoryOperation::Load {
            let _target = operands.register()?;

            operands.register()?
        } else {
            operands.register()?
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
        let mut operands = self.operands(instruction);
        let pointer = if operation == AtomicOperation::Load {
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
        let address = self.read(pointer.0).bits() as usize;
        let byte_len = scalar.bit_width() as usize / 8;

        Ok((address, byte_len))
    }

    /// Return memory accesses performed by one byte-range operation.
    pub(crate) fn byte_accesses(
        &self,
        instruction: Instruction<'_>,
    ) -> Result<[Option<(MemoryAccess, (usize, usize))>; 2]> {
        let mut operands = self.operands(instruction);
        match instruction.opcode() {
            Opcode::COPY_BYTES | Opcode::MOVE_BYTES => {
                let target = operands.register()?;
                let source = operands.register()?;
                let byte_len = operands.register()?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let source = (self.read(source.0).bits() as usize, byte_len);
                let target = (self.read(target.0).bits() as usize, byte_len);

                Ok([
                    Some((MemoryAccess::Read, source)),
                    Some((MemoryAccess::Write, target)),
                ])
            }
            Opcode::FILL_BYTES => {
                let target = operands.register()?;
                let _byte = operands.register()?;
                let byte_len = operands.register()?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let target = (self.read(target.0).bits() as usize, byte_len);

                Ok([Some((MemoryAccess::Write, target)), None])
            }
            Opcode::COMPARE_BYTES => {
                let _target = operands.register()?;
                let left = operands.register()?;
                let right = operands.register()?;
                let byte_len = operands.register()?;
                let byte_len = self.read(byte_len.0).bits() as usize;
                let left = (self.read(left.0).bits() as usize, byte_len);
                let right = (self.read(right.0).bits() as usize, byte_len);

                Ok([
                    Some((MemoryAccess::Read, left)),
                    Some((MemoryAccess::Read, right)),
                ])
            }
            Opcode::PREFETCH_READ | Opcode::PREFETCH_WRITE => Ok([None, None]),
            _ => unreachable!("byte memory dispatch selects one byte-range opcode"),
        }
    }

    /// Stop after one memory access selected by an active watchpoint.
    pub(crate) fn watch_after(
        &mut self,
        frame: Frame,
        pc: CodeOffset,
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

        // select the first matching site and watchpoint deterministically
        for &site in sites {
            let range = if watch_points.requires_memory_range() {
                let (address, byte_len) = address.ok_or_else(Error::invalid_instruction)?;

                Some(self.memory_range(site.space, address, byte_len)?)
            } else {
                None
            };
            let Some(watchpoint_id) = watch_points.watchpoint_at(site, access, range) else {
                continue;
            };

            // retain the physical machine after the completed memory operation
            self.save_position();
            self.is_retained = true;
            let reason = StopReason::Watchpoint {
                watchpoint_id,
                point,
            };

            return Ok(Some(Outcome::Stopped { reason }));
        }

        Ok(None)
    }

    /// Project one native byte range into its program memory space.
    fn memory_range(&self, space: Space, address: usize, byte_len: usize) -> Result<MemoryRange> {
        let range = match space {
            Space::Local | Space::Shared => {
                let offset = self
                    .activation
                    .memory
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
                    .ok_or_else(|| self.invalid_instruction())?;

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
                        .constant_address(global_address, global.byte_len()),
                    GlobalLocation::SharedStatic => self.activation.memory.shared_static.address(
                        global,
                        global_address,
                        global.byte_len(),
                    ),
                    GlobalLocation::LocalStatic => self.activation.memory.local_static.address(
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

        Err(Error::invalid_instruction())
    }
}
