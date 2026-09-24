use std::sync::Arc;

use destack_artifact::{DirExported, DirResolved, DirView};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::ContextError;

/// Checked DIR state for one module.
#[derive(Debug)]
pub struct ModuleContext {
    /// The stacked module DIR stages.
    stages: DirView,
    /// The resolved import and source-reference DIR.
    resolved: Arc<DirResolved>,
    /// The exported module DIR.
    exported: Arc<DirExported>,
    /// The cumulative checked binding table.
    bindings: dir::BindingTable<'static>,
    /// The cumulative checked type table.
    types: dir::TypeTable<'static>,
    /// The cumulative checked static table.
    statics: dir::StaticTable<'static>,
    /// The checked resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The checked decisions table.
    decisions: dir::DecisionTable<'static>,
    /// The checked generic declarations and derived variances.
    generics: dir::GenericTable<'static>,
    /// The module id.
    module: ModuleId,
}

impl ModuleContext {
    /// Build one module context from a coherent checked artifact set.
    pub fn new(view: DirView, exported: Arc<DirExported>) -> Result<Self, ContextError> {
        let bound = &view.bound;
        let expanded = &view.expanded;
        let resolved = Arc::clone(view.resolved.as_ref().unwrap_or_else(|| unreachable!()));
        let checked = view.checked.as_ref().unwrap_or_else(|| unreachable!());
        let module = checked.bindings.module_id;
        let artifact_modules = [
            bound.bindings.module_id,
            bound.types.module_id,
            bound.statics.module_id,
            expanded.bindings.module_id,
            expanded.modules.module_id,
            expanded.types.module_id,
            expanded.statics.module_id,
            exported.exports.module_id,
            exported.globals.module_id,
            resolved.imports.module_id,
            resolved.references.module_id,
            resolved.extensions.module_id,
            checked.bindings.module_id,
            checked.types.module_id,
            checked.statics.module_id,
            checked.resolutions.module_id,
            checked.generics.module_id,
            checked.decorators.module_id,
            checked.coercions.module_id,
            checked.captures.module_id,
        ];
        if artifact_modules
            .iter()
            .any(|artifact_module| *artifact_module != module)
        {
            return Err(ContextError::MismatchedModule);
        }

        Ok(Self {
            resolved,
            exported,
            bindings: view.bindings().clone(),
            types: view.types().clone(),
            statics: view.statics().clone(),
            resolutions: view.resolutions().clone(),
            decisions: view.decisions().clone(),
            generics: view.generics().clone(),
            stages: view,
            module,
        })
    }

    /// Return the module id.
    pub fn module(&self) -> ModuleId {
        self.module
    }

    /// Return the post-expansion DIR view.
    pub fn view(&self) -> dir::View<'_> {
        self.stages.tree()
    }

    /// Return the parsed source tree.
    pub fn tree(&self) -> &dir::Tree {
        &self.stages.parsed.tree
    }

    /// Return the checked binding table.
    pub fn bindings(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the checked type table.
    pub fn types(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return the checked static table.
    pub fn statics(&self) -> &dir::StaticTable<'static> {
        &self.statics
    }

    /// Return the checked resolution table.
    pub fn resolutions(&self) -> &dir::ResolutionTable<'static> {
        &self.resolutions
    }

    /// Return the decision table.
    pub fn decisions(&self) -> &dir::DecisionTable<'static> {
        &self.decisions
    }

    /// Return the checked generic declarations and derived variances.
    pub fn generics(&self) -> &dir::GenericTable<'static> {
        &self.generics
    }

    /// Return the resolved source references.
    pub fn resolved(&self) -> &DirResolved {
        &self.resolved
    }

    /// Return the exported module DIR.
    pub fn exported(&self) -> &DirExported {
        &self.exported
    }

    /// Return the checked type of one candidate node.
    pub fn node_type_id(&self, node: dir::LocalNodeIdAny) -> Option<dir::GlobalTypeId> {
        let node = node.into_global(self.module);

        self.types.get_node_type_id(node)
    }
}
