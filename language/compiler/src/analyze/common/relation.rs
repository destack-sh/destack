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
    /// Whether static parameter constraints may substitute apparent shapes.
    pub substitute_constraints_in_apparent_type: bool,
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
        substitute_constraints_in_apparent_type: false,
        allow_fresh_literals: false,
        prefer_non_widening: false,
        use_contextual_type: false,
    };

    /// Relation flags for type operations that query keys and shapes.
    pub(crate) const TYPE_OPS: Self = Self {
        use_apparent_type: true,
        substitute_constraints_in_apparent_type: false,
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
}
