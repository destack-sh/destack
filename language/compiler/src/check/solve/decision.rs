/// A relation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Decision {
    /// The relation is true.
    Yes,
    /// The relation is false.
    No,
    /// The relation cannot be decided from current information.
    Undecidable,
}

impl Decision {
    /// Combine decisions that must both hold.
    pub(in crate::check) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Undecidable, _) | (_, Self::Undecidable) => Self::Undecidable,
            (Self::Yes, Self::Yes) => Self::Yes,
        }
    }
}
