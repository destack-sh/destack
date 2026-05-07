use destack_dir::{
    Declaration, LocalNodeIdAny, LocalScopeId, LocalSymbolId, NodeType, SymbolOrigin, SymbolTable,
    Tree,
};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Mark symbols declared within global augmentation scopes.
    pub(super) fn mark_global_augmentation_symbols(
        &self,
        _module: &Module,
        tree: &Tree,
        symbols: &mut SymbolTable,
        global_augmentation_scope: LocalScopeId,
    ) {
        let symbol_ids = symbols
            .symbol_ids()
            .filter(|symbol_id| {
                symbol_is_within_scope(symbols, *symbol_id, global_augmentation_scope)
                    || symbol_declaration_is_within_global(tree, symbols, *symbol_id)
            })
            .collect::<Vec<_>>();

        for symbol_id in symbol_ids {
            let symbol = symbols.get_symbol_mut(symbol_id);
            symbol.origin = SymbolOrigin::GlobalAugmentation;
        }
    }
}

/// Return true when a symbol declaration is nested under `declare global`.
fn symbol_declaration_is_within_global(
    tree: &Tree,
    symbols: &SymbolTable,
    symbol_id: LocalSymbolId,
) -> bool {
    let symbol = symbols.get_symbol(symbol_id);
    let Some(declaration) = symbol.declaration else {
        return false;
    };

    node_is_within_global(tree, declaration.local_id)
}

/// Return true when a node is nested under `declare global`.
fn node_is_within_global(tree: &Tree, node_id: LocalNodeIdAny) -> bool {
    let mut current = Some(node_id);

    while let Some(current_node_id) = current {
        if current_node_id.ty == NodeType::Declaration {
            let declaration_id = current_node_id.into_typed::<Declaration>();
            if matches!(tree.get(declaration_id), Declaration::Global(_)) {
                return true;
            }
        }

        current = tree.get_parent(current_node_id.id);
    }

    false
}

/// Return true when a symbol is scoped inside a given scope.
fn symbol_is_within_scope(
    symbols: &SymbolTable,
    symbol_id: LocalSymbolId,
    scope_id: LocalScopeId,
) -> bool {
    let symbol = symbols.get_symbol(symbol_id);
    let mut current = Some(symbol.scope.0);

    while let Some(current_scope_id) = current {
        if current_scope_id == scope_id {
            return true;
        }

        current = symbols
            .get_scope_by_id(current_scope_id)
            .parent
            .map(|(id, _)| id);
    }

    false
}
