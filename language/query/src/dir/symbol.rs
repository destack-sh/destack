use destack_dir as dir;
use std::collections::HashSet;

use destack_core::StringPool;
use destack_source::{ModuleId, Span};

use super::{container_name_for_node, declaration_display_name, matches_symbol_space_filter};
use crate::core::{DirQueryContext, ModuleQueryContext, Name, SymbolEntry, SymbolEntryKind};

/// Build a global symbol id from a module and local symbol id.
pub(crate) fn global_symbol(
    module_id: ModuleId,
    local_id: dir::LocalSymbolId,
) -> dir::GlobalSymbolId {
    dir::GlobalSymbolId {
        module_id,
        local_id,
    }
}

impl DirQueryContext<'_> {
    /// Resolve the local symbol declared by one DIR node.
    pub(crate) fn local_symbol_for_node(
        self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalSymbolId> {
        self.symbol_for_node(node_id)
    }

    /// Resolve the global symbol declared by one DIR node.
    pub(crate) fn global_symbol_for_node(
        self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.local_symbol_for_node(node_id)
            .map(|symbol_id| global_symbol(self.module_id(), symbol_id))
    }

    /// Return the canonical symbol reached by dependency bindings.
    pub(crate) fn canonical_symbol(&self, symbol_id: dir::GlobalSymbolId) -> dir::GlobalSymbolId {
        let mut symbol_id = symbol_id;
        let mut seen = HashSet::new();

        // follow import bindings through recorded dependency resolutions
        while seen.insert(symbol_id) {
            let Some(ctx) = self.module_context(symbol_id.module_id) else {
                return symbol_id;
            };

            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let Some(declaration) = symbol.declaration else {
                return symbol_id;
            };

            if declaration.local_id.ty != dir::NodeType::DependencyItem {
                return symbol_id;
            }

            let Ok(item_id) = declaration.local_id.try_into() else {
                return symbol_id;
            };

            let Some(target_symbol) = ctx.dir().dependency_symbol_target(item_id) else {
                return symbol_id;
            };

            symbol_id = target_symbol;
        }

        symbol_id
    }

    /// Return whether one symbol still refers to one target.
    pub(crate) fn symbol_matches_reference_target(
        &self,
        symbol_id: dir::GlobalSymbolId,
        target_symbol_id: dir::GlobalSymbolId,
    ) -> bool {
        if symbol_id == target_symbol_id {
            return true;
        }

        self.canonical_symbol(symbol_id) == target_symbol_id
    }

    /// Resolve a symbol name string when possible.
    pub(crate) fn symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let ctx = self.module_context(symbol_id.module_id)?;
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol
            .name()
            .map(|name_id| ctx.dir().strings().get(name_id).to_string())
    }
}

impl ModuleQueryContext<'_> {
    /// Build symbol index entries for this module.
    pub(crate) fn build_workspace_symbol_candidates(&self) -> Vec<SymbolEntry> {
        let dir_tree = self.dir().view();
        let mut entries = Vec::new();
        let module_id = self.module_id();

        // collect declaration symbols
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            let name = declaration_display_name(self.dir().strings(), declaration);
            let kind = symbol_index_kind_for_declaration(declaration);
            let container_name =
                container_name_for_node(dir_tree, self.dir().strings(), declaration_id.into());

            let Some(range) = self.symbol_index_range(dir_tree, declaration_id.id) else {
                continue;
            };

            entries.push(SymbolEntry {
                name: Name::String(name),
                kind,
                module_id,
                file_id: self.file_id(),
                range,
                symbol_id: self
                    .dir()
                    .symbol_for_node(declaration_id.into())
                    .map(|symbol_id| symbol_id.into_global(module_id)),
                container_name,
            });
        }

        entries
    }

    /// Return the canonical symbol reached by dependency bindings.
    pub(crate) fn canonical_symbol(&self, symbol_id: dir::GlobalSymbolId) -> dir::GlobalSymbolId {
        self.dir().canonical_symbol(symbol_id)
    }

    /// Resolve a symbol name string when possible.
    pub(crate) fn symbol_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        self.dir().symbol_name(symbol_id)
    }

    /// Return the definition span of a symbol.
    pub(crate) fn symbol_definition_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        self.symbol_span_with(symbol_id, |ctx, tree, node| {
            ctx.get_node_tree_main_span(tree, node)
        })
    }

    /// Return the local definition span of a symbol without canonical expansion.
    pub(crate) fn symbol_local_definition_span(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<Span> {
        let declaration = {
            let symbols = self.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        if let Some(declaration) = declaration {
            return Some(
                self.dir()
                    .get_node_tree_main_span(self.dir().view(), declaration.local_id),
            );
        }

        None
    }

    /// Return the definition span when a symbol names a type.
    pub(crate) fn type_definition_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        let (is_type_symbol, declaration) = {
            let symbols = self.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            (
                matches_symbol_space_filter(symbol.kind, Some(dir::SymbolSpace::Type)),
                symbol.declaration,
            )
        };

        if !is_type_symbol {
            return None;
        }

        if declaration
            .is_some_and(|declaration| declaration.local_id.ty == dir::NodeType::DependencyItem)
        {
            let target_symbol = self.dependency_item_target_symbol(symbol_id)?;
            let target_ctx = self.module_context(target_symbol.module_id)?;

            return target_ctx.symbol_definition_span(target_symbol);
        }

        self.symbol_definition_span(symbol_id)
    }

    /// Return the full declaration span of a symbol.
    pub(crate) fn symbol_declaration_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        self.symbol_span_with(symbol_id, |ctx, tree, node| {
            ctx.get_node_tree_span(tree, node)
        })
    }
}

