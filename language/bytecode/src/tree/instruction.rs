use crate::{CodeOffset, Error, Opcode, Result};

const OPCODE_MASK: u16 = 0x0fff;
const CODE_UNIT_COUNT_SHIFT: u16 = 12;
const COMPACT_CODE_UNIT_COUNT_MAX: usize = 0x0f;
const EXTENDED_CODE_UNIT_COUNT_MIN: usize = COMPACT_CODE_UNIT_COUNT_MAX + 2;
const CODE_UNIT_BYTE_LEN: usize = size_of::<u16>();
const COMPACT_HEADER_BYTE_LEN: usize = size_of::<InstructionHeader>();
const EXTENDED_HEADER_BYTE_LEN: usize = COMPACT_HEADER_BYTE_LEN + size_of::<u16>();
const FUNCTION_BYTE_LEN_MAX: usize = i32::MAX as usize;

/// One borrowed instruction in a bytecode stream.
///
/// Every instruction stores operands in the exact order defined by its opcode layout.
/// Registers and inline counts are unsigned 16-bit integers.
/// Object symbols are unsigned 32-bit indices and control targets are signed 32-bit displacements
/// from the end of the containing instruction.
/// Scalar immediates occupy eight bytes and 128-bit integer immediates occupy sixteen.
/// Variable lists begin with one unsigned 16-bit element count.
/// Every multi-byte value is little-endian and every instruction ends on a two-byte boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction<'a> {
    /// The complete encoded instruction bytes.
    bytes: &'a [u8],
    /// The first encoded operand byte.
    operand_offset: u8,
}

/// One encoded instruction header.
///
/// The low twelve bits hold the opcode.
/// The high four bits hold the total 16-bit code-unit count for instructions of at most fifteen
/// code units.
/// Zero selects an extended header whose second code unit holds the complete count.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct InstructionHeader(u16);

impl InstructionHeader {
    /// Create one compact instruction header.
    const fn compact(opcode: Opcode, code_unit_count: u8) -> Self {
        Self(((code_unit_count as u16) << CODE_UNIT_COUNT_SHIFT) | opcode.code())
    }

    /// Create one extended instruction header.
    const fn extended(opcode: Opcode) -> Self {
        Self(opcode.code())
    }

    /// Return the encoded opcode.
    const fn opcode(self) -> Opcode {
        Opcode::from_code(self.0 & OPCODE_MASK)
    }

    /// Return the compact code-unit count, or zero for an extended header.
    const fn compact_code_unit_count(self) -> u8 {
        (self.0 >> CODE_UNIT_COUNT_SHIFT) as u8
    }

    /// Return the exact header bits.
    const fn bits(self) -> u16 {
        self.0
    }
}

impl<'a> Instruction<'a> {
    /// Append one instruction and return its function-local byte offset.
    pub fn pack(opcode: Opcode, operands: &[u8], code: &mut Vec<u8>) -> Result<CodeOffset> {
        if !opcode.is_defined() {
            return Err(Error::InvalidOpcode(opcode.code()));
        }
        if !operands.len().is_multiple_of(CODE_UNIT_BYTE_LEN) {
            return Err(Error::UnalignedOperands(operands.len()));
        }

        // select the shortest self-delimiting header
        let compact_byte_len = COMPACT_HEADER_BYTE_LEN + operands.len();
        let compact_code_unit_count = compact_byte_len / CODE_UNIT_BYTE_LEN;
        let (header, extended_code_unit_count) =
            if compact_code_unit_count <= COMPACT_CODE_UNIT_COUNT_MAX {
                (
                    InstructionHeader::compact(opcode, compact_code_unit_count as u8),
                    None,
                )
            } else {
                let extended_byte_len = EXTENDED_HEADER_BYTE_LEN + operands.len();
                let extended_code_unit_count = extended_byte_len / CODE_UNIT_BYTE_LEN;
                let extended_code_unit_count = u16::try_from(extended_code_unit_count)
                    .map_err(|_| Error::InstructionTooLarge(extended_byte_len))?;

                (
                    InstructionHeader::extended(opcode),
                    Some(extended_code_unit_count),
                )
            };

        // require every in-function branch displacement to remain representable
        let header_byte_len = if extended_code_unit_count.is_some() {
            EXTENDED_HEADER_BYTE_LEN
        } else {
            COMPACT_HEADER_BYTE_LEN
        };
        let byte_len = header_byte_len + operands.len();
        let end = code.len() + byte_len;
        if end > FUNCTION_BYTE_LEN_MAX {
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
            let code_unit_count = Self::decode_u16(bytes, COMPACT_HEADER_BYTE_LEN)? as usize;
            if code_unit_count < EXTENDED_CODE_UNIT_COUNT_MIN {
                return Err(Error::InvalidInstructionLength(
                    code_unit_count * CODE_UNIT_BYTE_LEN,
                ));
            }

            (code_unit_count, EXTENDED_HEADER_BYTE_LEN)
        } else {
            (compact_code_unit_count as usize, COMPACT_HEADER_BYTE_LEN)
        };

        // require a complete even-width instruction
        let byte_len = code_unit_count * CODE_UNIT_BYTE_LEN;
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
    pub fn operands(self) -> &'a [u8] {
        &self.bytes[self.operand_offset as usize..]
    }

    /// Read one little-endian 16-bit operand.
    pub fn read_u16(self, byte_offset: usize) -> Result<u16> {
        Self::decode_u16(self.operands(), byte_offset)
    }

    /// Read one little-endian 32-bit operand.
    pub fn read_u32(self, byte_offset: usize) -> Result<u32> {
        let bytes = Self::decode_bytes::<4>(self.operands(), byte_offset)?;

        Ok(u32::from_le_bytes(bytes))
    }

    /// Read one little-endian 64-bit operand.
    pub fn read_u64(self, byte_offset: usize) -> Result<u64> {
        let bytes = Self::decode_bytes::<8>(self.operands(), byte_offset)?;

        Ok(u64::from_le_bytes(bytes))
    }

    /// Validate this instruction against its opcode's exact operand layout.
    pub fn validate(self) -> Result<()> {
        let opcode = self.opcode();
        let Some(layout) = opcode.layout() else {
            return Err(Error::InvalidOpcode(opcode.code()));
        };
        let operands = self.operands();
        let mut byte_offset = 0;

        // consume each encoded operand in schema order
        for operand in layout.operands() {
            let bytes = &operands[byte_offset..];
            let byte_len = operand
                .byte_len(bytes)
                .map_err(|_| Error::InvalidOperands {
                    opcode: opcode.code(),
                    byte_offset,
                })?;
            byte_offset += byte_len;
        }

        // reject trailing bytes not owned by the opcode
        if byte_offset != operands.len() {
            return Err(Error::InvalidOperands {
                opcode: opcode.code(),
                byte_offset,
            });
        }

        Ok(())
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
