use std::collections::HashMap;

use destack_dir::{
    LocalSymbolId, NodeTree, NodeType, StaticKey, Symbol, SymbolBinding, SymbolKind, SymbolSpace,
    SymbolType,
};
use destack_workspace::Module;

use crate::import::{SymbolDescriptor, can_merge_declarations};
use crate::{Compiler, ImportError};

impl Compiler {
    /// Check for conflicting bindings in module scopes.
    pub(super) fn validate_binding_conflicts(&self, module: &Module) {
        let no_redeclare_locals = self
            .program
            .with_dsconfig_options(module, |opts| opts.compiler.no_redeclared_locals)
            .unwrap_or(!module.language_type.is_destack());

        // cross check all named symbols in all scopes in the module
        let tree = module.dir_base().tree.read();
        let symbols = module.dir_base().symbols.read();

        for scope in symbols.scopes() {
            // group symbols by name and category to avoid O(n^2) scans
            let mut buckets: HashMap<StaticKey, HashMap<SymbolCategory, LocalSymbolId>> =
                HashMap::new();
            for (key, symbol_id) in symbols.active_named_symbols(scope) {
                let symbol = symbols.get_symbol(symbol_id);
                let Some(primary_declaration) = symbol.primary_declaration else {
                    continue;
                };
                let category = SymbolCategory::from(symbol);
                let entry = buckets.entry(key).or_default();
                for other_symbol_id in entry.values() {
                    if *other_symbol_id == symbol_id {
                        continue;
                    }
                    let other_symbol = symbols.get_symbol(*other_symbol_id);

                    // check if symbols conflict based on space and merging rules
                    if !symbol.space.conflicts_with(other_symbol.space)
                        || can_merge_declarations(
                            module.language_type,
                            SymbolDescriptor::from(symbol),
                            SymbolDescriptor::from(other_symbol),
                        )
                    {
                        continue;
                    }

                    // local conflicts are allowed unless configured otherwise, parameters always conflict
                    // (parameter bindings are "local", but conflicts are errors, even with no_redeclare_locals)
                    if symbol.kind == SymbolKind::Local
                        && other_symbol.kind == SymbolKind::Local
                        && !no_redeclare_locals
                        && !self.is_parameter_binding(&tree, symbol)
                        && !self.is_parameter_binding(&tree, other_symbol)
                    {
                        continue;
                    }

                    // error on conflicting bindings
                    let Some(other_primary_declaration) = other_symbol.primary_declaration else {
                        continue;
                    };
                    let error = if symbol.export.is_some() && other_symbol.export.is_some() {
                        ImportError::ConflictingExport {
                            node: primary_declaration.into_anchored(None),
                            other_node: other_primary_declaration.into_anchored(None),
                            module: module.id,
                            name: Some(key),
                        }
                    } else {
                        ImportError::ConflictingBinding {
                            node: primary_declaration.into_anchored(None),
                            other_node: other_primary_declaration.into_anchored(None),
                            scope: symbol.scope.0.into_global(module.id),
                            name: Some(key),
                        }
                    };
                    self.error(error);
                    break;
                }
                entry.entry(category).or_insert(symbol_id);
            }
        }
    }

    /// Return true if the symbol is bound to a parameter node.
    fn is_parameter_binding(&self, tree: &NodeTree, symbol: &Symbol) -> bool {
        // require a primary declaration node
        let Some(primary) = symbol.primary_declaration else {
            return false;
        };

        // walk parents to find parameter bindings
        let mut current = Some(primary.local_id);
        while let Some(current_id) = current {
            // stop once a parameter node is found
            if matches!(current_id.ty, NodeType::Parameter) {
                return true;
            }

            current = tree.get_parent(current_id.id);
        }

        false
    }
}

/// Grouping key for conflict validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SymbolCategory {
    /// Symbol space for conflict grouping.
    space: SymbolSpace,
    /// Symbol type for conflict grouping.
    ty: SymbolType,
    /// Symbol binding for conflict grouping.
    binding: SymbolBinding,
    /// Symbol kind for conflict grouping.
    kind: SymbolKind,
}

impl From<&Symbol> for SymbolCategory {
    /// Create a conflict category from a symbol.
    fn from(symbol: &Symbol) -> Self {
        Self {
            space: symbol.space,
            ty: symbol.ty,
            binding: symbol.binding,
            kind: symbol.kind,
        }
    }
}
