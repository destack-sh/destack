use serde::{Deserialize, Serialize};

use crate::{AddressSpace, MemorySpaceSet};

/// Set of address spaces that an operation may access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AddressSpaceSet {
    /// Address spaces included in the set.
    pub spaces: Vec<AddressSpace>,
}

impl AddressSpaceSet {
    /// Create an address space set from the provided entries.
    pub fn new(spaces: Vec<AddressSpace>) -> Self {
        Self { spaces }
    }

    /// Check whether the set contains the target address space.
    pub fn contains(&self, space: AddressSpace) -> bool {
        self.spaces.contains(&space)
    }

    /// Check whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.spaces.is_empty()
    }

    /// Check whether two address space sets intersect.
    pub fn intersects(&self, other: &Self) -> bool {
        self.spaces
            .iter()
            .any(|space| other.contains(space.clone()))
    }

    /// Check whether two address space sets are disjoint.
    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.intersects(other)
    }
}

/// Memory effect summary for a call or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryEffect {
    /// Whether the operation may read memory.
    pub reads: bool,
    /// Whether the operation may write memory.
    pub writes: bool,
    /// The memory spaces that may be accessed.
    pub spaces: MemorySpaceSet,
    /// Optional address space restriction for the access set.
    pub address_spaces: Option<AddressSpaceSet>,
    /// True when the operation only touches memory reachable from arguments.
    pub argmemonly: bool,
    /// True when the operation only touches inaccessible memory.
    pub inaccessible_mem_only: bool,
    /// True when the operation does not synchronize or use atomics.
    pub nosync: bool,
}

