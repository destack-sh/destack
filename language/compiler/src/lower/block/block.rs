use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::super::TypeLowerer;

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

/// Lower statement-level expressions into MIR blocks.
pub(crate) struct BlockLowerer<'a, 'b> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Provide access to the program string pool for name resolution.
    pub(crate) strings: &'a StringPool,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Emit MIR into the current function builder.
    pub(crate) builder: &'b mut mir::FunctionBuilder<'a>,
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: &'b mut HashMap<GlobalSymbolId, LocalBinding>,
    /// Track loop contexts by symbol for labeled break/continue.
    pub(crate) loops_by_symbol: &'b mut HashMap<GlobalSymbolId, LoopContext>,
    /// Track loop nesting for unlabeled break/continue.
    pub(crate) loop_stack: &'b mut Vec<LoopContext>,
}
