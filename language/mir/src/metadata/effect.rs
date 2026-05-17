use serde::{Deserialize, Serialize};

use crate::SpaceSet;

/// Memory effect summary for a call or operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryEffect {
    /// Whether the operation may read memory.
    pub reads: bool,
    /// Whether the operation may write memory.
    pub writes: bool,
    /// The memory spaces that may be accessed.
    pub spaces: SpaceSet,
}

impl MemoryEffect {
    /// Create an effect with no memory access.
    pub const fn none() -> Self {
        Self {
            reads: false,
            writes: false,
            spaces: SpaceSet::NONE,
        }
    }

    /// Create a read only effect over the provided spaces.
    pub const fn read_only(spaces: SpaceSet) -> Self {
        Self {
            reads: true,
            writes: false,
            spaces,
        }
    }

    /// Create a write only effect over the provided spaces.
    pub const fn write_only(spaces: SpaceSet) -> Self {
        Self {
            reads: false,
            writes: true,
            spaces,
        }
    }

    /// Create a read write effect over the provided spaces.
    pub const fn read_write(spaces: SpaceSet) -> Self {
        Self {
            reads: true,
            writes: true,
            spaces,
        }
    }

    /// Create a conservative unknown effect.
    pub const fn unknown() -> Self {
        Self {
            reads: true,
            writes: true,
            spaces: SpaceSet::ANY,
        }
    }
}

impl Default for MemoryEffect {
    fn default() -> Self {
        Self::unknown()
    }
}

/// Determinism for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Determinism {
    /// The operation is deterministic for the same inputs and runtime state.
    Deterministic,
    /// The operation may observe entropy, time, scheduling, or host state.
    NonDeterministic,
}

impl Determinism {
    /// Return true when the operation is deterministic.
    pub fn is_deterministic(self) -> bool {
        matches!(self, Self::Deterministic)
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

/// Panic behavior for a call or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanicBehavior {
    /// The operation cannot panic.
    CannotPanic,
    /// The operation may panic and unwind cleanup.
    MayPanic,
}

impl PanicBehavior {
    /// Return true when the operation may panic.
    pub fn may_panic(self) -> bool {
        matches!(self, Self::MayPanic)
    }
}

/// Behavioral effects for calls and functions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FunctionBehavior {
    /// Determinism for this operation.
    pub determinism: Determinism,
    /// Whether this operation may suspend execution.
    pub suspend: SuspendBehavior,
    /// Panic behavior for this operation.
    pub panic: PanicBehavior,
    /// Return behavior for this operation.
    pub return_behavior: ReturnBehavior,
    /// Whether optimization must not duplicate this operation.
    pub must_not_duplicate: bool,
    /// Whether this operation may allocate storage.
    pub allocates: bool,
    /// Whether this operation may free storage.
    pub frees: bool,
}

impl FunctionBehavior {
    /// Create a behavior with no special effects.
    pub const fn none() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            suspend: SuspendBehavior::CannotSuspend,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::MayReturn,
            must_not_duplicate: false,
            allocates: false,
            frees: false,
        }
    }

    /// Create a conservative unknown behavior.
    pub const fn unknown() -> Self {
        Self {
            determinism: Determinism::NonDeterministic,
            suspend: SuspendBehavior::CannotSuspend,
            panic: PanicBehavior::MayPanic,
            return_behavior: ReturnBehavior::MayReturn,
            must_not_duplicate: false,
            allocates: true,
            frees: true,
        }
    }

    /// Create a pure behavior summary.
    pub const fn pure() -> Self {
        Self {
            determinism: Determinism::Deterministic,
            suspend: SuspendBehavior::CannotSuspend,
            panic: PanicBehavior::CannotPanic,
            return_behavior: ReturnBehavior::WillReturn,
            must_not_duplicate: false,
            allocates: false,
            frees: false,
        }
    }

    /// Return this behavior with the may-suspend flag enabled.
    pub const fn with_suspend(mut self) -> Self {
        self.suspend = SuspendBehavior::MaySuspend;
        self
    }

    /// Return this behavior with the may-panic flag enabled.
    pub const fn with_panic(mut self) -> Self {
        self.panic = PanicBehavior::MayPanic;
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

    /// Return this behavior with allocation enabled.
    pub const fn with_allocates(mut self) -> Self {
        self.allocates = true;
        self
    }

    /// Return this behavior with free enabled.
    pub const fn with_frees(mut self) -> Self {
        self.frees = true;
        self
    }
}

impl Default for FunctionBehavior {
    fn default() -> Self {
        Self::unknown()
    }
}