impl MemoryEffect {
    /// Create an effect with no memory access.
    pub const fn none() -> Self {
        Self {
            reads: false,
            writes: false,
            spaces: MemorySpaceSet::NONE,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a read only effect over the provided spaces.
    pub const fn read_only(spaces: MemorySpaceSet) -> Self {
        Self {
            reads: true,
            writes: false,
            spaces,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a write only effect over the provided spaces.
    pub const fn write_only(spaces: MemorySpaceSet) -> Self {
        Self {
            reads: false,
            writes: true,
            spaces,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a read write effect over the provided spaces.
    pub const fn read_write(spaces: MemorySpaceSet) -> Self {
        Self {
            reads: true,
            writes: true,
            spaces,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a conservative unknown effect.
    pub const fn unknown() -> Self {
        Self {
            reads: true,
            writes: true,
            spaces: MemorySpaceSet::ANY,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Return this effect with a refined address space set.
    pub fn with_address_spaces(mut self, address_spaces: AddressSpaceSet) -> Self {
        self.address_spaces = Some(address_spaces);
        self
    }

    /// Return this effect with the argmemonly flag enabled.
    pub const fn with_argmemonly(mut self) -> Self {
        self.argmemonly = true;
        self
    }

    /// Return this effect with the inaccessible memory only flag enabled.
    pub const fn with_inaccessible_mem_only(mut self) -> Self {
        self.inaccessible_mem_only = true;
        self
    }

    /// Return this effect with the nosync flag enabled.
    pub const fn with_nosync(mut self) -> Self {
        self.nosync = true;
        self
    }
}

impl Default for MemoryEffect {
    fn default() -> Self {
        Self::unknown()
    }
}

/// Effect class for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectClass {
    /// Pure operation with no observable side effects.
    Pure,
    /// Deterministic operation with observable effects.
    Deterministic,
    /// Non-deterministic operation with observable effects.
    NonDeterministic,
}

impl EffectClass {
    /// Return true when the operation is pure.
    pub fn is_pure(self) -> bool {
        matches!(self, Self::Pure)
    }

    /// Return true when the operation is deterministic.
    pub fn is_deterministic(self) -> bool {
        !matches!(self, Self::NonDeterministic)
    }
}

/// Unwind behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnwindBehavior {
    /// The operation cannot unwind.
    CannotUnwind,
    /// The operation may unwind.
    MayUnwind,
}

impl UnwindBehavior {
    /// Return true when the operation may unwind.
    pub fn may_unwind(self) -> bool {
        matches!(self, Self::MayUnwind)
    }
}

/// Suspend behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuspendBehavior {
    /// The operation cannot suspend execution.
    CannotSuspend,
    /// The operation may suspend execution.
    MaySuspend,
}

impl SuspendBehavior {
    /// Return true when the operation may suspend.
    pub fn may_suspend(self) -> bool {
        matches!(self, Self::MaySuspend)
    }
}

/// Return behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReturnBehavior {
    /// The operation may or may not return to the caller.
    MayReturn,
    /// The operation never returns to the caller.
    NoReturn,
    /// The operation is guaranteed to return to the caller.
    WillReturn,
}

impl ReturnBehavior {
    /// Return true when the operation never returns.
    pub fn is_no_return(self) -> bool {
        matches!(self, Self::NoReturn)
    }

    /// Return true when the operation is guaranteed to return.
    pub fn is_will_return(self) -> bool {
        matches!(self, Self::WillReturn)
    }
}

/// Memory-space and address-space scope for one allocation side effect.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationAccess {
    /// The memory spaces that may be touched.
    pub spaces: MemorySpaceSet,
    /// The address spaces that may be touched.
    pub address_spaces: Option<AddressSpaceSet>,
}

impl AllocationAccess {
    /// Create one unconstrained access summary.
    pub const fn unknown() -> Self {
        Self {
            spaces: MemorySpaceSet::ANY,
            address_spaces: None,
        }
    }
}

/// Allocation and free behavior for a call or function.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationEffect {
    /// Allocation behavior when the operation may allocate.
    pub allocate: Option<AllocationAccess>,
    /// Free behavior when the operation may free.
    pub free: Option<AllocationAccess>,
}

impl AllocationEffect {
    /// Create one behavior with no allocation side effects.
    pub const fn none() -> Self {
        Self {
            allocate: None,
            free: None,
        }
    }

    /// Create one conservative unknown allocation behavior.
    pub const fn unknown() -> Self {
        Self {
            allocate: Some(AllocationAccess::unknown()),
            free: Some(AllocationAccess::unknown()),
        }
    }
}

/// Behavioral effects for calls and functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallBehavior {
    /// Effect class for this operation.
    pub effect_class: EffectClass,
    /// Whether this operation may unwind.
    pub unwind: UnwindBehavior,
    /// Whether this operation may suspend execution.
    pub suspend: SuspendBehavior,
    /// Return behavior for this operation.
    pub return_behavior: ReturnBehavior,
    /// Whether optimization must not duplicate this operation.
    pub must_not_duplicate: bool,
    /// Allocation and free behavior for this operation.
    pub allocation: AllocationEffect,
}

impl CallBehavior {
    /// Create a behavior with no special effects.
    pub const fn none() -> Self {
        Self {
            effect_class: EffectClass::Deterministic,
            unwind: UnwindBehavior::CannotUnwind,
            suspend: SuspendBehavior::CannotSuspend,
            return_behavior: ReturnBehavior::MayReturn,
            must_not_duplicate: false,
            allocation: AllocationEffect::none(),
        }
    }

    /// Create a conservative unknown behavior.
    pub const fn unknown() -> Self {
        Self {
            effect_class: EffectClass::NonDeterministic,
            unwind: UnwindBehavior::MayUnwind,
            suspend: SuspendBehavior::CannotSuspend,
            return_behavior: ReturnBehavior::MayReturn,
            must_not_duplicate: false,
            allocation: AllocationEffect::unknown(),
        }
    }

    /// Create a pure behavior summary.
    pub const fn pure() -> Self {
        Self {
            effect_class: EffectClass::Pure,
            unwind: UnwindBehavior::CannotUnwind,
            suspend: SuspendBehavior::CannotSuspend,
            return_behavior: ReturnBehavior::WillReturn,
            must_not_duplicate: false,
            allocation: AllocationEffect::none(),
        }
    }

    /// Return this behavior with the may-suspend flag enabled.
    pub const fn with_suspend(mut self) -> Self {
        self.suspend = SuspendBehavior::MaySuspend;
        self
    }

    /// Return this behavior with the may-unwind flag enabled.
    pub const fn with_unwind(mut self) -> Self {
        self.unwind = UnwindBehavior::MayUnwind;
        self
    }

    /// Return this behavior with noreturn enabled.
    pub const fn with_noreturn(mut self) -> Self {
        self.return_behavior = ReturnBehavior::NoReturn;
        self
    }

    /// Return this behavior with will-return enabled.
    pub const fn with_will_return(mut self) -> Self {
        self.return_behavior = ReturnBehavior::WillReturn;
        self
    }

    /// Return this behavior with duplication disabled.
    pub const fn with_no_duplicate(mut self) -> Self {
        self.must_not_duplicate = true;
        self
    }
}

impl Default for CallBehavior {
    fn default() -> Self {
        Self::unknown()
    }
}
