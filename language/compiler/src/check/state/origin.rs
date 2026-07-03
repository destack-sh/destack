use destack_dir as dir;
use destack_source::ModuleId;

/// Source location that produced check work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Origin {
    /// Work came from one source node under one assuming template.
    Node(dir::GlobalNodeIdAny, Option<dir::GlobalGenericTemplateId>),
    /// Work came from one source symbol.
    Symbol(dir::GlobalSymbolId),
}

impl Origin {
    /// Return the source module that produced this work.
    pub(in crate::check) fn module(self) -> ModuleId {
        match self {
            Self::Node(node, _) => node.module_id,
            Self::Symbol(symbol) => symbol.module_id,
        }
    }

    /// Return the expression node that produced this work.
    pub(in crate::check) fn expression(self) -> Option<dir::GlobalNodeId<dir::Expression>> {
        let Self::Node(node, _) = self else {
            return None;
        };

        node.try_into_typed().ok()
    }
}
