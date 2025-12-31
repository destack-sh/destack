use destack_dir::{
    LocalMergeGroupId, LocalScopeId, LocalScopeMark, LocalSymbolId, StaticKey, Symbol,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolTable, SymbolType,
};
use destack_source::LanguageType;
use destack_workspace::Module;

use crate::Compiler;

/// Summary of a declaration for merge decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SymbolDescriptor {
    /// The declared symbol type.
    pub(crate) symbol_type: SymbolType,
    /// How the symbol was introduced.
    pub(crate) binding: SymbolBinding,
    /// The scope kind for the symbol.
    pub(crate) kind: SymbolKind,
}

impl From<&Symbol> for SymbolDescriptor {
    /// Create a symbol descriptor from a symbol.
    fn from(symbol: &Symbol) -> Self {
        Self {
            symbol_type: symbol.ty,
            binding: symbol.binding,
            kind: symbol.kind,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Select an existing symbol to merge or link with, if allowed.
    pub(crate) fn select_merge_candidate(
        &self,
        module: &Module,
        scope: (LocalScopeId, LocalScopeMark),
        key: StaticKey,
        kind: SymbolKind,
        symbol_type: SymbolType,
        binding: SymbolBinding,
        space: SymbolSpace,
        symbols: &mut SymbolTable,
    ) -> (Option<LocalSymbolId>, Option<LocalMergeGroupId>) {
        // skip merge handling when declaration merging is disabled
        if !module.language_type.supports_declaration_merging() {
            return (None, None);
        }

        // describe the incoming declaration for merge checks
        let incoming = SymbolDescriptor {
            symbol_type,
            binding,
            kind,
        };

        // collect candidate symbols from the scope
        let scope_view = symbols.get_scope_by_id(scope.0);
        let candidate_ids: Vec<_> = scope_view
            .named_symbols
            .iter()
            .filter_map(|(candidate_key, symbol_id)| {
                if *candidate_key == key {
                    Some(*symbol_id)
                } else {
                    None
                }
            })
            .collect();

        let mut merge_group = None;
        for symbol_id in candidate_ids {
            let symbol = symbols.get_symbol(symbol_id);
            if !can_merge_declarations(
                module.language_type,
                SymbolDescriptor::from(symbol),
                incoming,
            ) {
                continue;
            }

            if symbol.ty == symbol_type && symbol.space == space {
                return (Some(symbol_id), merge_group);
            }

            if merge_group.is_none() {
                merge_group = Some(
                    symbol
                        .merge_group
                        .unwrap_or_else(|| symbols.create_merge_group(vec![symbol_id])),
                );
            }
        }

        (None, merge_group)
    }
}

/// Check whether two declarations can merge.
pub(crate) fn can_merge_declarations(
    language_type: LanguageType,
    left: SymbolDescriptor,
    right: SymbolDescriptor,
) -> bool {
    // bail out when declaration merging is not supported
    if !language_type.supports_declaration_merging() {
        return false;
    }

    // reject type aliases in merge candidates
    if left.symbol_type == SymbolType::TypeAlias || right.symbol_type == SymbolType::TypeAlias {
        return false;
    }

    // allow namespaces to merge with namespace or value declarations
    if left.kind == SymbolKind::Namespace || right.kind == SymbolKind::Namespace {
        let other_type = if left.kind == SymbolKind::Namespace {
            right.symbol_type
        } else {
            left.symbol_type
        };
        return matches!(
            other_type,
            SymbolType::Class | SymbolType::Enum | SymbolType::Function | SymbolType::Void
        );
    }

    // allow ambient declarations with matching symbol types
    if left.binding == SymbolBinding::Ambient
        && right.binding == SymbolBinding::Ambient
        && left.symbol_type == right.symbol_type
    {
        return true;
    }

    // allow enum redeclarations
    if left.symbol_type == SymbolType::Enum && right.symbol_type == SymbolType::Enum {
        return true;
    }

    // allow interface redeclarations
    if left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Interface {
        return true;
    }

    // allow interface merges with class or function declarations
    if (left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Class)
        || (left.symbol_type == SymbolType::Class && right.symbol_type == SymbolType::Interface)
        || (left.symbol_type == SymbolType::Interface && right.symbol_type == SymbolType::Function)
        || (left.symbol_type == SymbolType::Function && right.symbol_type == SymbolType::Interface)
    {
        return true;
    }

    false
}
