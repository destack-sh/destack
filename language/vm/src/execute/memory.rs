use std::cmp::Ordering;
use std::{ptr, slice};

use destack_bytecode::{Address, Instruction, MemoryOperation, Opcode, Prefetch, Scalar, Transfer};
use destack_program::{GlobalId, Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Construct one stable global address.
    pub(crate) fn execute_global_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let global = GlobalId(operands.u32()?);
        let global = self
            .machine
            .program
            .global(global)
            .ok_or_else(|| self.invalid_instruction())?;
        let reference = self.activation.memory.reference(global);

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
        let reference = self.fiber.stack.memory_offset(byte_offset);

        self.write(target.0, Word::from_bits(reference as u64));

        Ok(())
    }

    /// Execute one reference or native pointer arithmetic operation, or a rebase between them.
    pub(crate) fn execute_address_arithmetic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let left = operands.register()?;
        let left = self.read(left.0).bits();
        let value = match instruction.opcode() {
            Opcode::ADDRESS_ADD_IMMEDIATE => {
                let offset = operands.i32()?;

                left.wrapping_add_signed(i64::from(offset))
            }
            Opcode::ADDRESS_ADD => {
                let offset = operands.register()?;

                left.wrapping_add_signed(self.read(offset.0).as_i64())
            }
            Opcode::ADDRESS_ADD_SCALED => {
                let offset = operands.register()?;
                let stride = operands.u32()?;
                let offset = self.read(offset.0).as_i64().wrapping_mul(i64::from(stride));

                left.wrapping_add_signed(offset)
            }
            Opcode::ADDRESS_DIFF => {
                let origin = operands.register()?;

                left.wrapping_sub(self.read(origin.0).bits())
            }
            Opcode::ADDRESS_POINTER => {
                left.wrapping_add(self.activation.memory.base_address() as u64)
            }
            Opcode::ADDRESS_REFERENCE => {
                left.wrapping_sub(self.activation.memory.base_address() as u64)
            }
            _ => unreachable!("address dispatch selects one arithmetic opcode"),
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
        mode: Address,
        scalar: Scalar,
        is_volatile: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        if operation == MemoryOperation::Load {
            let target = operands.register()?;
            let register = operands.register()?;
            let address = self.resolve_address(register.0, mode)?;
            let value = if is_volatile {
                self.load_volatile(address, scalar)
            } else {
                self.load(address, scalar)
            };

            self.write(target.0, value);
        } else {
            let register = operands.register()?;
            let value = operands.register()?;
            let address = self.resolve_address(register.0, mode)?;

            if is_volatile {
                self.store_volatile(address, scalar, self.read(value.0));
            } else {
                self.store(address, scalar, self.read(value.0));
            }
        }

        Ok(())
    }

    /// Execute one byte-range memory operation or prefetch hint.
    pub(crate) fn execute_transfer(
        &mut self,
        instruction: Instruction<'_>,
        operation: Transfer,
        target_mode: Address,
        source_mode: Address,
        is_immediate: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let source = operands.register()?;
        let byte_len = if is_immediate {
            operands.u32()? as usize
        } else {
            let byte_len = operands.register()?;

            self.read(byte_len.0).bits() as usize
        };
        let target = self.resolve_address(target.0, target_mode)? as *mut u8;
        let source = self.resolve_address(source.0, source_mode)? as *const u8;

        // SAFETY: bytecode range operations require live ranges for the encoded byte length
        unsafe {
            match operation {
                Transfer::Copy => ptr::copy_nonoverlapping(source, target, byte_len),
                Transfer::Move => ptr::copy(source, target, byte_len),
            }
        }

        Ok(())
    }

    /// Fill one byte range through one exact address representation.
    pub(crate) fn execute_fill(
        &mut self,
        instruction: Instruction<'_>,
        target_mode: Address,
        is_immediate: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let byte = operands.register()?;
        let byte_len = if is_immediate {
            operands.u32()? as usize
        } else {
            let byte_len = operands.register()?;

            self.read(byte_len.0).bits() as usize
        };
        let target = self.resolve_address(target.0, target_mode)? as *mut u8;
        let byte = self.read(byte.0).bits() as u8;

        // SAFETY: bytecode fill requires one live mutable target range
        unsafe { ptr::write_bytes(target, byte, byte_len) };

        Ok(())
    }

    /// Compare two byte ranges through exact address representations.
    pub(crate) fn execute_compare(
        &mut self,
        instruction: Instruction<'_>,
        left_mode: Address,
        right_mode: Address,
        is_immediate: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
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
        let order = if byte_len == 0 {
            Ordering::Equal
        } else {
            // SAFETY: bytecode comparison requires two live ranges of the encoded length
            let left = unsafe { slice::from_raw_parts(left as *const u8, byte_len) };
            let right = unsafe { slice::from_raw_parts(right as *const u8, byte_len) };

            left.cmp(right)
        };
        let value = match order {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };

        self.write(target.0, Word::int32(value));

        Ok(())
    }

    /// Consume one memory prefetch hint.
    pub(crate) fn execute_prefetch(
        &mut self,
        instruction: Instruction<'_>,
        _operation: Prefetch,
        _address: Address,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let _register = operands.register()?;

        Ok(())
    }

    /// Execute one packed value load or store.
    #[inline(always)]
    pub(crate) fn execute_value_memory(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let Some((operation, address, is_volatile)) = instruction.opcode().memory_range_operation()
        else {
            unreachable!("value memory dispatch selects load or store");
        };

        match operation {
            MemoryOperation::Load => self.execute_value_load(instruction, address, is_volatile),
            MemoryOperation::Store => self.execute_value_store(instruction, address, is_volatile),
        }
    }

    /// Load one packed value through the selected address representation.
    fn execute_value_load(
        &mut self,
        instruction: Instruction<'_>,
        mode: Address,
        is_volatile: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.span()?;
        let address = operands.register()?;
        let byte_len = operands.u32()? as usize;
        let target = self.register_byte_range(target)?;
        if byte_len > target.len() {
            return Err(self.invalid_instruction());
        }
        let address = self.resolve_address(address.0, mode)?;

        // load each volatile byte exactly once or copy the ordinary range
        let target_address = self.fiber.stack.address(target.start) as *mut u8;
        if is_volatile {
            for offset in 0..byte_len {
                // SAFETY: volatile load requires one live source and target byte
                unsafe {
                    let value = ptr::read_volatile((address + offset) as *const u8);
                    ptr::write(target_address.add(offset), value);
                }
            }
        } else {
            // SAFETY: load requires a live source layout and checked target range
            unsafe { ptr::copy(address as *const u8, target_address, byte_len) };
        }

        // clear trailing register padding after reading any overlapping frame source
        self.fiber
            .stack
            .zero(target.start + byte_len, target.len() - byte_len)?;

        Ok(())
    }

    /// Store one packed value through the selected address representation.
    fn execute_value_store(
        &mut self,
        instruction: Instruction<'_>,
        mode: Address,
        is_volatile: bool,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let address = operands.register()?;
        let value = operands.span()?;
        let byte_len = operands.u32()? as usize;
        let value = self.register_byte_range(value)?;
        if byte_len > value.len() {
            return Err(self.invalid_instruction());
        }
        let address = self.resolve_address(address.0, mode)?;

        // store each volatile byte exactly once or copy the ordinary range
        let value = self.fiber.stack.address(value.start) as *const u8;
        if is_volatile {
            for offset in 0..byte_len {
                // SAFETY: volatile store requires one live source and target byte
                unsafe {
                    let value = ptr::read(value.add(offset));
                    ptr::write_volatile((address + offset) as *mut u8, value);
                }
            }
        } else {
            // SAFETY: store requires a live target layout and checked source range
            unsafe { ptr::copy(value, address as *mut u8, byte_len) };
        }

        Ok(())
    }

    /// Resolve one encoded memory operand to a process-local address.
    #[inline(always)]
    pub(crate) fn resolve_address(&self, register: u16, mode: Address) -> Result<usize> {
        let bits = self.read(register).bits();
        let address = match mode {
            Address::Reference => self.activation.memory.base_address() + bits as usize,
            Address::Pointer => bits as usize,
        };

        Ok(address)
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

    /// Load one volatile scalar from one native address.
    fn load_volatile(&self, address: usize, scalar: Scalar) -> Word {
        // SAFETY: volatile bytecode requires a live naturally aligned scalar address
        let bits = unsafe {
            match scalar.bit_width() {
                8 => ptr::read_volatile(address as *const u8) as u64,
                16 => ptr::read_volatile(address as *const u16) as u64,
                32 => ptr::read_volatile(address as *const u32) as u64,
                64 => ptr::read_volatile(address as *const u64),
                _ => unreachable!("volatile scalars occupy one bytecode word"),
            }
        };

        Word::from_bits(scalar.encode(bits))
    }

    /// Store one volatile scalar at one native address.
    fn store_volatile(&self, address: usize, scalar: Scalar, value: Word) {
        let bits = value.bits();

        // SAFETY: volatile bytecode requires a live naturally aligned mutable scalar address
        unsafe {
            match scalar.bit_width() {
                8 => ptr::write_volatile(address as *mut u8, bits as u8),
                16 => ptr::write_volatile(address as *mut u16, bits as u16),
                32 => ptr::write_volatile(address as *mut u32, bits as u32),
                64 => ptr::write_volatile(address as *mut u64, bits),
                _ => unreachable!("volatile scalars occupy one bytecode word"),
            }
        }
    }
}
