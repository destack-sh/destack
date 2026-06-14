/// The kinds of IR a pass changed, used to invalidate analyses.
///
/// A pass returns the union of what it changed; each analysis declares the kinds
/// that invalidate it via [`Analysis::INVALIDATED_BY`](super::Analysis::INVALIDATED_BY).
/// The cache drops every analysis whose invalidation mask intersects a pass's
/// reported change, then cascades to dependents through the dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mutation(u8);

impl Mutation {
    /// Nothing changed; every analysis stays valid.
    pub const NONE: Self = Self(0);
    /// Control flow changed: blocks added or removed, or edge topology rewritten.
    pub const CONTROL_FLOW: Self = Self(1 << 0);
    /// Values changed: instructions or operands added, removed, or rewritten.
    pub const VALUES: Self = Self(1 << 1);
    /// Everything changed; every analysis is invalidated.
    pub const ALL: Self = Self(0b11);

    /// Return the union of two change sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return true when this change set shares any kind with `other`.
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return true when nothing changed.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Mutation {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}
