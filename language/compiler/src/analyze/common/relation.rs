/// Relation mode flags used to control contextual type behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RelationMode {
    /// Whether apparent types should be resolved.
    pub use_apparent_type: bool,
    /// Whether static parameter constraints may substitute apparent shapes.
    pub substitute_constraints_in_apparent_type: bool,
}

impl RelationMode {
    /// Relation mode for assignability style relations.
    pub(crate) const ASSIGN: Self = Self {
        use_apparent_type: true,
        substitute_constraints_in_apparent_type: false,
    };

    /// Relation mode for type operations that query keys and shapes.
    pub(crate) const TYPE_OPS: Self = Self {
        use_apparent_type: true,
        substitute_constraints_in_apparent_type: false,
    };
}
