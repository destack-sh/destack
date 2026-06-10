use destack_dir as dir;
use destack_dir::GuardTable;
use destack_repository::{ArtifactReader, Module, ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::elaborate::ElaborateOptions;

/// Mutable state for one elaborate phase run.
pub(crate) struct ElaborateState<'a> {
    /// The provider context for this elaboration.
    pub(crate) provider: &'a dyn ProviderContext,
    /// The provider-scoped artifact reader.
    pub(crate) artifacts: &'a ArtifactReader<'a>,
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
    pub(crate) symbols: &'a mut dir::BindingTable<'static>,
    /// The readable type table from prior phases.
    pub(crate) types: &'a dir::TypeTable<'static>,
    /// The local type segment.
    pub(crate) types_tail: &'a mut dir::TypeSegment,
    /// The readable resolution table from prior phases.
    pub(crate) resolutions: &'a dir::ResolutionTable<'static>,
    /// The local resolution segment.
    pub(crate) resolutions_tail: &'a mut dir::ResolutionSegment,
    /// Elaborated type guard entries.
    pub(crate) guards: &'a mut GuardTable,
}

impl<'a> ElaborateState<'a> {
    /// Construct mutable elaborate state.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        provider: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        module_id: ModuleId,
        module: &'a Module,
        profile: ProfileId,
        options: ElaborateOptions,
        tree: &'a mut dir::Tree,
        symbols: &'a mut dir::BindingTable<'static>,
        types: &'a dir::TypeTable<'static>,
        types_tail: &'a mut dir::TypeSegment,
        resolutions: &'a dir::ResolutionTable<'static>,
        resolutions_tail: &'a mut dir::ResolutionSegment,
        guards: &'a mut GuardTable,
    ) -> Self {
        Self {
            provider,
            artifacts,
            module_id,
            module,
            profile,
            options,
            tree,
            symbols,
            types,
            types_tail,
            resolutions,
            resolutions_tail,
            guards,
        }
    }

    /// Return the visible type table for this elaborate run.
    pub(crate) fn type_table(&self) -> dir::TypeTable<'_> {
        self.types.with_tail(self.types_tail)
    }

    /// Return the visible resolution table for this elaborate run.
    pub(crate) fn resolution_table(&self) -> dir::ResolutionTable<'_> {
        self.resolutions.with_tail(self.resolutions_tail)
    }
}
