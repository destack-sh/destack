use destack_dir as dir;

/// Source location that produced one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintOrigin {
    /// Constraint came from one source node.
    Node(dir::GlobalNodeIdAny),
    /// Constraint came from one source symbol.
    Symbol(dir::GlobalSymbolId),
}
