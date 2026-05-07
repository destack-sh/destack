use std::sync::atomic::Ordering;

use destack_mir as mir;

use crate::diagnostic::Error;

const ORDER_MASK: u32 = 0xFF;
const SIGNED_SHIFT: u32 = 8;
const ADDRESS_SHIFT: u32 = 9;
const WIDTH_SHIFT: u32 = 12;
const OPERATOR_SHIFT: u32 = 14;

/// Memory address representation selected by atomic lowering.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicAddress {
    /// Local heap reference.
    Heap = 0,
    /// Shared heap reference.
    SharedHeap = 1,
    /// Local raw-space pointer.
    Raw = 2,
    /// Shared raw-space pointer.
    SharedRaw = 3,
    /// Stack pointer.
    Stack = 4,
    /// Frame pointer.
    Frame = 5,
    /// Static pointer.
    Static = 6,
}

impl AtomicAddress {
    /// Encode this address class into an instruction field.
    #[inline(always)]
    pub(crate) const fn encode(self) -> u32 {
        self as u32
    }

    /// Decode one address class from an instruction field.
    #[inline(always)]
    pub(crate) fn decode(raw: u32) -> Result<Self, Error> {
        Ok(match raw {
            0 => Self::Heap,
            1 => Self::SharedHeap,
            2 => Self::Raw,
            3 => Self::SharedRaw,
            4 => Self::Stack,
            5 => Self::Frame,
            6 => Self::Static,
            _ => return Err(Error::InvalidInstruction),
        })
    }
}

/// Atomic memory ordering selected by lowering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicOrder {
    /// Relaxed ordering.
    Relaxed,
    /// Acquire ordering.
    Acquire,
    /// Release ordering.
    Release,
    /// Acquire-release ordering.
    AcquireRelease,
    /// Sequentially consistent ordering.
    SequentiallyConsistent,
}

impl AtomicOrder {
    /// Create one atomic order from MIR ordering.
    #[inline(always)]
    pub(crate) const fn from_mir(ordering: mir::MemoryOrdering) -> Self {
        match ordering {
            mir::MemoryOrdering::Relaxed => Self::Relaxed,
            mir::MemoryOrdering::Acquire => Self::Acquire,
            mir::MemoryOrdering::Release => Self::Release,
            mir::MemoryOrdering::AcquireRelease => Self::AcquireRelease,
            mir::MemoryOrdering::SequentiallyConsistent => Self::SequentiallyConsistent,
        }
    }

    /// Return the Rust atomic ordering.
    #[inline(always)]
    pub(crate) const fn to_std(self) -> Ordering {
        match self {
            Self::Relaxed => Ordering::Relaxed,
            Self::Acquire => Ordering::Acquire,
            Self::Release => Ordering::Release,
            Self::AcquireRelease => Ordering::AcqRel,
            Self::SequentiallyConsistent => Ordering::SeqCst,
        }
    }

    /// Return the Rust ordering for one atomic load.
    #[inline(always)]
    pub(crate) const fn to_std_load(self) -> Result<Ordering, Error> {
        match self {
            Self::Release | Self::AcquireRelease => Err(Error::InvalidInstruction),
            other => Ok(other.to_std()),
        }
    }

    /// Return the Rust ordering for one atomic store.
    #[inline(always)]
    pub(crate) const fn to_std_store(self) -> Result<Ordering, Error> {
        match self {
            Self::Acquire | Self::AcquireRelease => Err(Error::InvalidInstruction),
            other => Ok(other.to_std()),
        }
    }

    /// Return the Rust ordering for one atomic fence.
    #[inline(always)]
    pub(crate) const fn to_std_fence(self) -> Result<Ordering, Error> {
        match self {
            Self::Relaxed => Err(Error::InvalidInstruction),
            other => Ok(other.to_std()),
        }
    }

    /// Return the Rust ordering for one compare exchange failure load.
    #[inline(always)]
    pub(crate) const fn to_std_compare_exchange_failure(self) -> Result<Ordering, Error> {
        match self {
            Self::Release | Self::AcquireRelease => Err(Error::InvalidInstruction),
            other => Ok(other.to_std()),
        }
    }

    /// Return a valid failure ordering for an update CAS loop.
    #[inline(always)]
    pub(crate) const fn to_std_update_failure(self) -> Ordering {
        match self {
            Self::Release => Ordering::Relaxed,
            Self::AcquireRelease => Ordering::Acquire,
            other => other.to_std(),
        }
    }

    /// Encode this ordering into an instruction field.
    #[inline(always)]
    pub(crate) const fn encode(self) -> u32 {
        match self {
            Self::Relaxed => 0,
            Self::Acquire => 1,
            Self::Release => 2,
            Self::AcquireRelease => 3,
            Self::SequentiallyConsistent => 4,
        }
    }

    /// Decode one ordering from an instruction field.
    #[inline(always)]
    pub(crate) fn decode(raw: u32) -> Result<Self, Error> {
        Ok(match raw {
            0 => Self::Relaxed,
            1 => Self::Acquire,
            2 => Self::Release,
            3 => Self::AcquireRelease,
            4 => Self::SequentiallyConsistent,
            _ => return Err(Error::InvalidInstruction),
        })
    }
}

/// Atomic payload width selected by lowering.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicWidth {
    /// 8-bit payload.
    Width8 = 0,
    /// 16-bit payload.
    Width16 = 1,
    /// 32-bit payload.
    Width32 = 2,
    /// 64-bit payload.
    Width64 = 3,
}

