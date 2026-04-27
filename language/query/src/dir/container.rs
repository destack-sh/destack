use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::{GlobalSymbolId, NodeType};
use destack_workspace::{Repository, Revision};

use crate::core::query_context_for_module_id;
use crate::dir::declaration_name;

/// Resolve the container name for a symbol when it belongs to a type scope.
pub(crate) fn container_name_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<String> {
    // resolve the module query context
    let ctx = query_context_for_module_id(repository, revision, symbol_id.module_id)?;

    // resolve the symbol scope owner
    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let scope = symbols.get_scope_by_id(symbol.scope.0);
    let owner_id = scope.owner_id?;
    let owner = symbols.get_symbol(owner_id);
    let name_id = owner.name()?;

    // return the container name
    Some(repository.strings.get(name_id).to_string())
}

/// Resolve the container name by walking the DIR parent chain.
pub(crate) fn container_name_for_node(
    dir_tree: &dir::Tree,
    strings: &StringPool,
    node_id: u32,
) -> Option<String> {
    // start at the current node id
    let mut current_id = node_id;

    // walk up parent chain
    while let Some(parent) = dir_tree.get_parent(current_id) {
        if parent.ty == NodeType::Declaration {
            // resolve the parent declaration name
            let Ok(decl_id) = dir::LocalNodeId::<dir::Declaration>::try_from(parent) else {
                current_id = parent.id;
                continue;
            };
            let parent_decl = dir_tree.get(decl_id);
            if let Some(name) = declaration_name(parent_decl) {
                return Some(strings.get(name.string()).to_string());
            }
        }

        current_id = parent.id;
    }

    None
}
