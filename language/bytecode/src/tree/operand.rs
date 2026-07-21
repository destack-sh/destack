use crate::{Error, ReferenceType, Result, ValueType};

/// One encoded instruction operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operand {
    /// One result register encoded as an unsigned 16-bit index.
    Result,
    /// One result window stored in contiguous register words.
    ResultRange,
    /// One input register encoded as an unsigned 16-bit index.
    Register,
    /// A counted list of input registers.
    RegisterList,
    /// One input window stored in contiguous register words.
    RegisterRange,

    /// One signed branch displacement from the end of the instruction.
    Branch,
    /// Counted 64-bit case values paired with branch displacements.
    Switch,

    /// One relocatable runtime type symbol.
    Type,
    /// One relocatable function symbol.
    Function,
    /// One relocatable global symbol.
    Global,
    /// One relocatable immutable constant.
    Constant,
    /// One relocatable function type.
    FunctionType,
    /// One function-local profile counter.
    Counter,
    /// One function-local profile sampler.
    Sampler,
    /// One function-local frame slot.
    FrameSlot,

    /// One scalar representation code.
    Scalar,
    /// One reference ownership and memory space.
    Reference,
    /// One complete bytecode value type.
    ValueType,
    /// One operation code interpreted by the containing opcode.
    Operator,
    /// One atomic ordering and execution scope.
    AtomicAccess,
    /// One compare-exchange ordering and execution scope.
    CompareExchangeAccess,
    /// One fence ordering, execution scope, and storage set.
    FenceAccess,
    /// One fixed-width vector type.
    VectorType,

    /// Contracting and batch axes for one tensor contraction.
    ContractionAxes,
    /// Input, kernel, and output axes for one tensor convolution.
    ConvolutionAxes,
    /// Stride, padding, dilation, and reversal for one tensor window.
    Window,
    /// Feature and batch group counts for one tensor convolution.
    ConvolutionGroups,
    /// Output, collapsed, input, and index-vector axes for one tensor gather.
    GatherAxes,
    /// Update, inserted, input, and index-vector axes for one tensor scatter.
    ScatterAxes,

    /// One unsigned 16-bit immediate.
    Unsigned16,
    /// One counted list of unsigned 16-bit immediates.
    Unsigned16List,
    /// One unsigned 32-bit immediate.
    Unsigned32,
    /// One counted list of unsigned 32-bit immediates.
    Unsigned32List,
    /// One signed 32-bit immediate.
    Signed32,
    /// One exact 64-bit immediate.
    Bits64,
    /// One counted list of exact 64-bit immediates.
    Bits64List,
    /// One exact 128-bit immediate.
    Bits128,
}

impl Operand {
    /// Return this operand's encoded byte length at the start of one byte slice.
    pub fn byte_len(self, bytes: &[u8]) -> Result<usize> {
        let mut cursor = OperandCursor::new(bytes);

        // consume the exact encoded shape of this operand
        match self {
            Self::Result
            | Self::Register
            | Self::Scalar
            | Self::Operator
            | Self::AtomicAccess
            | Self::CompareExchangeAccess
            | Self::Unsigned16 => cursor.take::<u16>()?,
            Self::Reference => cursor.take::<ReferenceType>()?,
            Self::ValueType => cursor.take_bytes(ValueType::BYTE_LEN)?,
            Self::ResultRange | Self::RegisterRange => cursor.take::<[u16; 2]>()?,
            Self::Branch
            | Self::Signed32
            | Self::Unsigned32
            | Self::FrameSlot
            | Self::FenceAccess
            | Self::VectorType => cursor.take::<u32>()?,
            Self::ConvolutionGroups => cursor.take::<[u32; 2]>()?,
            Self::Type | Self::Function | Self::Global | Self::Constant | Self::FunctionType => {
                cursor.take::<u32>()?
            }
            Self::Counter | Self::Sampler => cursor.take::<u32>()?,
            Self::Bits64 => cursor.take::<u64>()?,
            Self::Bits128 => cursor.take::<u128>()?,
            Self::RegisterList | Self::Unsigned16List => cursor.take_list::<u16>()?,
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
