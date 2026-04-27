use crate::CompilerContext;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

/// Shared immutable elaborate phase identity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ElaborateContext<'a> {
    /// The pinned compiler context for this elaboration.
    pub compiler_context: &'a CompilerContext<'a>,
    /// The active module id.
    pub module_id: ModuleId,
    /// The active module data.
    pub module: &'a Module,
    /// The active profile id.
    pub profile: ProfileId,
}

impl<'a> ElaborateContext<'a> {
    /// Construct an elaborate context from module phase inputs.
    pub(crate) fn new(
        compiler_context: &'a CompilerContext<'a>,
        module_id: ModuleId,
        module: &'a Module,
        profile: ProfileId,
    ) -> Self {
        Self {
            compiler_context,
            module_id,
            module,
            profile,
        }
    }
}

/// Shared mutable elaborate state.
pub(crate) struct ElaborateState<'a> {
    /// The active phase identity.
    pub ctx: ElaborateContext<'a>,
    /// The local DIR tree.
    pub tree: &'a mut dir::NodeTree,
    /// The local symbol table.
    pub symbols: &'a mut dir::SymbolTable,
    /// The local type table.
    pub types: &'a mut dir::TypeTable,
}

impl<'a> ElaborateState<'a> {
    /// Construct mutable elaborate state.
    pub(crate) fn new(
        ctx: ElaborateContext<'a>,
        tree: &'a mut dir::NodeTree,
        symbols: &'a mut dir::SymbolTable,
        types: &'a mut dir::TypeTable,
    ) -> Self {
        Self {
            ctx,
            tree,
            symbols,
            types,
        }
    }
}
