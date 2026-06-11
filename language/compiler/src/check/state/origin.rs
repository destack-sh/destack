use destack_dir as dir;
use destack_source::ModuleId;

/// Source location that produced check work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Origin {
    /// Work came from one source node.
    Node(dir::GlobalNodeIdAny),
    /// Work came from one source symbol.
    Symbol(dir::GlobalSymbolId),
    /// Work came from one committed type.
    Type(dir::GlobalTypeId),
}

impl Origin {
    /// Return the source module that produced this work.
    pub(in crate::check) fn module(self) -> ModuleId {
        match self {
            Self::Node(node) => node.module_id,
            Self::Symbol(symbol) => symbol.module_id,
            Self::Type(ty) => ty.module_id,
        }
    }
}
