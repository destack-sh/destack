use destack_mir as mir;

/// Track whether a statement terminates control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Terminates {
    /// The statement terminates control flow.
    Yes,
    /// The statement falls through to the next block.
    No,
}

impl Terminates {
    /// Return true if the statement terminates control flow.
    pub(crate) fn is_yes(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// Track a lowered local binding for value expressions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalBinding {
    /// The MIR variable holding the binding value.
    pub(crate) variable: mir::Variable,
    /// The MIR type of the binding.
    pub(crate) ty: mir::LocalNodeId<mir::Type>,
}

/// Track loop context for break/continue resolution.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoopContext {
    /// Block to jump to on continue (loop header or increment block).
    pub(crate) continue_block: mir::LocalNodeId<mir::Block>,
    /// Block to jump to on break (loop exit).
    pub(crate) break_block: mir::LocalNodeId<mir::Block>,
}

/// Track break context for non-loop control flow.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BreakContext {
    /// Block to jump to on break.
    pub(crate) break_block: mir::LocalNodeId<mir::Block>,
}