/// Resolve a member name from a key.
pub(crate) fn member_key_name(strings: &StringPool, key: &dir::Key) -> Option<String> {
    match key {
        dir::Key::Name(name) => Some(strings.get(name.string()).to_string()),
        dir::Key::Private(name) => {
            let name = strings.get(*name).to_string();
            Some(format!("#{name}"))
        }
        dir::Key::Expression(_) => None,
    }
}

/// Check whether a member field is the synthetic `function` keyword placeholder.
pub(crate) fn is_synthetic_function_keyword_field(
    member: &dir::Member,
    name: &str,
    range: Span,
) -> bool {
    if !matches!(member, dir::Member::Field { .. }) || name != "function" {
        return false;
    }

    let full_len = range.end.saturating_sub(range.start);
    full_len == 8
}

impl ModuleQueryContext<'_> {
    /// Resolve one symbol index range without failing the whole query on bad source ids.
    fn symbol_index_range(&self, dir_tree: dir::View<'_>, node_id: u32) -> Option<Span> {
        let ctx = self;
        let node_id = dir::LocalNodeIdAny::new(node_id, dir_tree.get_node_type(node_id));
        ctx.dir().try_span_for_dir_node(dir_tree, node_id)
    }

    /// Resolve a symbol span using the provided declaration span strategy.
    fn symbol_span_with(
        &self,
        symbol_id: dir::GlobalSymbolId,
        span_for_declaration: impl Fn(DirQueryContext<'_>, dir::View<'_>, dir::LocalNodeIdAny) -> Span
        + Copy,
    ) -> Option<Span> {
        let ctx = self;
        let ctx = ctx.module_context(symbol_id.module_id)?;

        let (canonical_id, declaration) = {
            let symbols = ctx.dir().symbols();
            let canonical_id = ctx.canonical_symbol(symbol_id);

            if canonical_id.module_id != symbol_id.module_id {
                (canonical_id, None)
            } else {
                let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
                (canonical_id, canonical_symbol.declaration)
            }
        };

        if canonical_id.module_id != symbol_id.module_id {
            let canonical_ctx = ctx.module_context(canonical_id.module_id)?;

            return canonical_ctx.symbol_span_with(canonical_id, span_for_declaration);
        }

        if let Some(declaration) = declaration {
            return Some(span_for_declaration(
                ctx.dir(),
                ctx.dir().view(),
                declaration.local_id,
            ));
        }

        None
    }

    /// Return the imported target symbol for a dependency-item binding.
    fn dependency_item_target_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalSymbolId> {
        let dir = self.dir();

        let declaration = {
            let symbols = dir.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id = declaration.local_id.try_into().ok()?;
        dir.dependency_symbol_target(item_id)
    }
}

/// Map one declaration to the symbol index kind.
fn symbol_index_kind_for_declaration(declaration: &dir::Declaration) -> SymbolEntryKind {
    match declaration {
        dir::Declaration::Global { .. } => SymbolEntryKind::Namespace,
        dir::Declaration::Module { .. } => SymbolEntryKind::Namespace,
        dir::Declaration::Function { .. } => SymbolEntryKind::Function,
        dir::Declaration::Struct { .. } => SymbolEntryKind::Struct,
        dir::Declaration::Class { .. } => SymbolEntryKind::Class,
        dir::Declaration::Interface { .. } => SymbolEntryKind::Interface,
        dir::Declaration::Enum { .. } => SymbolEntryKind::Enum,
        dir::Declaration::Type { .. } => SymbolEntryKind::TypeParameter,
        dir::Declaration::Extension { .. } => SymbolEntryKind::Class,
    }
}
