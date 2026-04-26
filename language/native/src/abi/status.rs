/// Status returned by native entry functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u32)]
pub enum Status {
    /// Execution completed normally.
    Completed = 0,
    /// Execution yielded a continuation.
    Yielded = 1,
    /// Execution trapped.
    Trapped = 2,
}

/// Runtime trap code reported by generated native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u32)]
pub enum Trap {
    /// Integer arithmetic overflowed.
    IntegerOverflow = 1,
    /// A checked cast failed.
    InvalidCast = 2,
    /// A bounds check failed.
    Bounds = 3,
    /// A null reference was used.
    NullReference = 4,
    /// An unreachable block executed.
    Unreachable = 5,
}
