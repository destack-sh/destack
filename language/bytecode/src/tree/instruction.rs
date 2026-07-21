use crate::{
    CodeOffset, CounterId, Error, InstructionLayout, Opcode, Operand, ReferenceType, RegisterId,
    RegisterRange, Result, SamplerId, ValueType,
};

/// One borrowed instruction in a bytecode stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction<'a> {
    /// The complete encoded instruction bytes.
    bytes: &'a [u8],
    /// The first encoded operand byte.
    operand_offset: u8,
}

/// One encoded instruction header.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct InstructionHeader(u16);

impl InstructionHeader {
    /// The bit offset of the compact code-unit count.
    const CODE_UNIT_COUNT_SHIFT: u16 = 12;
    /// The greatest compact code-unit count.
    const COMPACT_CODE_UNIT_COUNT_MAX: usize = 0x0f;
    /// The smallest valid extended code-unit count.
    const EXTENDED_CODE_UNIT_COUNT_MIN: usize = Self::COMPACT_CODE_UNIT_COUNT_MAX + 2;
    /// The compact instruction header byte length.
    const BYTE_LEN: usize = size_of::<Self>();
    /// The extended instruction header byte length.
    const EXTENDED_BYTE_LEN: usize = Self::BYTE_LEN + size_of::<u16>();

    /// Create one compact instruction header.
    const fn compact(opcode: Opcode, code_unit_count: u8) -> Self {
        Self(((code_unit_count as u16) << Self::CODE_UNIT_COUNT_SHIFT) | opcode.code())
    }

    /// Create one extended instruction header.
    const fn extended(opcode: Opcode) -> Self {
        Self(opcode.code())
    }

    /// Return the encoded opcode.
    const fn opcode(self) -> Opcode {
        Opcode::from_code(self.0 & Opcode::MAX.code())
    }

    /// Return the compact code-unit count, or zero for an extended header.
    const fn compact_code_unit_count(self) -> u8 {
        (self.0 >> Self::CODE_UNIT_COUNT_SHIFT) as u8
    }

    /// Return the exact header bits.
    const fn bits(self) -> u16 {
        self.0
    }
}

impl<'a> Instruction<'a> {
    /// The byte length of one encoded code unit.
    const CODE_UNIT_BYTE_LEN: usize = size_of::<u16>();
    /// The greatest function byte length supported by relative branches.
    const FUNCTION_BYTE_LEN_MAX: usize = i32::MAX as usize;

    /// Append one instruction and return its function-local byte offset.
    pub fn pack(opcode: Opcode, operands: &[u8], code: &mut Vec<u8>) -> Result<CodeOffset> {
        if !opcode.is_defined() {
            return Err(Error::InvalidOpcode(opcode.code()));
        }
        if !operands.len().is_multiple_of(Self::CODE_UNIT_BYTE_LEN) {
            return Err(Error::UnalignedOperands(operands.len()));
        }

        // select the shortest self-delimiting header
        let compact_byte_len = InstructionHeader::BYTE_LEN + operands.len();
        let compact_code_unit_count = compact_byte_len / Self::CODE_UNIT_BYTE_LEN;
        let (header, extended_code_unit_count) =
            if compact_code_unit_count <= InstructionHeader::COMPACT_CODE_UNIT_COUNT_MAX {
                (
                    InstructionHeader::compact(opcode, compact_code_unit_count as u8),
                    None,
                )
            } else {
                let extended_byte_len = InstructionHeader::EXTENDED_BYTE_LEN + operands.len();
                let extended_code_unit_count = extended_byte_len / Self::CODE_UNIT_BYTE_LEN;
                let extended_code_unit_count = u16::try_from(extended_code_unit_count)
                    .map_err(|_| Error::InstructionTooLarge(extended_byte_len))?;

                (
                    InstructionHeader::extended(opcode),
                    Some(extended_code_unit_count),
                )
            };

        // require every in-function branch displacement to remain representable
        let header_byte_len = if extended_code_unit_count.is_some() {
            InstructionHeader::EXTENDED_BYTE_LEN
        } else {
            InstructionHeader::BYTE_LEN
        };
        let byte_len = header_byte_len + operands.len();
        let end = code.len() + byte_len;
        if end > Self::FUNCTION_BYTE_LEN_MAX {
            return Err(Error::FunctionTooLarge(end));
        }

        // append the header and exact operand bytes
        let offset = CodeOffset(code.len() as u32);
        code.extend_from_slice(&header.bits().to_le_bytes());
        if let Some(code_unit_count) = extended_code_unit_count {
            code.extend_from_slice(&code_unit_count.to_le_bytes());
        }
        code.extend_from_slice(operands);

        Ok(offset)
    }

