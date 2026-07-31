use std::cmp::Ordering;
use std::{ptr, slice};

use destack_bytecode::{Instruction, MemoryOperation, Opcode, Scalar};
use destack_mir::Space;
use destack_program::{GlobalAddress, GlobalId, GlobalLocation, Runtime, Word};

use crate::diagnostic::{Error, Result};
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Construct one stable global address.
    pub(crate) fn execute_global_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let global = GlobalId(operands.u32()?);
        let reference = GlobalAddress::new(global, 0);

        self.write(target.0, reference.into());

        Ok(())
    }

    /// Construct one stable frame address.
    pub(crate) fn execute_frame_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let registers = operands.span()?;
        let frame = self.frame();
        let byte_offset = frame.range(registers) * Word::BYTE_LEN;

        self.write(target.0, Word::from_bits((byte_offset + 2) as u64));

        Ok(())
    }

    /// Materialize one relative reference as a native pointer.
    pub(crate) fn execute_pointer(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let reference = operands.register()?;
        let bits = self.read(reference.0).bits();
        let address = match instruction.opcode() {
            Opcode::POINTER_FRAME => {
                let byte_offset = (bits as usize)
                    .checked_sub(2)
                    .filter(|offset| *offset < self.machine.stack.byte_len())
                    .ok_or_else(|| self.invalid_instruction())?;

                self.machine.stack.address(byte_offset)
            }
            Opcode::POINTER_GLOBAL => self.global_pointer(GlobalAddress::from_bits(bits))?,
            Opcode::POINTER_LOCAL => {
                let edge = self.read_edge(reference, Space::Local)?;

                self.activation.memory.address(edge)
            }
            Opcode::POINTER_SHARED => {
                let edge = self.read_edge(reference, Space::Shared)?;

                self.activation.memory.address(edge)
            }
            _ => unreachable!("pointer reference dispatch selects one storage space"),
        };

        self.write(target.0, Word::from_bits(address as u64));

        Ok(())
    }

    /// Execute one native pointer arithmetic operation.
    pub(crate) fn execute_pointer_arithmetic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let left = operands.register()?;
        let left = self.read(left.0).bits();
        let value = match instruction.opcode() {
            Opcode::POINTER_ADD_IMMEDIATE => {
                let offset = operands.i32()?;

                left.wrapping_add_signed(i64::from(offset))
            }
            Opcode::POINTER_ADD => {
                let offset = operands.register()?;

                left.wrapping_add_signed(self.read(offset.0).as_i64())
            }
            Opcode::POINTER_ADD_SCALED => {
                let offset = operands.register()?;
                let stride = operands.u32()?;
                let offset = self.read(offset.0).as_i64().wrapping_mul(i64::from(stride));

                left.wrapping_add_signed(offset)
            }
            Opcode::POINTER_BYTE_OFFSET_FROM => {
                let origin = operands.register()?;

                left.wrapping_sub(self.read(origin.0).bits())
            }
            _ => unreachable!("pointer dispatch selects one pointer opcode"),
        };

        self.write(target.0, Word::from_bits(value));

        Ok(())
    }

    /// Execute one scalar memory operation.
    #[inline(always)]
    pub(crate) fn execute_memory(
        &mut self,
        instruction: Instruction<'_>,
        operation: MemoryOperation,
        scalar: Scalar,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        if operation == MemoryOperation::Load {
            let target = operands.register()?;
            let address = operands.register()?;
            let value = self.load(self.read(address.0).bits() as usize, scalar);

            self.write(target.0, value);
        } else {
            let address = operands.register()?;
            let value = operands.register()?;

            self.store(
                self.read(address.0).bits() as usize,
                scalar,
                self.read(value.0),
            );
        }

        Ok(())
    }

    /// Execute one byte-range memory operation or prefetch hint.
    pub(crate) fn execute_byte_memory(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);

        // execute the selected byte-range operation
        match instruction.opcode() {
            Opcode::COPY_BYTES | Opcode::MOVE_BYTES => {
                let target = operands.register()?;
                let source = operands.register()?;
                let byte_len = operands.register()?;
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
                let target = operands.register()?;
                let byte = operands.register()?;
                let byte_len = operands.register()?;
                let target = self.read(target.0).bits() as *mut u8;
                let byte = self.read(byte.0).bits() as u8;
                let byte_len = self.read(byte_len.0).bits() as usize;

                // SAFETY: raw byte operations require one live mutable target range
                unsafe { ptr::write_bytes(target, byte, byte_len) };
            }
            Opcode::COMPARE_BYTES => {
                let target = operands.register()?;
                let left = operands.register()?;
                let right = operands.register()?;
                let byte_len = operands.register()?;
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
                let _pointer = operands.register()?;
            }
            _ => unreachable!("byte memory dispatch selects one byte-range opcode"),
        }

        Ok(())
    }

    /// Execute one packed value load or store.
    #[inline(always)]
    pub(crate) fn execute_value_memory(&mut self, instruction: Instruction<'_>) -> Result<()> {
        match instruction.opcode() {
            Opcode::LOAD => self.execute_value_load(instruction),
            Opcode::STORE => self.execute_value_store(instruction),
            _ => unreachable!("value memory dispatch selects load or store"),
        }
    }

    /// Load one packed value from a native pointer.
    fn execute_value_load(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let address = operands.register()?;
        let byte_len = operands.u32()? as usize;
        let target = self.register_byte_range(target)?;
        if byte_len > target.len() {
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
                byte_len,
            );
        }

        Ok(())
    }

    /// Store one packed value through a native pointer.
    fn execute_value_store(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let address = operands.register()?;
        let value = operands.span()?;
        let byte_len = operands.u32()? as usize;
        let value = self.register_byte_range(value)?;
        if byte_len > value.len() {
            return Err(self.invalid_instruction());
        }
        let address = self.read(address.0).bits() as usize;

        // SAFETY: store requires a live mutable target layout and the source range was checked
        unsafe {
            ptr::copy(
                self.machine.stack.address(value.start) as *const u8,
                address as *mut u8,
                byte_len,
            );
        }

        Ok(())
    }

    /// Materialize one global reference in activation memory.
    fn global_pointer(&mut self, address: GlobalAddress) -> Result<usize> {
        let program = &self.machine.program;
        let global_id = address.global().ok_or_else(|| self.invalid_instruction())?;
        let Some(global) = program.global(global_id).copied() else {
            return Err(self.invalid_instruction());
        };
        let byte_len = global.byte_len();
        let native = match global.location {
            GlobalLocation::Constant => program.constant_address(address, byte_len),
            GlobalLocation::SharedStatic if global.is_mutable() => self
                .activation
                .memory
                .shared_static
                .address_mut(&global, address, byte_len)
                .map_err(|_| Error::memory_exhausted())?,
            GlobalLocation::LocalStatic if global.is_mutable() => self
                .activation
                .memory
                .local_static
                .address_mut(&global, address, byte_len)
                .map_err(|_| Error::memory_exhausted())?,
            GlobalLocation::SharedStatic => self
                .activation
                .memory
                .shared_static
                .address(&global, address, byte_len),
            GlobalLocation::LocalStatic => self
                .activation
                .memory
                .local_static
                .address(&global, address, byte_len),
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
