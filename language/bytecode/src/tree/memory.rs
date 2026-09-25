use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Scalar, ValueType};

/// The base used to resolve one bytecode memory operand.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Address {
    /// A stable offset inside the active memory map.
    Reference = 0,
    /// A process-local native pointer.
    Pointer = 1,
}

impl Address {
    /// The number of reserved addressing modes.
    pub(crate) const COUNT: u16 = 2;

    /// Decode one stable addressing mode code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Reference),
            1 => Some(Self::Pointer),
            _ => None,
        }
    }
}

/// One memory operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryOperation {
    /// Load one value.
    Load = 0,
    /// Store one value.
    Store = 1,
}

impl MemoryOperation {
    /// Decode one stable memory operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Load),
            1 => Some(Self::Store),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Store => "store",
        }
    }
}

/// One byte range transfer.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Transfer {
    /// Copy one non-overlapping byte range.
    Copy = 0,
    /// Move one potentially overlapping byte range.
    Move = 1,
}

impl Transfer {
    /// Decode one stable byte range transfer code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Copy),
            1 => Some(Self::Move),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Move => "move",
        }
    }
}

/// One memory prefetch direction.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Prefetch {
    /// Prefetch one range for reading.
    Read = 0,
    /// Prefetch one range for writing.
    Write = 1,
}

impl Prefetch {
    /// Decode one stable prefetch code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Read),
            1 => Some(Self::Write),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// One atomic memory operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AtomicOperation {
    /// Load one atomic value.
    Load = 0,
    /// Store one atomic value.
    Store = 1,
    /// Exchange one atomic value.
    Exchange = 2,
    /// Compare and exchange one atomic value.
    CompareExchange = 3,
    /// Weakly compare and exchange one atomic value.
    CompareExchangeWeak = 4,
    /// Atomically add one value.
    FetchAdd = 5,
    /// Atomically subtract one value.
    FetchSubtract = 6,
    /// Atomically apply bitwise conjunction.
    FetchAnd = 7,
    /// Atomically apply bitwise disjunction.
    FetchOr = 8,
    /// Atomically apply bitwise exclusive disjunction.
    FetchXor = 9,
    /// Atomically select the minimum value.
    FetchMinimum = 10,
    /// Atomically select the maximum value.
    FetchMaximum = 11,
}

impl AtomicOperation {
    /// Return the atomic operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "load" => Some(Self::Load),
            "store" => Some(Self::Store),
            "exchange" => Some(Self::Exchange),
            "cas" => Some(Self::CompareExchange),
            "cas.weak" => Some(Self::CompareExchangeWeak),
            "rmw.add" => Some(Self::FetchAdd),
            "rmw.sub" => Some(Self::FetchSubtract),
            "rmw.and" => Some(Self::FetchAnd),
            "rmw.or" => Some(Self::FetchOr),
            "rmw.xor" => Some(Self::FetchXor),
            "rmw.min" => Some(Self::FetchMinimum),
            "rmw.max" => Some(Self::FetchMaximum),
            _ => None,
        }
    }

    /// Return the canonical atomic operation name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Store => "store",
            Self::Exchange => "exchange",
            Self::CompareExchange => "cas",
            Self::CompareExchangeWeak => "cas.weak",
            Self::FetchAdd => "rmw.add",
            Self::FetchSubtract => "rmw.sub",
            Self::FetchAnd => "rmw.and",
            Self::FetchOr => "rmw.or",
            Self::FetchXor => "rmw.xor",
            Self::FetchMinimum => "rmw.min",
            Self::FetchMaximum => "rmw.max",
        }
    }

    /// Decode one stable atomic operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Load),
            1 => Some(Self::Store),
            2 => Some(Self::Exchange),
            3 => Some(Self::CompareExchange),
            4 => Some(Self::CompareExchangeWeak),
            5 => Some(Self::FetchAdd),
            6 => Some(Self::FetchSubtract),
            7 => Some(Self::FetchAnd),
            8 => Some(Self::FetchOr),
            9 => Some(Self::FetchXor),
            10 => Some(Self::FetchMinimum),
            11 => Some(Self::FetchMaximum),
            _ => None,
        }
    }

    /// Return whether this operation accepts one scalar representation.
    pub const fn supports(self, scalar: Scalar) -> bool {
        match self {
            Self::Load
            | Self::Store
            | Self::Exchange
            | Self::CompareExchange
            | Self::CompareExchangeWeak => true,
            Self::FetchAdd | Self::FetchSubtract | Self::FetchMinimum | Self::FetchMaximum => {
                scalar.is_integer() || scalar.is_float()
            }
            Self::FetchAnd | Self::FetchOr | Self::FetchXor => scalar.is_integer(),
        }
    }

    /// Return whether this operation accepts one memory order.
    pub const fn accepts(self, order: AtomicOrder) -> bool {
        match self {
            Self::Load => matches!(
                order,
                AtomicOrder::Relaxed | AtomicOrder::Acquire | AtomicOrder::SequentiallyConsistent
            ),
            Self::Store => matches!(
                order,
                AtomicOrder::Relaxed | AtomicOrder::Release | AtomicOrder::SequentiallyConsistent
            ),
            Self::Exchange
            | Self::CompareExchange
            | Self::CompareExchangeWeak
            | Self::FetchAdd
            | Self::FetchSubtract
            | Self::FetchAnd
            | Self::FetchOr
            | Self::FetchXor
            | Self::FetchMinimum
            | Self::FetchMaximum => true,
        }
    }

    /// Return whether this is one strong or weak compare exchange.
    pub const fn is_compare_exchange(self) -> bool {
        matches!(self, Self::CompareExchange | Self::CompareExchangeWeak)
    }

    /// Return the number of values produced by this operation.
    pub const fn result_count(self) -> usize {
        match self {
            Self::Store => 0,
            Self::CompareExchange | Self::CompareExchangeWeak => 2,
            _ => 1,
        }
    }

    /// Return the logical result type for this operation and scalar.
    pub const fn result_type(self, scalar: Scalar) -> ValueType {
        ValueType::scalar(scalar)
    }
}

