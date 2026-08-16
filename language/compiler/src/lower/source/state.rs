use std::sync::Arc;

use destack_artifact::{
    DirBound, DirChecked, DirDeclared, DirElaborated, DirExpanded, DirMaterialized, DirParsed,
};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// The state of one module, read during lowering.
pub(crate) struct LowerModuleState {
    /// The artifact holding the expression tree.
    parsed: Arc<DirParsed>,
    /// The module roots.
    pub(in crate::lower) roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The type table.
    pub(in crate::lower) types: dir::TypeTable<'static>,
    /// The decision table.
    pub(in crate::lower) decisions: dir::DecisionTable<'static>,
    /// The resolution table.
    pub(in crate::lower) resolutions: dir::ResolutionTable<'static>,
    /// The binding table.
    pub(in crate::lower) bindings: dir::BindingTable<'static>,
    /// The coercion table.
    pub(in crate::lower) coercions: dir::CoercionTable<'static>,
    /// The definition table.
    pub(in crate::lower) definitions: dir::DefinitionTable<'static>,
    /// The static table.
    pub(in crate::lower) statics: dir::StaticTable<'static>,
    /// The generic table.
    pub(in crate::lower) generics: dir::GenericTable<'static>,
    /// The decorator table.
    pub(in crate::lower) decorators: dir::DecoratorTable<'static>,
    /// The capture table.
    pub(in crate::lower) captures: dir::CaptureTable<'static>,
    /// The canonical symbol path of the module.
    pub(in crate::lower) path: String,
}

impl LowerModuleState {
    /// Load the state of one module.
    pub(crate) fn new(
        parsed: Arc<DirParsed>,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
        checked: &DirChecked,
        materialized: &DirMaterialized,
        path: String,
    ) -> Self {
        Self {
            roots: materialized.roots.to_vec(),
            types: materialized.type_table(bound, expanded, declared, elaborated, checked),
            resolutions: checked.resolution_table(declared, elaborated),
            decisions: materialized.decision_table(declared, elaborated, checked),
            bindings: checked.binding_table(bound, expanded, declared, elaborated),
            coercions: materialized.coercion_table(checked),
            definitions: materialized.definition_table(elaborated, checked),
            statics: checked.static_table(bound, expanded, declared, elaborated),
            generics: materialized.generic_table(declared, elaborated, checked),
            decorators: checked.decorator_table(elaborated),
            captures: checked.capture_table(),
            path,
            parsed,
        }
    }

    /// Return the expression tree.
    pub(in crate::lower) fn tree(&self) -> &dir::Tree {
        &self.parsed.tree
    }
}

impl ModuleLowerer<'_> {
    /// Return the state of one loaded module.
    pub(in crate::lower) fn state(&self, module: ModuleId) -> CompilerResult<&LowerModuleState> {
        self.modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("reference to the unloaded module {module:?}"),
            })
    }

    /// Return the state of the module being lowered.
    pub(in crate::lower) fn local(&self) -> &LowerModuleState {
        match self.modules.get(&self.module) {
            Some(state) => state,
            None => unreachable!("the lowered module is always loaded"),
        }
    }
}
