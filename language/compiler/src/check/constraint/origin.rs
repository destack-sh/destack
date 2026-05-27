use destack_dir as dir;
use destack_source::ModuleId;

/// Source location that produced one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintOrigin {
    /// Constraint came from one source node.
    Node(dir::GlobalNodeIdAny),
    /// Constraint came from one source symbol.
    Symbol(dir::GlobalSymbolId),
}

impl ConstraintOrigin {
    /// Return the source module that produced this constraint.
    pub(in crate::check) fn module(self) -> ModuleId {
        match self {
            Self::Node(node) => node.module_id,
            Self::Symbol(symbol) => symbol.module_id,
        }
    }
}
