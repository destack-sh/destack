use tspp_dir as dir;
use tspp_source::ModuleId;

use super::CheckState;
use crate::CompilerResult;

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
    /// The committed decisions, checked bodies writing them.
    pub(in crate::sema) decisions: Option<&'a dir::DecisionTable<'static>>,
    /// The committed coercions, checked bodies writing them.
    pub(in crate::sema) coercions: Option<&'a dir::CoercionTable<'static>>,
    /// The committed definitions.
    pub(in crate::sema) definitions: &'a dir::DefinitionTable<'static>,
}

impl CheckState<'_> {
    /// Return one loaded module's committed tables, the checked module's beneath its open tails.
    pub(in crate::sema) fn committed(
        &self,
        module: ModuleId,
    ) -> CompilerResult<Option<CommittedModule<'_>>> {
        // read the checked module's committed bases
        if self.is_own_module(module) {
            let own = &self.module;

            return Ok(Some(CommittedModule {
                tree: &own.parsed.tree,
                bindings: own.binding_table(),
                types: own.types,
                generics: &own.generics,
                decisions: Some(&own.decisions),
                coercions: Some(&own.coercions),
                definitions: &own.definitions,
            }));
        }

        // otherwise read the foreign module at its stage
        let Some(external) = self.external(module)? else {
            return Ok(None);
        };

        Ok(Some(CommittedModule {
            tree: &external.parsed().tree,
            bindings: external.bindings().clone(),
            types: external.types(),
            generics: external.generics(),
            decisions: external.checked.as_ref().map(|_| external.decisions()),
            coercions: external.checked.as_ref().map(|_| external.coercions()),
            definitions: external.definitions(),
        }))
    }
}
