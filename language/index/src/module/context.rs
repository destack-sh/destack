use std::sync::Arc;

use destack_artifact::{DirResolved, DirView};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{ModuleId, SourceIndex};

/// DIR artifacts shared by module index builders.
#[derive(Debug)]
pub(crate) struct ModuleIndexContext<'a> {
    /// The stacked module DIR stages.
    stages: DirView,
    /// The visible binding table.
    bindings: dir::BindingTable<'static>,
    /// The checked type table.
    types: dir::TypeTable<'static>,
    /// The checked decorator table.
    decorators: dir::DecoratorTable<'static>,
    /// The checked definition table.
    definitions: dir::DefinitionTable<'static>,
    /// The checked resolution table.
    resolutions: dir::ResolutionTable<'static>,
    /// The checked member table.
    members: dir::MemberTable<'static>,
    /// The checked decision table.
    decisions: dir::DecisionTable<'static>,
    /// The resolved source references.
    resolved: Arc<DirResolved>,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The indexed module id.
    module_id: ModuleId,
}

impl<'a> ModuleIndexContext<'a> {
    /// Build module index read state from exact DIR artifacts.
    pub(crate) fn new(strings: &'a StringPool, module_id: ModuleId, view: DirView) -> Self {
        Self {
            module_id,
            bindings: view.bindings().clone(),
            types: view.types().clone(),
            decorators: view.decorators().clone(),
            definitions: view.definitions().clone(),
            resolutions: view.resolutions().clone(),
            members: view.members().clone(),
            decisions: view.decisions().clone(),
            resolved: Arc::clone(view.resolved.as_ref().unwrap_or_else(|| unreachable!())),
            strings,
            stages: view,
        }
    }

    /// Return the indexed module id.
    pub(super) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the visible expanded tree.
    pub(super) fn view(&self) -> dir::View<'_> {
        self.stages.tree()
    }

    /// Return the parsed source index.
    pub(super) fn source_index(&self) -> &SourceIndex {
        &self.stages.parsed.tree.source_index
    }

    /// Return the visible binding table.
    pub(super) fn bindings(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the checked type table.
    pub(super) fn types(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return the checked decorator table.
    pub(super) fn decorators(&self) -> &dir::DecoratorTable<'static> {
        &self.decorators
    }

    /// Return the checked definition table.
    pub(super) fn definitions(&self) -> &dir::DefinitionTable<'static> {
        &self.definitions
    }

    /// Return the checked resolution table.
    pub(super) fn resolutions(&self) -> &dir::ResolutionTable<'static> {
        &self.resolutions
    }

    /// Return the checked member table.
    pub(super) fn members(&self) -> &dir::MemberTable<'static> {
        &self.members
    }

    /// Return the module's decision table.
    pub(super) fn decisions(&self) -> &dir::DecisionTable<'static> {
        &self.decisions
    }

    /// Return the resolved source references.
    pub(super) fn resolved(&self) -> &DirResolved {
        &self.resolved
    }

    /// Return whether one symbol is an explicit local import alias.
    pub(super) fn is_local_import_alias(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        if symbol_id.module_id != self.module_id {
            return false;
        }

        // require a dependency declaration
        let symbol = self.bindings().get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return false;
        };
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return false;
        }

        // inspect the authored local binding
        let item_id = dir::LocalNodeId::<dir::DependencyItem>::new(declaration.local_id.id);
        let item = self.view().get(item_id);

        item.local_import_alias_name().is_some()
    }

    /// Resolve the source name of one decorator application.
    pub(super) fn decorator_name(
        &self,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> Option<String> {
        let decorator = self.view().get(decorator);
        let mut expression_id = decorator.expression;

        // unwrap calls and return the final path segment
        loop {
            let name = match self.view().get(expression_id) {
                dir::Expression::Call { left, .. } => {
                    expression_id = *left;

                    continue;
                }
                dir::Expression::Member { name, .. } => *name,
                dir::Expression::Identifier { name } => Some(*name),
                _ => None,
            };

            return name.map(|name| self.strings.get(name).to_string());
        }
    }
}
