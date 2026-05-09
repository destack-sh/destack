use destack_dir as dir;
use destack_dir::GuardTable;
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, ProviderContext};

use crate::elaborate::ElaborateOptions;

/// Mutable state for one elaborate phase run.
pub(crate) struct ElaborateState<'a> {
    /// The provider context for this elaboration.
    pub(crate) provider: &'a dyn ProviderContext,
    /// The active module id.
    pub(crate) module_id: ModuleId,
    /// The active module data.
    pub(crate) module: &'a Module,
    /// The active profile id.
    pub(crate) profile: ProfileId,
    /// Options for this elaborate pass.
    pub(crate) options: ElaborateOptions,
    /// The local DIR tree.
    pub(crate) tree: &'a mut dir::Tree,
    /// The local symbol table.
    pub(crate) symbols: &'a mut dir::BindingTable,
    /// The local type table.
    pub(crate) types: &'a mut dir::TypeTable,
    /// Elaborated type guard entries.
    pub(crate) guards: &'a mut GuardTable,
}

impl<'a> ElaborateState<'a> {
    /// Construct mutable elaborate state.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        provider: &'a dyn ProviderContext,
        module_id: ModuleId,
        module: &'a Module,
        profile: ProfileId,
        options: ElaborateOptions,
        tree: &'a mut dir::Tree,
        symbols: &'a mut dir::BindingTable,
        types: &'a mut dir::TypeTable,
        guards: &'a mut GuardTable,
    ) -> Self {
        Self {
            provider,
            module_id,
            module,
            profile,
            options,
            tree,
            symbols,
            types,
            guards,
        }
    }
}
