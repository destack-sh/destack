use destack_mir as mir;

/// Track a lowered global binding for value expressions.
#[derive(Debug, Clone)]
pub(crate) struct GlobalBinding {
    /// The MIR global holding the binding value.
    pub(crate) global: mir::LocalNodeId<mir::Global>,
    /// The MIR type of the binding.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
    /// The mutability of the global.
    pub(crate) mutability: mir::Mutability,
    /// The address space of the global.
    pub(crate) space: mir::AddressSpace,
}

/// Storage for a lowered local binding.
#[derive(Debug, Clone, Copy)]
pub(crate) enum LocalStorage {
    /// An SSA variable binding.
    Variable(mir::Variable),
    /// A stack slot binding.
    Local(mir::LocalNodeId<mir::Local>),
    /// A boxed binding stored behind a reference.
    IndirectBinding {
        /// The SSA variable holding the reference.
        variable: mir::Variable,
        /// The MIR type of the reference value.
        reference_type: mir::LocalNodeId<mir::Type>,
    },
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
    pub(crate) fn variable(variable: mir::Variable, ty: mir::LocalNodeId<mir::Type>) -> Self {
        Self {
            storage: LocalStorage::Variable(variable),
            ty,
        }
    }

    /// Create a local binding backed by a stack slot.
    pub(crate) fn local(
        local: mir::LocalNodeId<mir::Local>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Self {
        Self {
            storage: LocalStorage::Local(local),
            ty,
        }
    }

    /// Create a local binding backed by a boxed reference.
    pub(crate) fn indirect_binding(
        variable: mir::Variable,
        reference_type: mir::LocalNodeId<mir::Type>,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Self {
        Self {
            storage: LocalStorage::IndirectBinding {
                variable,
                reference_type,
            },
            ty,
        }
    }
}
