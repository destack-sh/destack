/// Combined MIR input changes used to invalidate cached analyses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mutation(u8);

impl Mutation {
    /// Nothing changed; every analysis stays valid.
    pub const NONE: Self = Self(0);
    /// Block membership or order, entry blocks, or successor targets or order changed.
    pub const CONTROL: Self = Self(1 << 0);
    /// Instructions, operands, their order, parameters, locals, or globals changed.
    pub const VALUE: Self = Self(1 << 1);
    /// Memory access metadata changed.
    pub const MEMORY: Self = Self(1 << 2);
    /// Effect tables changed.
    pub const EFFECT: Self = Self(1 << 3);
    /// Type definitions, recorded value types, or physical layouts changed.
    pub const LAYOUT: Self = Self(1 << 4);
    /// Symbol visibility or linkage changed.
    pub const SYMBOL: Self = Self(1 << 5);
    /// Dispatch targets or witness implementations changed.
    pub const DISPATCH: Self = Self(1 << 6);
    /// Destruction requirements or referenced destructors changed.
    pub const DROP: Self = Self(1 << 7);
    /// Everything changed; every analysis is invalidated.
    pub const ALL: Self = Self(u8::MAX);

    /// Return the union of two change sets.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Keep only changes present in both sets.
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
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