/// One atomic memory order.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AtomicOrder {
    /// No ordering beyond atomicity.
    Relaxed = 0,
    /// Acquire ordering.
    Acquire = 1,
    /// Release ordering.
    Release = 2,
    /// Acquire and release ordering.
    AcquireRelease = 3,
    /// Sequentially consistent ordering.
    SequentiallyConsistent = 4,
}

impl AtomicOrder {
    /// Return the memory order with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "relaxed" => Some(Self::Relaxed),
            "acquire" => Some(Self::Acquire),
            "release" => Some(Self::Release),
            "acquireRelease" => Some(Self::AcquireRelease),
            "sequentiallyConsistent" => Some(Self::SequentiallyConsistent),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Acquire => "acquire",
            Self::Release => "release",
            Self::AcquireRelease => "acquireRelease",
            Self::SequentiallyConsistent => "sequentiallyConsistent",
        }
    }

    /// Decode one stable memory order code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Relaxed),
            1 => Some(Self::Acquire),
            2 => Some(Self::Release),
            3 => Some(Self::AcquireRelease),
            4 => Some(Self::SequentiallyConsistent),
            _ => None,
        }
    }

    /// Return whether this success order permits one failure order.
    pub const fn permits_failure(self, failure: Self) -> bool {
        match self {
            Self::Relaxed => matches!(failure, Self::Relaxed),
            Self::Acquire | Self::AcquireRelease => {
                matches!(failure, Self::Relaxed | Self::Acquire)
            }
            Self::Release => matches!(failure, Self::Relaxed),
            Self::SequentiallyConsistent => matches!(
                failure,
                Self::Relaxed | Self::Acquire | Self::SequentiallyConsistent
            ),
        }
    }
}

/// One accelerated execution scope.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ExecutionScope {
    /// One invocation.
    Invocation = 0,
    /// One synchronized subgroup.
    Subgroup = 1,
    /// One synchronized workgroup.
    Workgroup = 2,
    /// One accelerator device.
    Device = 3,
    /// The whole system.
    System = 4,
}

