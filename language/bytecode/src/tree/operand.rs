use crate::{Error, ReferenceType, Result, TensorOperand, ValueType};

/// One encoded instruction operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    // register file
    /// One result register encoded as an unsigned 16-bit index.
    Result,
    /// One result range encoded as a 16-bit start and 16-bit word count.
    ResultRange,
    /// One input register encoded as an unsigned 16-bit index.
    Register,
    /// One input range encoded as a 16-bit start and 16-bit word count.
    RegisterRange,
    /// A 16-bit count followed by unsigned 16-bit input register indices.
    RegisterList,

    // control flow
    /// One signed 32-bit branch displacement from the end of the instruction.
    Branch,
    /// A 16-bit count followed by 64-bit case values and 32-bit branch displacements.
    Switch,

    // linked identities
    /// One relocatable runtime type symbol encoded as an unsigned 32-bit index.
    Type,
    /// One relocatable function symbol encoded as an unsigned 32-bit index.
    Function,
    /// One relocatable global symbol encoded as an unsigned 32-bit index.
    Global,
    /// One relocatable immutable constant encoded as an unsigned 32-bit index.
    Constant,
    /// One relocatable dynamic dispatch table encoded as an unsigned 32-bit index.
    DynamicTable,

    // function-local identities
    /// One function-local frame slot encoded as an unsigned 32-bit index.
    FrameSlot,
    /// One function-local profile counter encoded as an unsigned 32-bit index.
    Counter,
    /// One function-local profile sampler encoded as an unsigned 32-bit index.
    Sampler,

    // value representations
    /// One scalar representation encoded as an unsigned 16-bit code.
    Scalar,
    /// One reference kind and space encoded as two unsigned bytes.
    Reference,
    /// One fixed-width vector type encoded as four bytes.
    VectorType,
    /// One complete fixed-width bytecode value type.
    ValueType,
    /// One tensor register range and runtime type symbol.
    Tensor,
    /// A 16-bit count followed by tensor register ranges and runtime type symbols.
    TensorList,

    // execution controls
    /// One operation code interpreted by the containing opcode, encoded as 16 bits.
    Operator,
    /// One atomic ordering and execution scope encoded as 16 bits.
    AtomicAccess,
    /// One compare-exchange ordering and execution scope encoded as 16 bits.
    CompareExchangeAccess,
    /// One fence ordering, execution scope, and storage set encoded as 32 bits.
    FenceAccess,

    // tensor geometry
    /// Four counted 16-bit axis lists for one tensor contraction.
    ContractionAxes,
    /// Three input, kernel, and output dimension mappings for one tensor convolution.
    ConvolutionAxes,
    /// Five counted 64-bit dimension lists and one 16-bit reversal list.
    Window,
    /// Feature and batch group counts encoded as two unsigned 32-bit values.
    ConvolutionGroups,
    /// Three counted 16-bit axis lists and one 16-bit index-vector axis for gather.
    GatherAxes,
    /// Three counted 16-bit axis lists and one 16-bit index-vector axis for scatter.
    ScatterAxes,

    // immediates
    /// One unsigned 16-bit immediate.
    Unsigned16,
    /// A 16-bit count followed by unsigned 16-bit immediates.
    Unsigned16List,
    /// One unsigned 32-bit immediate.
    Unsigned32,
    /// A 16-bit count followed by unsigned 32-bit immediates.
    Unsigned32List,
    /// One signed 32-bit immediate.
    Signed32,
    /// One exact 64-bit immediate.
    Bits64,
    /// A 16-bit count followed by exact 64-bit immediates.
    Bits64List,
    /// One exact 128-bit immediate.
    Bits128,
}

impl Operand {
    /// Return the encoded byte length when it does not depend on operand bytes.
    pub const fn fixed_byte_len(self) -> Option<usize> {
        match self {
            Self::Result
            | Self::Register
            | Self::Scalar
            | Self::Operator
            | Self::AtomicAccess
            | Self::CompareExchangeAccess
            | Self::Unsigned16 => Some(size_of::<u16>()),
            Self::Reference => Some(size_of::<ReferenceType>()),
            Self::ValueType => Some(ValueType::BYTE_LEN),
            Self::ResultRange | Self::RegisterRange => Some(size_of::<[u16; 2]>()),
            Self::Branch
            | Self::Type
            | Self::Function
            | Self::Global
            | Self::Constant
            | Self::DynamicTable
            | Self::FrameSlot
            | Self::Counter
            | Self::Sampler
            | Self::FenceAccess
            | Self::VectorType
            | Self::Unsigned32
            | Self::Signed32 => Some(size_of::<u32>()),
            Self::ConvolutionGroups => Some(size_of::<[u32; 2]>()),
            Self::Tensor => Some(TensorOperand::BYTE_LEN),
            Self::Bits64 => Some(size_of::<u64>()),
            Self::Bits128 => Some(size_of::<u128>()),
            Self::RegisterList
            | Self::Switch
            | Self::TensorList
            | Self::ContractionAxes
            | Self::ConvolutionAxes
            | Self::Window
            | Self::GatherAxes
            | Self::ScatterAxes
            | Self::Unsigned16List
            | Self::Unsigned32List
            | Self::Bits64List => None,
        }
    }

