use std::cmp::Ordering;
use std::{ptr, slice};

use destack_bytecode::{Address, Instruction, MemoryOperation, Opcode, Prefetch, Scalar, Transfer};
use destack_program::{GlobalAddress, Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Construct one stable global address.
    pub(crate) fn execute_global_address(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let offset = usize::try_from(operands.u64()?).map_err(|_| self.invalid_instruction())?;
        let offset = match instruction.opcode() {
            Opcode::GLOBAL_ADDRESS_CONSTANT => offset,
            Opcode::GLOBAL_ADDRESS_LOCAL => self
                .activation
                .memory
                .local_statics
                .offset()
                .checked_add(offset)
                .ok_or_else(|| self.invalid_instruction())?,
            Opcode::GLOBAL_ADDRESS_SHARED => self
                .activation
                .memory
                .shared_statics
                .offset()
                .checked_add(offset)
                .ok_or_else(|| self.invalid_instruction())?,
            _ => unreachable!("global address dispatch selects one static storage"),
        };
        let reference = GlobalAddress::new(offset);

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

    /// Execute one reference or native pointer arithmetic operation.
    pub(crate) fn execute_address_arithmetic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<()> {
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let left = operands.register()?;
        let left = self.read(left.0).bits();
        let value = match instruction.opcode() {
            Opcode::REFERENCE_ADD_IMMEDIATE | Opcode::POINTER_ADD_IMMEDIATE => {
                let offset = operands.i32()?;

                left.wrapping_add_signed(i64::from(offset))
            }
            Opcode::REFERENCE_ADD | Opcode::POINTER_ADD => {
                let offset = operands.register()?;

                left.wrapping_add_signed(self.read(offset.0).as_i64())
            }
            Opcode::REFERENCE_ADD_SCALED | Opcode::POINTER_ADD_SCALED => {
                let offset = operands.register()?;
                let stride = operands.u32()?;
                let offset = self.read(offset.0).as_i64().wrapping_mul(i64::from(stride));

                left.wrapping_add_signed(offset)
            }
            Opcode::REFERENCE_DIFF | Opcode::POINTER_DIFF => {
                let origin = operands.register()?;

                left.wrapping_sub(self.read(origin.0).bits())
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
            let reference = operands.register()?;
            let byte_len = usize::from(scalar.bit_width() / 8);
            let address = self.resolve_address(reference.0, mode, byte_len)?;
            let value = if is_volatile {
                self.load_volatile(address, scalar)
            } else {
                self.load(address, scalar)
            };

            self.write(target.0, value);
        } else {
            let reference = operands.register()?;
            let value = operands.register()?;
            let byte_len = usize::from(scalar.bit_width() / 8);
            let address = self.resolve_address(reference.0, mode, byte_len)?;

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
        let target = self.resolve_address(target.0, target_mode, byte_len)? as *mut u8;
        let source = self.resolve_address(source.0, source_mode, byte_len)? as *const u8;

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
        let target = self.resolve_address(target.0, target_mode, byte_len)? as *mut u8;
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
        let left = self.resolve_address(left.0, left_mode, byte_len)?;
        let right = self.resolve_address(right.0, right_mode, byte_len)?;
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
        let _pointer = operands.register()?;

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
        let address = self.resolve_address(address.0, mode, byte_len)?;

        // clear register padding before loading exact layout bytes
        self.fiber.stack.zero(target.start, target.len())?;

        // load each volatile byte exactly once or copy the ordinary range
        let target = self.fiber.stack.address(target.start) as *mut u8;
        if is_volatile {
            for offset in 0..byte_len {
                // SAFETY: volatile load requires one live source and target byte
                unsafe {
                    let value = ptr::read_volatile((address + offset) as *const u8);
                    ptr::write(target.add(offset), value);
                }
            }
        } else {
            // SAFETY: load requires a live source layout and checked target range
            unsafe { ptr::copy(address as *const u8, target, byte_len) };
        }

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
        let address = self.resolve_address(address.0, mode, byte_len)?;

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
    pub(crate) fn resolve_address(
        &self,
        register: u16,
        mode: Address,
        byte_len: usize,
    ) -> Result<usize> {
        let bits = self.read(register).bits();
        let pointer = match mode {
            Address::Memory => self.activation.memory.base_address() + bits as usize,
            Address::Constant => self
                .machine
                .program
                .constant_address(GlobalAddress::from_bits(bits), byte_len)
                .ok_or_else(|| self.invalid_instruction())?,
            Address::Pointer => bits as usize,
        };

        Ok(pointer)
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
