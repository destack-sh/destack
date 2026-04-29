/// Borrow checking mode for ownership references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowMode {
    /// Hint mode with warnings only.
    Hint,
    /// Strict mode with errors on violations.
    Strict,
}

impl BorrowMode {
    /// Whether this mode is strict.
    pub fn is_strict(self) -> bool {
        matches!(self, BorrowMode::Strict)
    }

    /// Whether this mode is stricter than another mode.
    pub fn is_stricter_than(self, other: BorrowMode) -> bool {
        matches!((self, other), (BorrowMode::Strict, BorrowMode::Hint))
    }
}

pub(crate) use destack_artifact::{Platform, Runtime, TargetAbi, TargetArch, TargetVendor};
