use std::slice;
use std::sync::Arc;

use destack_artifact::{DirBound, DirCheckedModule, DirExpanded, DirParsed, DirResolved};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{ModuleId, SourceIndex, Span};

/// Semantic DIR read state shared by module index builders.
#[derive(Debug)]
pub(in crate::index) struct ModuleIndexContext<'a> {
    /// The parsed module DIR.
    parsed: Arc<DirParsed>,
    /// The expanded module DIR.
    expanded: Arc<DirExpanded>,
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
    /// The resolved source references.
    resolved: Arc<DirResolved>,
    /// The shared string pool.
    strings: &'a StringPool,
    /// The indexed module id.
    module_id: ModuleId,
}

impl<'a> ModuleIndexContext<'a> {
    /// Build semantic index read state from exact DIR artifacts.
    pub(in crate::index) fn new(
        strings: &'a StringPool,
        parsed: Arc<DirParsed>,
        bound: &DirBound,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        checked: &DirCheckedModule,
    ) -> Self {
        let bindings = checked.binding_table(bound, &expanded);
        let types = checked.type_table(bound, &expanded);

        Self {
            module_id: checked.module,
            parsed,
            expanded,
            bindings,
            types,
            decorators: checked.decorator_table(),
            definitions: checked.definition_table(),
            resolutions: checked.resolution_table(),
            resolved,
            strings,
        }
    }

    /// Return the indexed module id.
    pub(super) fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Return the visible expanded tree.
    pub(super) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, slice::from_ref(&self.expanded.patch))
    }

    /// Return the parsed source index.
    pub(super) fn source_index(&self) -> &SourceIndex {
        &self.parsed.tree.source_index
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

    /// Return the resolved source references.
    pub(super) fn resolved(&self) -> &DirResolved {
        &self.resolved
    }

    /// Return the shared string pool.
    pub(super) fn strings(&self) -> &StringPool {
        self.strings
    }

    /// Return the symbol declared by one local node.
    pub(super) fn node_symbol(&self, node_id: dir::LocalNodeIdAny) -> Option<dir::LocalSymbolId> {
        let declaration = node_id.into_global(self.module_id);

        self.bindings().declaration_symbol(declaration)
    }

    /// Return the authored selection span for one node.
    pub(super) fn node_selection_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<Span> {
        let source_id = view.get_source_any(node_id);

        self.source_index().get_main(source_id)
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
