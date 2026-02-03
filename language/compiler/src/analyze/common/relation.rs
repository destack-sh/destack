/// The kind of relation being checked.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationKind {
    /// Assignability checks (like `T` assignable to `U`).
    Assignable,
    /// Comparable checks (like equality or overlap queries).
    Comparable,
    /// Subtype checks used by constraints.
    Subtype,
    /// Identity checks used for strict equality.
    Identical,
    /// Constraint checks for bounds and inference.
    Constraint,
}

/// Relation flags used to control contextual type behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RelationFlags {
    /// Whether apparent types should be resolved.
    pub use_apparent_type: bool,
    /// Whether fresh literals should be preserved.
    pub allow_fresh_literals: bool,
    /// Whether widening should be avoided.
    pub prefer_non_widening: bool,
    /// Whether contextual typing should be applied.
    pub use_contextual_type: bool,
}

impl RelationFlags {
    /// Relation flags for assignability style relations.
    pub(crate) const ASSIGN: Self = Self {
        use_apparent_type: true,
        allow_fresh_literals: false,
        prefer_non_widening: false,
        use_contextual_type: false,
    };

    /// Relation flags for type operations that query keys and shapes.
    pub(crate) const TYPE_OPS: Self = Self {
        use_apparent_type: true,
        allow_fresh_literals: false,
        prefer_non_widening: false,
        use_contextual_type: false,
    };
}

/// Relation mode used to control contextual type behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RelationMode {
    /// The relation kind.
    pub kind: RelationKind,
    /// The relation flags.
    pub flags: RelationFlags,
}

impl RelationMode {
    /// Relation mode for assignability style relations.
    pub(crate) const ASSIGN: Self = Self {
        kind: RelationKind::Assignable,
        flags: RelationFlags::ASSIGN,
    };

    /// Relation mode for type operations that query keys and shapes.
    pub(crate) const TYPE_OPS: Self = Self {
        kind: RelationKind::Constraint,
        flags: RelationFlags::TYPE_OPS,
    };

    /// Return a cache key representing this relation mode.
    pub(crate) fn cache_key(self) -> u64 {
        if self == Self::ASSIGN {
            return 0;
        }

        let kind_key = match self.kind {
            RelationKind::Assignable => 1_u64,
            RelationKind::Comparable => 2_u64,
            RelationKind::Subtype => 3_u64,
            RelationKind::Identical => 4_u64,
            RelationKind::Constraint => 5_u64,
        };

        // build a compact bitset for flags and kind
        let mut key = kind_key;
        key |= (self.flags.use_apparent_type as u64) << 8;
        key |= (self.flags.allow_fresh_literals as u64) << 9;
        key |= (self.flags.prefer_non_widening as u64) << 10;
        key |= (self.flags.use_contextual_type as u64) << 11;
        key
    }

    /// Return true when normalization caching is valid for this relation mode.
    pub(crate) fn is_cacheable(self) -> bool {
        // NOTE #Suspicious: only ASSIGN is cacheable even though TYPE_OPS is deterministic
        self == Self::ASSIGN
    }
}
