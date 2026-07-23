use std::cmp::Ordering;
use std::{ptr, slice};

use destack_bytecode::{CodeOffset, Instruction, MemoryOperation, Opcode, Scalar};
use destack_program::{
    Continuation, FrameSlotId, GlobalAddress, GlobalId, GlobalLocation, MemoryAccess, Outcome,
    TypeId, Word,
};

use crate::diagnostic::{Error, Result};
use crate::machine::{Activation, Frame};

impl Activation<'_, '_> {
    /// Materialize one linked global's native address.
    pub(crate) fn execute_global_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let global = GlobalId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let address = self.global_address(global)?;

        self.write(target.0, Word::from_bits(address as u64));

        Ok(())
    }

    /// Materialize one active frame slot's native address.
    pub(crate) fn execute_frame_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let slot = operands.u32().map_err(|_| self.invalid_instruction())?;
        let frame = self.frame();
        let program = &self.machine.program;
        let layout = program
            .frame_layout(frame.function)
            .ok_or_else(|| self.invalid_instruction())?;
        let slot = program
            .frame_slot(layout, FrameSlotId(slot))
            .ok_or_else(|| self.invalid_instruction())?;
        let address = self.machine.stack.address(frame.slot(slot.offset));

        self.write(target.0, Word::from_bits(address as u64));

        Ok(())
    }

    /// Move one value between registers and canonical frame storage.
    pub(crate) fn execute_frame_memory(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let frame = self.frame();
        let mut operands = instruction.operands();

        // decode the register range and frame slot in operation order
        let (registers, slot) = match instruction.opcode() {
            Opcode::FRAME_LOAD => {
                let registers = operands.range().map_err(|_| self.invalid_instruction())?;
                let slot = operands.u32().map_err(|_| self.invalid_instruction())?;

                (registers, FrameSlotId(slot))
            }
            Opcode::FRAME_STORE => {
                let slot = operands.u32().map_err(|_| self.invalid_instruction())?;
                let registers = operands.range().map_err(|_| self.invalid_instruction())?;

                (registers, FrameSlotId(slot))
            }
            _ => unreachable!("frame memory dispatch selects one frame memory opcode"),
        };
        let layout = self
            .machine
            .program
            .frame_layout(frame.function)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let slot = self
            .machine
            .program
            .frame_slot(&layout, slot)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let registers = self.register_byte_range(registers)?;
        if slot.byte_len() as usize > registers.len() {
            return Err(self.invalid_instruction());
        }
        let frame_offset = frame.slot(slot.offset);

        // initialize register padding or update the canonical frame bytes
        match instruction.opcode() {
            Opcode::FRAME_LOAD => {
                self.machine.stack.zero(registers.start, registers.len())?;
                self.machine.stack.move_bytes(
                    frame_offset,
                    registers.start,
                    slot.byte_len() as usize,
                );
            }
            Opcode::FRAME_STORE => self.machine.stack.move_bytes(
                registers.start,
                frame_offset,
                slot.byte_len() as usize,
            ),
            _ => unreachable!("frame memory dispatch selects one frame memory opcode"),
        }

        Ok(())
    }

    /// Resolve one stable heap reference into an ephemeral native pointer.
    pub(crate) fn execute_reference_pointer(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let reference = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let representation = operands
            .reference()
            .map_err(|_| self.invalid_instruction())?;
        let edge = self.read_reference_edge(reference, representation)?;
        let address = self.call.storage.native_address(edge);

        self.write(target.0, Word::from_bits(address as u64));

        Ok(())
    }

    /// Execute one native pointer calculation.
    pub(crate) fn execute_pointer(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let left = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let left = self.read(left.0).bits();
        let value = match instruction.opcode() {
            Opcode::POINTER_OFFSET => {
                let offset = operands.i32().map_err(|_| self.invalid_instruction())?;

                left.wrapping_add_signed(i64::from(offset))
            }
            Opcode::POINTER_INDEX => {
                let index = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let stride = operands.u32().map_err(|_| self.invalid_instruction())?;
                let offset = self.read(index.0).bits().wrapping_mul(u64::from(stride));

                left.wrapping_add(offset)
            }
            Opcode::POINTER_DISTANCE => {
                let right = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;

                left.wrapping_sub(self.read(right.0).bits())
            }
            _ => unreachable!("pointer dispatch selects one pointer opcode"),
        };

        self.write(target.0, Word::from_bits(value));

        Ok(())
    }

    /// Execute one scalar memory operation.
    pub(crate) fn execute_memory<const WATCH: bool>(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
        operation: MemoryOperation,
        scalar: Scalar,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let needs_range = WATCH
            && self
                .watch_points
                .is_some_and(|points| points.requires_memory_range());
        let address = if needs_range {
            Some(self.memory_address(instruction, operation, scalar)?)
        } else {
            None
        };

        // execute one scalar load or store
        let mut operands = instruction.operands();
        if operation == MemoryOperation::Load {
            let target = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;
            let address = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;
            let value = self.load(self.read(address.0).bits() as usize, scalar);

            self.write(target.0, value);
        } else {
            let address = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;
            let value = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;

            self.store(
                self.read(address.0).bits() as usize,
                scalar,
                self.read(value.0),
            );
        }

        // report the completed access only in the observed loop
        let access = match operation {
            MemoryOperation::Load => MemoryAccess::Read,
            MemoryOperation::Store => MemoryAccess::Write,
        };
        if WATCH {
            self.watch_after(frame, instruction_offset, access, address)
        } else {
            Ok(None)
        }
    }

    /// Execute one byte-range memory operation or prefetch hint.
    pub(crate) fn execute_byte_memory<const WATCH: bool>(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let mut operands = instruction.operands();

        // execute the selected byte-range operation
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
                let target = self.read(target.0).bits() as *mut u8;
                let source = self.read(source.0).bits() as *const u8;
                let byte_len = self.read(byte_len.0).bits() as usize;

                // SAFETY: raw byte operations require live ranges for the encoded byte length
                unsafe {
                    if instruction.opcode() == Opcode::COPY_BYTES {
                        ptr::copy_nonoverlapping(source, target, byte_len);
                    } else {
                        ptr::copy(source, target, byte_len);
                    }
                }
            }
            Opcode::FILL_BYTES => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let byte_len = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let target = self.read(target.0).bits() as *mut u8;
                let byte = self.read(byte.0).bits() as u8;
                let byte_len = self.read(byte_len.0).bits() as usize;

                // SAFETY: raw byte operations require one live mutable target range
                unsafe { ptr::write_bytes(target, byte, byte_len) };
            }
            Opcode::COMPARE_BYTES => {
                let target = operands
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
                let order = if byte_len == 0 {
                    Ordering::Equal
                } else {
                    // SAFETY: raw byte comparison requires two live ranges of the encoded length
                    let left = unsafe {
                        slice::from_raw_parts(self.read(left.0).bits() as *const u8, byte_len)
                    };
                    let right = unsafe {
                        slice::from_raw_parts(self.read(right.0).bits() as *const u8, byte_len)
                    };

                    left.cmp(right)
                };
                let value = match order {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                };

                self.write(target.0, Word::int32(value));
            }
            Opcode::PREFETCH_READ | Opcode::PREFETCH_WRITE => {
                let _pointer = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
            }
            _ => unreachable!("byte memory dispatch selects one byte-range opcode"),
        }

        // report completed byte accesses only in the observed loop
        if WATCH {
            self.watch_bytes_after(frame, instruction_offset, instruction)
        } else {
            Ok(None)
        }
    }

    /// Execute one packed value load or store and apply active watchpoints.
    pub(crate) fn execute_value_memory<const WATCH: bool>(
        &mut self,
        frame: Frame,
        instruction_offset: CodeOffset,
        instruction: Instruction<'_>,
    ) -> Result<Option<Outcome<Continuation, Vec<Word>>>> {
        let access = match instruction.opcode() {
            Opcode::LOAD => MemoryAccess::Read,
            Opcode::STORE => MemoryAccess::Write,
            _ => unreachable!("value memory dispatch selects load or store"),
        };
        let needs_range = WATCH
            && self
                .watch_points
                .is_some_and(|points| points.requires_memory_range());
        let address = if needs_range {
            Some(self.value_address(instruction, access == MemoryAccess::Read)?)
        } else {
            None
        };

        // execute the selected packed value operation
        match access {
            MemoryAccess::Read => self.execute_value_load(instruction)?,
            MemoryAccess::Write => self.execute_value_store(instruction)?,
            MemoryAccess::ReadWrite => unreachable!("value memory operations are directional"),
        }

        // report the completed access only in the observed loop
        if WATCH {
            self.watch_after(frame, instruction_offset, access, address)
        } else {
            Ok(None)
        }
    }

    /// Load one packed value from a native pointer.
    fn execute_value_load(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands.range().map_err(|_| self.invalid_instruction())?;
        let address = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let layout = self
            .machine
            .program
            .layout(ty)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let target = self.register_byte_range(target)?;
        if layout.byte_len() > target.len() {
            return Err(self.invalid_instruction());
        }
        let address = self.read(address.0).bits() as usize;

        // clear register padding before loading exact layout bytes
        self.machine.stack.zero(target.start, target.len())?;

        // SAFETY: load requires a live source layout and the target range was checked above
        unsafe {
            ptr::copy(
                address as *const u8,
                self.machine.stack.address(target.start) as *mut u8,
                layout.byte_len(),
            );
        }

        Ok(())
    }

    /// Store one packed value through a native pointer.
    fn execute_value_store(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let address = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let value = operands.range().map_err(|_| self.invalid_instruction())?;
        let ty = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let layout = self
            .machine
            .program
            .layout(ty)
            .copied()
            .ok_or_else(|| self.invalid_instruction())?;
        let value = self.register_byte_range(value)?;
        if layout.byte_len() > value.len() {
            return Err(self.invalid_instruction());
        }
        let address = self.read(address.0).bits() as usize;

        // SAFETY: store requires a live mutable target layout and the source range was checked
        unsafe {
            ptr::copy(
                self.machine.stack.address(value.start) as *const u8,
                address as *mut u8,
                layout.byte_len(),
            );
        }

        Ok(())
    }

    /// Resolve one linked global into activation memory.
    fn global_address(&mut self, global_id: GlobalId) -> Result<usize> {
        let program = &self.machine.program;
        let Some(global) = program.global(global_id).copied() else {
            return Err(self.invalid_instruction());
        };
        let address = GlobalAddress::new(global_id, 0);
        let byte_len = global.byte_len();
        let native = match global.location {
            GlobalLocation::Constant => program.constant_native_address(address, byte_len),
            GlobalLocation::SharedStatic if global.is_mutable() => self
                .call
                .storage
                .shared_static
                .native_address_mut(&global, address, byte_len)
                .map_err(|_| Error::memory_exhausted())?,
            GlobalLocation::LocalStatic if global.is_mutable() => self
                .call
                .storage
                .local_static
                .native_address_mut(&global, address, byte_len)
                .map_err(|_| Error::memory_exhausted())?,
            GlobalLocation::SharedStatic => self
                .call
                .storage
                .shared_static
                .native_address(&global, address, byte_len),
            GlobalLocation::LocalStatic => self
                .call
                .storage
                .local_static
                .native_address(&global, address, byte_len),
        };

        native.ok_or_else(|| self.invalid_instruction())
    }

    /// Load one scalar from a valid native address.
    #[inline(always)]
    pub(super) fn load(&self, address: usize, scalar: Scalar) -> Word {
        // SAFETY: scalar loads require a live native address for the selected scalar width
        let bits = unsafe {
            match scalar.bit_width() {
                8 => u64::from(ptr::read_unaligned(address as *const u8)),
                16 => u64::from(ptr::read_unaligned(address as *const u16)),
                32 => u64::from(ptr::read_unaligned(address as *const u32)),
                64 => ptr::read_unaligned(address as *const u64),
                _ => unreachable!("scalar memory operations use one bytecode word"),
            }
        };

        Word::from_bits(scalar.encode(bits))
    }

    /// Store one scalar at a valid mutable native address.
    #[inline(always)]
    pub(super) fn store(&self, address: usize, scalar: Scalar, value: Word) {
        let bits = value.bits();

        // SAFETY: scalar stores require a live mutable address for the selected scalar width
        unsafe {
            match scalar.bit_width() {
                8 => ptr::write_unaligned(address as *mut u8, bits as u8),
                16 => ptr::write_unaligned(address as *mut u16, bits as u16),
                32 => ptr::write_unaligned(address as *mut u32, bits as u32),
                64 => ptr::write_unaligned(address as *mut u64, bits),
                _ => unreachable!("scalar memory operations use one bytecode word"),
            }
        }
    }
}
