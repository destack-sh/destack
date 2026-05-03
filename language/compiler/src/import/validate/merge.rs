use destack_dir::{
    LocalMergeGroupId, LocalScopeId, LocalScopeMark, LocalSymbolId, StaticKey, SymbolBinding,
    SymbolKind, SymbolSpace, SymbolTable, SymbolType,
};
use destack_workspace::Module;

use crate::Compiler;
use crate::common::dir::{SymbolDescriptor, can_merge_declarations};

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
        let supports_destack_runtime_namespace_merge = module.is_destack()
            && (symbol_type == SymbolType::Function || kind == SymbolKind::Namespace);
        if !module.supports_declaration_merging() && !supports_destack_runtime_namespace_merge {
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
        let candidate_ids: Vec<_> = symbols
            .active_named_symbols(scope_view)
            .filter_map(|(candidate_key, symbol_id)| {
                if candidate_key == key {
                    Some(symbol_id)
                } else {
                    None
                }
            })
            .collect();

        let mut merge_group = None;
        for symbol_id in candidate_ids {
            let symbol = symbols.get_symbol(symbol_id);
            if !can_merge_declarations(
                module.code_language_type(),
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
