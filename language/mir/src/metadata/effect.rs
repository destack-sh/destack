use crate::AddressSpace;

/// Set of memory locations that an operation may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLocationSet(
    /// Bitset describing accessible memory locations.
    u16,
);

/// Set of address spaces that an operation may access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
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
}

impl MemoryLocationSet {
    /// No memory locations.
    pub const NONE: Self = Self(0);
    /// Memory reachable from pointer arguments.
    pub const ARGUMENTS: Self = Self(1 << 0);
    /// Heap allocated memory.
    pub const HEAP: Self = Self(1 << 1);
    /// Stack memory.
    pub const STACK: Self = Self(1 << 2);
    /// Global or static memory.
    pub const GLOBAL: Self = Self(1 << 3);
    /// Shared or workgroup memory.
    pub const SHARED: Self = Self(1 << 4);
    /// Target local or thread local memory.
    pub const LOCAL: Self = Self(1 << 5);
    /// Target constant or read only memory.
    pub const CONSTANT: Self = Self(1 << 6);
    /// Inaccessible memory that cannot be aliased.
    pub const INACCESSIBLE: Self = Self(1 << 7);
    /// Memory mapped IO or other side channel memory.
    pub const IO: Self = Self(1 << 8);
    /// All memory locations.
    pub const ANY: Self = Self(
        Self::ARGUMENTS.0
            | Self::HEAP.0
            | Self::STACK.0
            | Self::GLOBAL.0
            | Self::SHARED.0
            | Self::LOCAL.0
            | Self::CONSTANT.0
            | Self::INACCESSIBLE.0
            | Self::IO.0,
    );

    /// Check if the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Check if this set contains the other set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Insert another set of locations.
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl Default for MemoryLocationSet {
    fn default() -> Self {
        Self::ANY
    }
}

/// Memory effect summary for a call or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryEffect {
    /// Whether the operation may read memory.
    pub reads: bool,
    /// Whether the operation may write memory.
    pub writes: bool,
    /// The memory locations that may be accessed.
    pub locations: MemoryLocationSet,
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
            locations: MemoryLocationSet::NONE,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a read only effect over the provided locations.
    pub const fn read_only(locations: MemoryLocationSet) -> Self {
        Self {
            reads: true,
            writes: false,
            locations,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a write only effect over the provided locations.
    pub const fn write_only(locations: MemoryLocationSet) -> Self {
        Self {
            reads: false,
            writes: true,
            locations,
            address_spaces: None,
            argmemonly: false,
            inaccessible_mem_only: false,
            nosync: false,
        }
    }

    /// Create a read write effect over the provided locations.
    pub const fn read_write(locations: MemoryLocationSet) -> Self {
        Self {
            reads: true,
            writes: true,
            locations,
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
            locations: MemoryLocationSet::ANY,
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

/// Behavioral effects for calls and functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallBehavior {
    /// The call never returns to the caller.
    pub noreturn: bool,
    /// The call is guaranteed to return eventually.
    pub will_return: bool,
    /// The call is convergent and cannot be arbitrarily duplicated.
    pub convergent: bool,
    /// The call may allocate memory.
    pub allocates: bool,
    /// The memory locations that may be allocated.
    pub alloc_locations: Option<MemoryLocationSet>,
    /// The address spaces that allocations may use.
    pub alloc_address_spaces: Option<AddressSpaceSet>,
    /// The call may free memory.
    pub frees: bool,
    /// The memory locations that may be freed.
    pub free_locations: Option<MemoryLocationSet>,
    /// The address spaces that frees may touch.
    pub free_address_spaces: Option<AddressSpaceSet>,
}

impl CallBehavior {
    /// Create a behavior with no special effects.
    pub const fn none() -> Self {
        Self {
            noreturn: false,
            will_return: false,
            convergent: false,
            allocates: false,
            alloc_locations: None,
            alloc_address_spaces: None,
            frees: false,
            free_locations: None,
            free_address_spaces: None,
        }
    }

    /// Create a conservative unknown behavior.
    pub const fn unknown() -> Self {
        Self {
            noreturn: false,
            will_return: false,
            convergent: false,
            allocates: true,
            alloc_locations: None,
            alloc_address_spaces: None,
            frees: true,
            free_locations: None,
            free_address_spaces: None,
        }
    }
}

impl Default for CallBehavior {
    fn default() -> Self {
        Self::unknown()
    }
}