impl ExecutionScope {
    /// Return the execution scope with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "invocation" => Some(Self::Invocation),
            "subgroup" => Some(Self::Subgroup),
            "workgroup" => Some(Self::Workgroup),
            "device" => Some(Self::Device),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Invocation => "invocation",
            Self::Subgroup => "subgroup",
            Self::Workgroup => "workgroup",
            Self::Device => "device",
            Self::System => "system",
        }
    }

    /// Decode one stable execution scope code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Invocation),
            1 => Some(Self::Subgroup),
            2 => Some(Self::Workgroup),
            3 => Some(Self::Device),
            4 => Some(Self::System),
            _ => None,
        }
    }
}

/// Storage regions ordered by one atomic fence.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct StorageSet(pub u16);

impl StorageSet {
    /// No storage regions.
    pub const NONE: Self = Self(0);
    /// Worker-local storage.
    pub const LOCAL: Self = Self(1 << 0);
    /// Runtime-shared storage.
    pub const SHARED: Self = Self(1 << 1);
    /// Activation-frame storage.
    pub const FRAME: Self = Self(1 << 2);
    /// Program global storage.
    pub const GLOBAL: Self = Self(1 << 3);
    /// Every bytecode-visible storage region.
    pub const ANY: Self = Self(Self::LOCAL.0 | Self::SHARED.0 | Self::FRAME.0 | Self::GLOBAL.0);

    /// Return whether every selected storage region is defined by the ISA.
    pub const fn is_defined(self) -> bool {
        self.0 & !Self::ANY.0 == 0
    }
}

/// Ordering and visibility for one atomic access.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AtomicAccess {
    /// The memory order.
    pub order: AtomicOrder,
    /// The participating execution scope.
    pub scope: ExecutionScope,
}

impl AtomicAccess {
    /// Decode one stable instruction operand.
    pub const fn from_bits(bits: u16) -> Option<Self> {
        let order = AtomicOrder::from_code((bits & 0x7) as u8);
        let scope = ExecutionScope::from_code(((bits >> 3) & 0x7) as u8);

        match (order, scope) {
            (Some(order), Some(scope)) => {
                let access = Self { order, scope };

                if access.bits() == bits {
                    Some(access)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Encode this access into one stable instruction operand.
    pub const fn bits(self) -> u16 {
        self.order as u16 | ((self.scope as u16) << 3)
    }
}

/// Ordering and visibility for one atomic compare exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CompareExchangeAccess {
    /// The success memory order.
    pub success: AtomicOrder,
    /// The failure memory order.
    pub failure: AtomicOrder,
    /// The participating execution scope.
    pub scope: ExecutionScope,
}

impl CompareExchangeAccess {
    /// Decode one stable instruction operand.
    pub const fn from_bits(bits: u16) -> Option<Self> {
        let success = AtomicOrder::from_code((bits & 0x7) as u8);
        let failure = AtomicOrder::from_code(((bits >> 3) & 0x7) as u8);
        let scope = ExecutionScope::from_code(((bits >> 6) & 0x7) as u8);

        match (success, failure, scope) {
            (Some(success), Some(failure), Some(scope)) => {
                let access = Self {
                    success,
                    failure,
                    scope,
                };

                if access.bits() == bits {
                    Some(access)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Encode this access into one stable instruction operand.
    pub const fn bits(self) -> u16 {
        self.success as u16 | ((self.failure as u16) << 3) | ((self.scope as u16) << 6)
    }
}

/// Ordering, visibility, and storage for one atomic fence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FenceAccess {
    /// The memory order.
    pub order: AtomicOrder,
    /// The participating execution scope.
    pub scope: ExecutionScope,
    /// The ordered storage regions.
    pub storage: StorageSet,
}

impl FenceAccess {
    /// Decode one stable instruction operand.
    pub const fn from_bits(bits: u32) -> Option<Self> {
        let order = AtomicOrder::from_code((bits & 0x7) as u8);
        let scope = ExecutionScope::from_code(((bits >> 3) & 0x7) as u8);
        let storage = StorageSet((bits >> 8) as u16);

        match (order, scope) {
            (Some(order), Some(scope)) if storage.is_defined() => {
                let access = Self {
                    order,
                    scope,
                    storage,
                };

                if access.bits() == bits {
                    Some(access)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Encode this access into one stable instruction operand.
    pub const fn bits(self) -> u32 {
        self.order as u32 | ((self.scope as u32) << 3) | ((self.storage.0 as u32) << 8)
    }
}
