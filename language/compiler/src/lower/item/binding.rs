use destack_mir as mir;

/// Track a lowered global binding for value expressions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlobalBinding {
    /// The MIR global holding the binding value.
    pub(crate) global: mir::LocalNodeId<mir::Global>,
    /// The MIR type of the binding.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
    /// The mutability of the global.
    pub(crate) mutability: mir::Mutability,
}

/// Storage for a lowered local binding.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LocalStorage {
    /// An SSA variable binding.
    Variable(mir::Variable),
    /// A stack slot binding.
    Local(mir::LocalNodeId<mir::Local>),
}

/// Track a lowered local binding for value expressions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalBinding {
    /// The storage used for the binding.
    pub(crate) storage: LocalStorage,
    /// The MIR type of the binding.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
}

impl LocalBinding {
    /// Create a local binding backed by an SSA variable.
    pub(crate) fn from_variable(variable: mir::Variable, ty: mir::LocalNodeId<mir::Type>) -> Self {
        Self {
            storage: LocalStorage::Variable(variable),
            ty,
        }
    }

    /// Create a local binding backed by a stack slot.
    pub(crate) fn from_local(
        local: mir::LocalNodeId<mir::Local>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Self {
        Self {
            storage: LocalStorage::Local(local),
            ty,
        }
    }
}