impl AtomicWidth {
    /// Return the byte width.
    #[inline(always)]
    pub(crate) const fn byte_len(self) -> usize {
        match self {
            Self::Width8 => 1,
            Self::Width16 => 2,
            Self::Width32 => 4,
            Self::Width64 => 8,
        }
    }

    /// Encode this width into an instruction field.
    #[inline(always)]
    const fn encode(self) -> u32 {
        self as u32
    }

    /// Decode one width from an instruction field.
    #[inline(always)]
    fn decode(raw: u32) -> Result<Self, Error> {
        Ok(match raw {
            0 => Self::Width8,
            1 => Self::Width16,
            2 => Self::Width32,
            3 => Self::Width64,
            _ => return Err(Error::InvalidInstruction),
        })
    }
}

/// Address, width, and ordering for one atomic memory operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtomicShape {
    /// The addressed memory space.
    pub(crate) address: AtomicAddress,
    /// The atomic payload width.
    pub(crate) width: AtomicWidth,
    /// The memory ordering.
    pub(crate) order: AtomicOrder,
    /// Whether integer results should be sign-extended.
    pub(crate) is_signed: bool,
}

impl AtomicShape {
    /// Create one atomic memory shape.
    #[inline(always)]
    pub(crate) const fn new(
        address: AtomicAddress,
        width: AtomicWidth,
        order: AtomicOrder,
        is_signed: bool,
    ) -> Self {
        Self {
            address,
            width,
            order,
            is_signed,
        }
    }

    /// Encode this shape into an instruction field.
    #[inline(always)]
    pub(crate) const fn encode(self) -> u32 {
        self.order.encode()
            | ((self.is_signed as u32) << SIGNED_SHIFT)
            | (self.address.encode() << ADDRESS_SHIFT)
            | (self.width.encode() << WIDTH_SHIFT)
    }

    /// Decode one shape from an instruction field.
    #[inline(always)]
    pub(crate) fn decode(raw: u32) -> Result<Self, Error> {
        let order = AtomicOrder::decode(raw & ORDER_MASK)?;
        let is_signed = ((raw >> SIGNED_SHIFT) & 1) != 0;
        let address = AtomicAddress::decode((raw >> ADDRESS_SHIFT) & 0x7)?;
        let width = AtomicWidth::decode((raw >> WIDTH_SHIFT) & 0x3)?;

        Ok(Self {
            address,
            width,
            order,
            is_signed,
        })
    }
}

/// Atomic read-modify-write operation selected by lowering.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicReadModifyWriteOperator {
    /// Add the operand and return the old value.
    Add = 0,
    /// Subtract the operand and return the old value.
    Sub = 1,
    /// Bitwise and the operand and return the old value.
    And = 2,
    /// Bitwise or the operand and return the old value.
    Or = 3,
    /// Bitwise xor the operand and return the old value.
    Xor = 4,
    /// Signed minimum with the operand and return the old value.
    Min = 5,
    /// Signed maximum with the operand and return the old value.
    Max = 6,
    /// Unsigned minimum with the operand and return the old value.
    Umin = 7,
    /// Unsigned maximum with the operand and return the old value.
    Umax = 8,
    /// Floating add with the operand and return the old value.
    Fadd = 9,
    /// Floating minimum with the operand and return the old value.
    Fmin = 10,
    /// Floating maximum with the operand and return the old value.
    Fmax = 11,
}

impl AtomicReadModifyWriteOperator {
    /// Encode this operation into an instruction field.
    #[inline(always)]
    const fn encode(self) -> u32 {
        self as u32
    }

    /// Decode one operation from an instruction field.
    #[inline(always)]
    fn decode(raw: u32) -> Result<Self, Error> {
        Ok(match raw {
            0 => Self::Add,
            1 => Self::Sub,
            2 => Self::And,
            3 => Self::Or,
            4 => Self::Xor,
            5 => Self::Min,
            6 => Self::Max,
            7 => Self::Umin,
            8 => Self::Umax,
            9 => Self::Fadd,
            10 => Self::Fmin,
            11 => Self::Fmax,
            _ => return Err(Error::InvalidInstruction),
        })
    }
}

/// Shape for one atomic read-modify-write instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtomicReadModifyWriteShape {
    /// The atomic memory shape.
    pub(crate) shape: AtomicShape,
    /// The update operation.
    pub(crate) operator: AtomicReadModifyWriteOperator,
}

impl AtomicReadModifyWriteShape {
    /// Create one atomic read-modify-write shape.
    #[inline(always)]
    pub(crate) const fn new(shape: AtomicShape, operator: AtomicReadModifyWriteOperator) -> Self {
        Self { shape, operator }
    }

    /// Encode this shape into an instruction field.
    #[inline(always)]
    pub(crate) const fn encode(self) -> u32 {
        self.shape.encode() | (self.operator.encode() << OPERATOR_SHIFT)
    }

    /// Decode one shape from an instruction field.
    #[inline(always)]
    pub(crate) fn decode(raw: u32) -> Result<Self, Error> {
        let shape = AtomicShape::decode(raw)?;
        let operator = AtomicReadModifyWriteOperator::decode((raw >> OPERATOR_SHIFT) & 0xF)?;

        Ok(Self { shape, operator })
    }
}
