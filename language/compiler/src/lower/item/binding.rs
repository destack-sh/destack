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
