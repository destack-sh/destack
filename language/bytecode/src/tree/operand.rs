use crate::{Error, ReferenceType, Result, ValueType};

const COUNT_BYTE_LEN: usize = size_of::<u16>();
const REGISTER_BYTE_LEN: usize = size_of::<u16>();
const AXIS_BYTE_LEN: usize = size_of::<u16>();
const BRANCH_BYTE_LEN: usize = size_of::<i32>();
const SYMBOL_BYTE_LEN: usize = size_of::<u32>();
const BITS64_BYTE_LEN: usize = size_of::<u64>();
const SWITCH_ENTRY_BYTE_LEN: usize = BITS64_BYTE_LEN + BRANCH_BYTE_LEN;
const CONVOLUTION_GROUP_BYTE_LEN: usize = size_of::<u32>() * 2;
const CONTRACTION_AXIS_LISTS: [usize; 4] = [AXIS_BYTE_LEN; 4];
const WINDOW_LISTS: [usize; 6] = [
    BITS64_BYTE_LEN,
    BITS64_BYTE_LEN,
    BITS64_BYTE_LEN,
    BITS64_BYTE_LEN,
    BITS64_BYTE_LEN,
    AXIS_BYTE_LEN,
];
const INDEX_AXIS_LISTS: [usize; 3] = [AXIS_BYTE_LEN; 3];

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
        let byte_len = match self {
            Self::Result
            | Self::Register
            | Self::Scalar
            | Self::Operator
            | Self::AtomicAccess
            | Self::CompareExchangeAccess
            | Self::Unsigned16 => size_of::<u16>(),
            Self::Reference => size_of::<ReferenceType>(),
            Self::ValueType => ValueType::BYTE_LEN,
            Self::ResultRange | Self::RegisterRange => REGISTER_BYTE_LEN * 2,
            Self::Branch
            | Self::Signed32
            | Self::Unsigned32
            | Self::FrameSlot
            | Self::FenceAccess
            | Self::VectorType => size_of::<u32>(),
            Self::ConvolutionGroups => CONVOLUTION_GROUP_BYTE_LEN,
            Self::Type | Self::Function | Self::Global | Self::Constant | Self::FunctionType => {
                SYMBOL_BYTE_LEN
            }
            Self::Counter => size_of::<u32>(),
            Self::Bits64 => BITS64_BYTE_LEN,
            Self::Bits128 => size_of::<u128>(),
            Self::RegisterList | Self::Unsigned16List => {
                COUNT_BYTE_LEN + Self::count(bytes)? * REGISTER_BYTE_LEN
            }
            Self::Unsigned32List => COUNT_BYTE_LEN + Self::count(bytes)? * size_of::<u32>(),
            Self::Bits64List => COUNT_BYTE_LEN + Self::count(bytes)? * BITS64_BYTE_LEN,
            Self::Switch => COUNT_BYTE_LEN + Self::count(bytes)? * SWITCH_ENTRY_BYTE_LEN,
            Self::ContractionAxes => Self::lists_byte_len(bytes, &CONTRACTION_AXIS_LISTS)?,
            Self::ConvolutionAxes => Self::convolution_axes_byte_len(bytes)?,
            Self::Window => Self::lists_byte_len(bytes, &WINDOW_LISTS)?,
            Self::GatherAxes | Self::ScatterAxes => {
                Self::lists_byte_len(bytes, &INDEX_AXIS_LISTS)? + size_of::<u16>()
            }
        };

        if bytes.len() < byte_len {
            Err(Error::TruncatedInstruction)
        } else {
            Ok(byte_len)
        }
    }

    /// Read the leading element count of one variable-length operand.
    fn count(bytes: &[u8]) -> Result<usize> {
        let Some(bytes) = bytes.get(..COUNT_BYTE_LEN) else {
            return Err(Error::TruncatedInstruction);
        };
        let bytes = [bytes[0], bytes[1]];

        Ok(u16::from_le_bytes(bytes) as usize)
    }

    /// Return the encoded byte length of consecutive counted lists.
    fn lists_byte_len(bytes: &[u8], element_byte_lens: &[usize]) -> Result<usize> {
        let mut byte_offset = 0;

        // consume each counted list in field order
        for element_byte_len in element_byte_lens {
            let bytes = bytes
                .get(byte_offset..)
                .ok_or(Error::TruncatedInstruction)?;
            let count = Self::count(bytes)?;
            let byte_len = COUNT_BYTE_LEN + count * element_byte_len;
            byte_offset += byte_len;
        }

        Ok(byte_offset)
    }

    /// Return the encoded byte length of one convolution axis descriptor.
    fn convolution_axes_byte_len(bytes: &[u8]) -> Result<usize> {
        let mut byte_offset = AXIS_BYTE_LEN * 2;

        // consume input spatial axes
        let input = bytes
            .get(byte_offset..)
            .ok_or(Error::TruncatedInstruction)?;
        byte_offset += Self::lists_byte_len(input, &[AXIS_BYTE_LEN])?;

        // consume kernel feature and spatial axes
        byte_offset += AXIS_BYTE_LEN * 2;
        let kernel = bytes
            .get(byte_offset..)
            .ok_or(Error::TruncatedInstruction)?;
        byte_offset += Self::lists_byte_len(kernel, &[AXIS_BYTE_LEN])?;

        // consume output feature and spatial axes
        byte_offset += AXIS_BYTE_LEN * 2;
        let output = bytes
            .get(byte_offset..)
            .ok_or(Error::TruncatedInstruction)?;
        byte_offset += Self::lists_byte_len(output, &[AXIS_BYTE_LEN])?;

        Ok(byte_offset)
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