    /// Read the first complete instruction from a byte stream.
    pub fn read(bytes: &'a [u8]) -> Result<Self> {
        let header = Self::decode_u16(bytes, 0)?;
        let header = InstructionHeader(header);
        let compact_code_unit_count = header.compact_code_unit_count();

        // reject operation codes outside the stable ISA
        let opcode = header.opcode();
        if !opcode.is_defined() {
            return Err(Error::InvalidOpcode(opcode.code()));
        }

        // read the compact or extended instruction width
        let (code_unit_count, operand_offset) = if compact_code_unit_count == 0 {
            let code_unit_count = Self::decode_u16(bytes, InstructionHeader::BYTE_LEN)? as usize;
            if code_unit_count < InstructionHeader::EXTENDED_CODE_UNIT_COUNT_MIN {
                return Err(Error::InvalidInstructionLength(
                    code_unit_count * Self::CODE_UNIT_BYTE_LEN,
                ));
            }

            (code_unit_count, InstructionHeader::EXTENDED_BYTE_LEN)
        } else {
            (
                compact_code_unit_count as usize,
                InstructionHeader::BYTE_LEN,
            )
        };

        // require a complete even-width instruction
        let byte_len = code_unit_count * Self::CODE_UNIT_BYTE_LEN;
        if byte_len < operand_offset {
            return Err(Error::InvalidInstructionLength(byte_len));
        }
        let Some(bytes) = bytes.get(..byte_len) else {
            return Err(Error::TruncatedInstruction);
        };

        Ok(Self {
            bytes,
            operand_offset: operand_offset as u8,
        })
    }

    /// Return this instruction's exact opcode.
    pub fn opcode(self) -> Opcode {
        let header = u16::from_le_bytes([self.bytes[0], self.bytes[1]]);

        InstructionHeader(header).opcode()
    }

    /// Return this instruction's complete encoded bytes.
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Return this instruction's complete encoded byte length.
    pub const fn byte_len(self) -> usize {
        self.bytes.len()
    }

    /// Return the encoded operand bytes.
    pub fn operand_bytes(self) -> &'a [u8] {
        &self.bytes[self.operand_offset as usize..]
    }

    /// Read this instruction's operands from the beginning.
    pub fn operands(self) -> Operands<'a> {
        Operands::new(self.operand_bytes())
    }

    /// Read one little-endian 16-bit operand.
    pub fn read_u16(self, byte_offset: usize) -> Result<u16> {
        Self::decode_u16(self.operand_bytes(), byte_offset)
    }

    /// Read one little-endian 32-bit operand.
    pub fn read_u32(self, byte_offset: usize) -> Result<u32> {
        let bytes = Self::decode_bytes::<4>(self.operand_bytes(), byte_offset)?;

        Ok(u32::from_le_bytes(bytes))
    }

    /// Read one little-endian 64-bit operand.
    pub fn read_u64(self, byte_offset: usize) -> Result<u64> {
        let bytes = Self::decode_bytes::<8>(self.operand_bytes(), byte_offset)?;

        Ok(u64::from_le_bytes(bytes))
    }

    /// Return every branch displacement byte offset in this instruction.
    pub(crate) fn branch_offsets(self, layout: InstructionLayout) -> Result<Vec<usize>> {
        let mut branch_offsets = Vec::new();
        let mut operand_offset = 0;

        // visit each encoded operand in schema order
        for operand in layout.operands() {
            let bytes = self
                .operand_bytes()
                .get(operand_offset..)
                .ok_or(Error::TruncatedInstruction)?;
            let byte_len = operand.byte_len(bytes)?;

            // collect direct and switch branch displacements
            match operand {
                Operand::Branch => branch_offsets.push(operand_offset),
                Operand::Switch => {
                    let count = self.read_u16(operand_offset)? as usize;
                    let entry_byte_len = size_of::<u64>() + size_of::<i32>();
                    for index in 0..count {
                        let branch_offset = operand_offset
                            + size_of::<u16>()
                            + index * entry_byte_len
                            + size_of::<u64>();
                        branch_offsets.push(branch_offset);
                    }
                }
                _ => {}
            }

            operand_offset += byte_len;
        }

        Ok(branch_offsets)
    }

    /// Read one little-endian 16-bit value.
    fn decode_u16(bytes: &[u8], byte_offset: usize) -> Result<u16> {
        let bytes = Self::decode_bytes::<2>(bytes, byte_offset)?;

        Ok(u16::from_le_bytes(bytes))
    }

    /// Read one fixed-width byte array.
    fn decode_bytes<const N: usize>(bytes: &[u8], byte_offset: usize) -> Result<[u8; N]> {
        let Some(bytes) = bytes.get(byte_offset..byte_offset + N) else {
            return Err(Error::TruncatedInstruction);
        };
        let mut result = [0; N];
        result.copy_from_slice(bytes);

        Ok(result)
    }
}

