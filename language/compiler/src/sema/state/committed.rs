use destack_dir as dir;
use destack_source::ModuleId;

use super::CheckState;

/// The committed tables one loaded module's declarations are read through, own or foreign.
pub(in crate::sema) struct CommittedModule<'a> {
    /// The parsed tree.
    pub(in crate::sema) tree: &'a dir::Tree,
    /// The binding table.
    pub(in crate::sema) bindings: dir::BindingTable<'a>,
    /// The committed types.
    pub(in crate::sema) types: &'a dir::TypeTable<'static>,
    /// The committed generics.
    pub(in crate::sema) generics: &'a dir::GenericTable<'static>,
    /// The committed decisions.
    pub(in crate::sema) decisions: &'a dir::DecisionTable<'static>,
    /// The committed coercions.
    pub(in crate::sema) coercions: &'a dir::CoercionTable<'static>,
    /// The committed definitions.
    pub(in crate::sema) definitions: &'a dir::DefinitionTable<'static>,
}

impl CheckState<'_> {
    /// Return one loaded module's committed tables, the checked module's beneath its open tails.
    pub(in crate::sema) fn committed(&self, module: ModuleId) -> Option<CommittedModule<'_>> {
        // read the checked module's committed bases
        if let Some(own) = self.module_maybe(module) {
            return Some(CommittedModule {
                tree: &own.parsed.tree,
                bindings: own.binding_table(),
                types: &own.types,
                generics: &own.generics,
                decisions: &own.decisions,
                coercions: &own.coercions,
                definitions: &own.definitions,
            });
        }

        // otherwise read the loaded foreign module
        let external = self.external_modules.get(&module)?;

        Some(CommittedModule {
            tree: &external.parsed.tree,
            bindings: external.bindings.clone(),
            types: &external.types,
            generics: &external.generics,
            decisions: &external.decisions,
            coercions: &external.coercions,
            definitions: &external.definitions,
        })
    }
}
