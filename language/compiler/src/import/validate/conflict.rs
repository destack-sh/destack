use std::collections::HashMap;

use destack_dir::{
    Declaration, DependencyItem, DependencyKind, EnumKind, GlobalNodeIdAny, LocalSymbolId, NodeTree,
    NodeType, StaticKey, Symbol, SymbolBinding, SymbolKind, SymbolSpace, SymbolType,
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

                if let Some((node, other_node)) =
                    self.enum_kind_mismatch_nodes(&tree, symbol)
                {
                    let error = ImportError::ConflictingBinding {
                        node: node.into_anchored(None),
                        other_node: other_node.into_anchored(None),
                        scope: symbol.scope.0.into_global(module.id),
                        name: Some(key),
                    };
                    self.error(error);
                }

                let category = SymbolCategory::from(symbol);
                let entry = buckets.entry(key).or_default();
                for other_symbol_id in entry.values() {
                    if *other_symbol_id == symbol_id {
                        continue;
                    }
                    let other_symbol = symbols.get_symbol(*other_symbol_id);

                    // check if symbols conflict based on space and merging rules
                    let import_kind_conflict =
                        self.is_type_value_import_conflict(&tree, symbol, other_symbol);
                    if !symbol.space.conflicts_with(other_symbol.space) && !import_kind_conflict {
                        continue;
                    }

                    let enum_kind_mismatch =
                        self.is_const_enum_mismatch(&tree, symbol, other_symbol);
                    let can_merge = can_merge_declarations(
                        module.language_type,
                        SymbolDescriptor::from(symbol),
                        SymbolDescriptor::from(other_symbol),
                    ) && !enum_kind_mismatch;
                    if can_merge {
                        continue;
                    }

                    // local conflicts are allowed unless configured otherwise, parameters always conflict
                    // (parameter bindings are "local", but conflicts are errors, even with no_redeclare_locals)
                    let is_strict_local_conflict =
                        matches!(symbol.ty, SymbolType::TypeAlias | SymbolType::Newtype)
                            || matches!(
                                other_symbol.ty,
                                SymbolType::TypeAlias | SymbolType::Newtype
                            );
                    if symbol.kind == SymbolKind::Local
                        && other_symbol.kind == SymbolKind::Local
                        && !no_redeclare_locals
                        && !self.is_parameter_binding(&tree, symbol)
                        && !self.is_parameter_binding(&tree, other_symbol)
                        && !is_strict_local_conflict
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

    fn is_const_enum_mismatch(&self, tree: &NodeTree, left: &Symbol, right: &Symbol) -> bool {
        let Some(left_kind) = self.enum_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.enum_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }

    fn enum_kind_for_symbol(&self, tree: &NodeTree, symbol: &Symbol) -> Option<EnumKind> {
        let primary = symbol.primary_declaration?;
        if primary.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = primary.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    fn enum_kind_for_declaration(
        &self,
        tree: &NodeTree,
        node: GlobalNodeIdAny,
    ) -> Option<EnumKind> {
        if node.local_id.ty != NodeType::Declaration {
            return None;
        }
        let declaration_id = node.local_id.into_typed::<Declaration>();
        match tree.get(declaration_id) {
            Declaration::Enum { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    fn enum_kind_mismatch_nodes(
        &self,
        tree: &NodeTree,
        symbol: &Symbol,
    ) -> Option<(GlobalNodeIdAny, GlobalNodeIdAny)> {
        if symbol.ty != SymbolType::Enum {
            return None;
        }
        let primary = symbol.primary_declaration?;
        let primary_kind = self.enum_kind_for_declaration(tree, primary)?;
        let Some(secondaries) = symbol.secondary_declarations.as_deref() else {
            return None;
        };
        for secondary in secondaries {
            let Some(kind) = self.enum_kind_for_declaration(tree, *secondary) else {
                continue;
            };
            if kind != primary_kind {
                return Some((primary, *secondary));
            }
        }
        None
    }

    fn is_type_value_import_conflict(
        &self,
        tree: &NodeTree,
        left: &Symbol,
        right: &Symbol,
    ) -> bool {
        let Some(left_kind) = self.dependency_kind_for_symbol(tree, left) else {
            return false;
        };
        let Some(right_kind) = self.dependency_kind_for_symbol(tree, right) else {
            return false;
        };
        left_kind != right_kind
    }

    fn dependency_kind_for_symbol(
        &self,
        tree: &NodeTree,
        symbol: &Symbol,
    ) -> Option<DependencyKind> {
        let primary = symbol.primary_declaration?;
        if primary.local_id.ty != NodeType::DependencyItem {
            return None;
        }
        let item_id = primary.local_id.into_typed::<DependencyItem>();
        match tree.get(item_id) {
            DependencyItem::UnresolvedRemote { kind, .. }
            | DependencyItem::UnresolvedLocal { kind, .. }
            | DependencyItem::Local { kind, .. } => Some(*kind),
            _ => None,
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