/// One cursor over an instruction's encoded operands.
#[derive(Clone, Copy, Debug)]
pub struct Operands<'a> {
    /// The complete encoded operand bytes.
    bytes: &'a [u8],
    /// The first unread operand byte.
    byte_offset: usize,
}

impl<'a> Operands<'a> {
    /// Create one cursor at the first encoded operand.
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_offset: 0,
        }
    }

    /// Return the first unread operand byte offset.
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    /// Return whether every encoded operand byte has been read.
    pub const fn is_empty(self) -> bool {
        self.byte_offset == self.bytes.len()
    }

    /// Read one register id.
    pub fn register(&mut self) -> Result<RegisterId> {
        Ok(RegisterId(self.u16()?))
    }

    /// Read one counted list of register ids.
    pub fn registers(&mut self) -> Result<Vec<RegisterId>> {
        let count = self.u16()? as usize;
        let mut registers = Vec::with_capacity(count);

        // decode the exact register sequence
        for _ in 0..count {
            registers.push(self.register()?);
        }

        Ok(registers)
    }

    /// Read one contiguous register range.
    pub fn range(&mut self) -> Result<RegisterRange> {
        let start = self.register()?;
        let word_count = self.u16()?;

        Ok(RegisterRange::new(start, word_count))
    }

    /// Read one reference representation.
    pub fn reference(&mut self) -> Result<ReferenceType> {
        let bits = self.u16()?;

        ReferenceType::from_bits(bits).ok_or(Error::InvalidOperand)
    }

    /// Read one complete value type.
    pub fn value_type(&mut self) -> Result<ValueType> {
        let bytes = self.take::<{ ValueType::BYTE_LEN }>()?;

        ValueType::from_bytes(bytes).ok_or(Error::InvalidOperand)
    }

    /// Read one function-local profile counter.
    pub fn counter(&mut self) -> Result<CounterId> {
        Ok(CounterId(self.u32()?))
    }

    /// Read one function-local profile sampler.
    pub fn sampler(&mut self) -> Result<SamplerId> {
        Ok(SamplerId(self.u32()?))
    }

    /// Read one counted list of unsigned 16-bit values.
    pub fn u16s(&mut self) -> Result<Vec<u16>> {
        let count = self.u16()? as usize;
        let mut values = Vec::with_capacity(count);

        // decode the exact value sequence
        for _ in 0..count {
            values.push(self.u16()?);
        }

        Ok(values)
    }

    /// Read one counted list of unsigned 64-bit values.
    pub fn u64s(&mut self) -> Result<Vec<u64>> {
        let count = self.u16()? as usize;
        let mut values = Vec::with_capacity(count);

        // decode the exact value sequence
        for _ in 0..count {
            values.push(self.u64()?);
        }

        Ok(values)
    }

    /// Read one unsigned 16-bit value.
    pub fn u16(&mut self) -> Result<u16> {
        let bytes = self.take::<2>()?;

        Ok(u16::from_le_bytes(bytes))
    }

    /// Read one unsigned 32-bit value.
    pub fn u32(&mut self) -> Result<u32> {
        let bytes = self.take::<4>()?;

        Ok(u32::from_le_bytes(bytes))
    }

    /// Read one signed 32-bit value.
    pub fn i32(&mut self) -> Result<i32> {
        let bytes = self.take::<4>()?;

        Ok(i32::from_le_bytes(bytes))
    }

    /// Read one unsigned 64-bit value.
    pub fn u64(&mut self) -> Result<u64> {
        let bytes = self.take::<8>()?;

        Ok(u64::from_le_bytes(bytes))
    }

    /// Read one unsigned 128-bit value.
    pub fn u128(&mut self) -> Result<u128> {
        let bytes = self.take::<16>()?;

        Ok(u128::from_le_bytes(bytes))
    }

    /// Read one exact fixed-width byte array.
    pub fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = Instruction::decode_bytes::<N>(self.bytes, self.byte_offset)?;
        self.byte_offset += N;

        Ok(bytes)
    }
}

/// Instructions borrowed from one complete function body.
#[derive(Clone, Copy, Debug)]
pub struct Instructions<'a> {
    /// The unread instruction bytes.
    bytes: &'a [u8],
}

impl<'a> Instructions<'a> {
    /// Create an instruction iterator over one function body.
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl<'a> Iterator for Instructions<'a> {
    type Item = Result<Instruction<'a>>;

    /// Read the next instruction.
    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        // stop after malformed input to avoid repeatedly returning the same error
        let instruction = match Instruction::read(self.bytes) {
            Ok(instruction) => instruction,
            Err(error) => {
                self.bytes = &[];

                return Some(Err(error));
            }
        };
        self.bytes = &self.bytes[instruction.bytes().len()..];

        Some(Ok(instruction))
    }
}

const _: () = assert!(size_of::<InstructionHeader>() == size_of::<u16>());