    /// Return this operand's encoded byte length at the start of one byte slice.
    pub fn byte_len(self, bytes: &[u8]) -> Result<usize> {
        let mut cursor = OperandCursor::new(bytes);

        // consume fixed-width operands without interpreting their bits
        if let Some(byte_len) = self.fixed_byte_len() {
            cursor.take_bytes(byte_len)?;

            return Ok(cursor.byte_len());
        }

        // consume the exact variable-width operand shape
        match self {
            Self::RegisterList | Self::Unsigned16List => cursor.take_list::<u16>()?,
            Self::TensorList => cursor.take_list_bytes(TensorOperand::BYTE_LEN)?,
            Self::Unsigned32List => cursor.take_list::<u32>()?,
            Self::Bits64List => cursor.take_list::<u64>()?,
            Self::Switch => {
                cursor.take_list_bytes(size_of::<u64>() + size_of::<i32>())?;
            }
            Self::ContractionAxes => {
                for _ in 0..4 {
                    cursor.take_list::<u16>()?;
                }
            }
            Self::ConvolutionAxes => {
                for _ in 0..3 {
                    cursor.take::<[u16; 2]>()?;
                    cursor.take_list::<u16>()?;
                }
            }
            Self::Window => {
                for _ in 0..5 {
                    cursor.take_list::<u64>()?;
                }
                cursor.take_list::<u16>()?;
            }
            Self::GatherAxes | Self::ScatterAxes => {
                for _ in 0..3 {
                    cursor.take_list::<u16>()?;
                }
                cursor.take::<u16>()?;
            }
            _ => unreachable!("fixed-width operand handled above"),
        }

        Ok(cursor.byte_len())
    }
}

/// The exact operand sequence of one bytecode instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstructionLayout {
    /// The encoded operands in byte order.
    operands: &'static [Operand],
}

impl InstructionLayout {
    /// Create one instruction layout from its exact operand sequence.
    pub(crate) const fn new(operands: &'static [Operand]) -> Self {
        Self { operands }
    }

    /// Return the encoded operands in byte order.
    pub const fn operands(self) -> &'static [Operand] {
        self.operands
    }
}

/// One cursor over a variable-width encoded operand.
struct OperandCursor<'a> {
    /// The complete available operand bytes.
    bytes: &'a [u8],
    /// The first unread byte.
    byte_offset: usize,
}

impl<'a> OperandCursor<'a> {
    /// Create one cursor at the start of an operand.
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte_offset: 0,
        }
    }

    /// Return the consumed operand byte length.
    const fn byte_len(&self) -> usize {
        self.byte_offset
    }

    /// Consume one fixed-width value.
    fn take<T>(&mut self) -> Result<()> {
        self.take_bytes(size_of::<T>())
    }

    /// Consume one counted list of fixed-width values.
    fn take_list<T>(&mut self) -> Result<()> {
        self.take_list_bytes(size_of::<T>())
    }

    /// Consume one counted list with an exact element width.
    fn take_list_bytes(&mut self, element_byte_len: usize) -> Result<()> {
        let count = self.read_u16()? as usize;
        self.take_bytes(size_of::<u16>() + count * element_byte_len)
    }

    /// Consume one exact byte range.
    fn take_bytes(&mut self, byte_len: usize) -> Result<()> {
        let byte_offset = self.byte_offset + byte_len;
        if self.bytes.len() < byte_offset {
            return Err(Error::TruncatedInstruction);
        }
        self.byte_offset = byte_offset;

        Ok(())
    }

    /// Read one unsigned 16-bit value at the current position.
    fn read_u16(&self) -> Result<u16> {
        let Some(bytes) = self
            .bytes
            .get(self.byte_offset..self.byte_offset + size_of::<u16>())
        else {
            return Err(Error::TruncatedInstruction);
        };

        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }
}
