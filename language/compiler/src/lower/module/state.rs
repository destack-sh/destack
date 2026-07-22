use std::sync::Arc;

use destack_artifact::{DirBound, DirCheckedModule, DirExpanded, DirMaterialized, DirParsed};
use destack_dir as dir;

/// The sealed check output of one module, read during lowering.
pub(crate) struct LowerModuleState {
    /// The parsed DIR artifact holding the tree.
    parsed: Arc<DirParsed>,
    /// The DIR roots.
    pub(in crate::lower) roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The checked type table.
    pub(in crate::lower) types: dir::TypeTable<'static>,
    /// The checked resolution table.
    pub(in crate::lower) resolutions: dir::ResolutionTable<'static>,
    /// The checked binding table.
    pub(in crate::lower) bindings: dir::BindingTable<'static>,
    /// The checked coercion table.
    pub(in crate::lower) coercions: dir::CoercionTable<'static>,
    /// The checked definition table.
    pub(in crate::lower) definitions: dir::DefinitionTable<'static>,
    /// The checked static table.
    pub(in crate::lower) statics: dir::StaticTable<'static>,
    /// The checked generic table.
    pub(in crate::lower) generics: dir::GenericTable<'static>,
    /// The checked decorator table.
    pub(in crate::lower) decorators: dir::DecoratorTable<'static>,
    /// The canonical symbol path of the module.
    pub(in crate::lower) path: String,
}

impl LowerModuleState {
    /// Load the sealed check output of one module.
    pub(crate) fn new(
        parsed: Arc<DirParsed>,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
        path: String,
    ) -> Self {
        Self {
            roots: materialized.roots.to_vec(),
            types: materialized.type_table(bound, expanded, checked),
            resolutions: materialized.resolution_table(checked),
            bindings: materialized.binding_table(bound, expanded),
            coercions: materialized.coercion_table(checked),
            definitions: materialized.definition_table(checked),
            statics: materialized.static_table(bound, expanded, checked),
            generics: materialized.generic_table(checked),
            decorators: checked.decorator_table(),
            path,
            parsed,
        }
    }

    /// Return the DIR tree.
    pub(in crate::lower) fn tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }
}
