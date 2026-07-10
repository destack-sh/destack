use std::collections::HashSet;

use destack_core::StringPool;
use destack_dir as dir;
use destack_source::Span;

use crate::{ModuleQueryContext, SymbolUse};

/// Name methods for DIR member keys.
pub(crate) trait MemberKeyName {
    /// Resolve a member name from the key.
    fn member_name(&self, strings: &StringPool) -> Option<String>;
}

impl MemberKeyName for dir::Key {
    fn member_name(&self, strings: &StringPool) -> Option<String> {
        match self {
            dir::Key::Name(name) => Some(strings.get(name.string()).to_string()),
            dir::Key::Expression(_) => None,
        }
    }
}

/// Synthetic member classification methods.
pub(crate) trait SyntheticMember {
    /// Return whether this member is the synthetic `function` keyword placeholder.
    fn is_synthetic_function_keyword_field(&self, name: &str, range: Span) -> bool;
}

impl SyntheticMember for dir::Member {
    fn is_synthetic_function_keyword_field(&self, name: &str, range: Span) -> bool {
        if !matches!(self, dir::Member::Field { .. }) || name != "function" {
            return false;
        }

        let full_len = range.end.saturating_sub(range.start);
        full_len == 8
    }
}

/// Resolve a display name for a declaration.
pub(crate) fn declaration_display_name(
    strings: &StringPool,
    declaration: &dir::Declaration,
) -> Option<String> {
    // default block declarations to keyword labels
    if matches!(declaration, dir::Declaration::Global(_)) {
        return Some("global".to_string());
    }
    if matches!(declaration, dir::Declaration::Module(_)) {
        return Some("module".to_string());
    }

    // prefer the explicit declaration name
    declaration
        .name()
        .map(|name| strings.get(name.string()).to_string())
}

impl ModuleQueryContext<'_> {
    /// Resolve the local symbol declared by one DIR node.
    pub(crate) fn local_node_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalSymbolId> {
        self.node_symbol(node_id)
    }

    /// Resolve the global symbol declared by one DIR node.
    pub(crate) fn global_node_symbol(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        self.local_node_symbol(node_id)
            .map(|symbol_id| symbol_id.into_global(self.module_id()))
    }

    /// Return the canonical symbol reached by dependency bindings.
    pub(crate) fn canonical_symbol(&self, symbol_id: dir::GlobalSymbolId) -> dir::GlobalSymbolId {
        let mut symbol_id = symbol_id;
        let mut seen = HashSet::new();

        // follow import bindings through recorded dependency resolutions
        while seen.insert(symbol_id) {
            let module = self.module_context(symbol_id.module_id);

            let symbols = module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            let Some(declaration) = symbol.declaration else {
                return symbol_id;
            };

            if declaration.local_id.ty != dir::NodeType::DependencyItem {
                return symbol_id;
            }

            let item_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                panic!(
                    "dependency symbol has non dependency item declaration: {:?}",
                    declaration.local_id
                )
            });

            let target_symbol = module.dependency_symbol_target(item_id).unwrap_or_else(|| {
                panic!("missing dependency target symbol for declaration {item_id:?}")
            });

            symbol_id = target_symbol;
        }

        panic!("cyclic canonical symbol chain at {symbol_id:?}");
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
        let module = self.module_context(symbol_id.module_id);
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol
            .name()
            .map(|name_id| module.strings().get(name_id).to_string())
    }

    /// Resolve the container name for a symbol when it belongs to a type scope.
    pub(crate) fn symbol_container_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let symbol_module = self.module_context(symbol_id.module_id);

        // resolve the owner symbol of the symbol scope
        let symbols = symbol_module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let scope = symbols.get_scope_by_id(symbol.scope.id);
        let owner_id = scope.owner?;
        let owner = symbols.get_symbol(owner_id);
        let name_id = owner.name()?;

        Some(symbol_module.strings().get(name_id).to_string())
    }

    /// Resolve the container name by walking the DIR parent chain.
    pub(crate) fn node_container_name(&self, node_id: dir::LocalNodeIdAny) -> Option<String> {
        let view = self.view();
        let mut current_id = node_id;

        // walk up parent chain
        while let Some(parent) = view.get_parent_any(current_id) {
            if parent.ty == dir::NodeType::Declaration {
                let declaration_id =
                    parent
                        .try_into_typed::<dir::Declaration>()
                        .unwrap_or_else(|_| {
                            panic!("container parent is not a declaration: {parent:?}")
                        });
                let declaration = view.get(declaration_id);
                if let Some(name) = declaration.name() {
                    return Some(self.strings().get(name.string()).to_string());
                }
            }

            current_id = parent;
        }

        None
    }

    /// Return one symbol's kind.
    pub(crate) fn symbol_kind(&self, symbol_id: dir::GlobalSymbolId) -> dir::SymbolKind {
        let module = self.module_context(symbol_id.module_id);
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        symbol.kind
    }
}

impl ModuleQueryContext<'_> {
    /// Return the definition span of a symbol.
    pub(crate) fn symbol_definition_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        self.symbol_span_with(symbol_id, |module, tree, node| {
            module.get_main_span(tree, node)
        })
    }

    /// Return the local definition span of a symbol without canonical expansion.
    pub(crate) fn symbol_local_definition_span(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<Span> {
        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        if let Some(declaration) = declaration {
            return Some(self.get_main_span(self.view(), declaration.local_id));
        }

        None
    }

    /// Return the definition span when a symbol names a type.
    pub(crate) fn type_definition_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        let (is_type_symbol, declaration) = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            (
                SymbolUse::Type.accepts_symbol_kind(symbol.kind),
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
            let target_module = self.module_context(target_symbol.module_id);

            return target_module.symbol_definition_span(target_symbol);
        }

        self.symbol_definition_span(symbol_id)
    }

    /// Return the full declaration span of a symbol.
    pub(crate) fn symbol_declaration_span(&self, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
        self.symbol_span_with(symbol_id, |module, tree, node| module.get_span(tree, node))
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve a symbol span using the provided declaration span strategy.
    fn symbol_span_with(
        &self,
        symbol_id: dir::GlobalSymbolId,
        span_for_declaration: impl Fn(
            &ModuleQueryContext<'_>,
            dir::View<'_>,
            dir::LocalNodeIdAny,
        ) -> Span
        + Copy,
    ) -> Option<Span> {
        let module = self.module_context(symbol_id.module_id);

        let (canonical_id, declaration) = {
            let symbols = module.symbols();
            let canonical_id = module.canonical_symbol(symbol_id);

            if canonical_id.module_id != symbol_id.module_id {
                (canonical_id, None)
            } else {
                let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
                (canonical_id, canonical_symbol.declaration)
            }
        };

        if canonical_id.module_id != symbol_id.module_id {
            let canonical_module = module.module_context(canonical_id.module_id);

            return canonical_module.symbol_span_with(canonical_id, span_for_declaration);
        }

        if let Some(declaration) = declaration {
            return Some(span_for_declaration(
                &module,
                module.view(),
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
        let declaration = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };
        if declaration.local_id.ty != dir::NodeType::DependencyItem {
            return None;
        }

        let item_id = declaration.local_id.try_into().unwrap_or_else(|_| {
            panic!(
                "dependency symbol has non dependency item declaration: {:?}",
                declaration.local_id
            )
        });
        self.dependency_symbol_target(item_id)
    }
}
