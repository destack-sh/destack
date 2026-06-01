use destack_core::StringPool;
use destack_dir as dir;

use crate::core::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Resolve the container name for a symbol when it belongs to a type scope.
    pub(crate) fn container_name_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<String> {
        // read the module query context
        let ctx = self.module_context(symbol_id.module_id)?;

        // read the symbol scope owner
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let scope = symbols.get_scope_by_id(symbol.scope.id);
        let owner_id = scope.owner?;
        let owner = symbols.get_symbol(owner_id);
        let name_id = owner.name()?;

        // return the container name
        Some(ctx.dir().strings().get(name_id).to_string())
    }
}

/// Resolve the container name by walking the DIR parent chain.
pub(crate) fn container_name_for_node(
    dir_tree: dir::View<'_>,
    strings: &StringPool,
    node_id: dir::LocalNodeIdAny,
) -> Option<String> {
    // start at the current node id
    let mut current_id = node_id;

    // walk up parent chain
    while let Some(parent) = dir_tree.get_parent_any(current_id) {
        if parent.ty == dir::NodeType::Declaration {
            // resolve the parent declaration name
            let Ok(decl_id) = dir::LocalNodeId::<dir::Declaration>::try_from(parent) else {
                current_id = parent;
                continue;
            };
            let parent_decl = dir_tree.get(decl_id);
            if let Some(name) = parent_decl.name() {
                return Some(strings.get(name.string()).to_string());
            }
        }

        current_id = parent;
    }

    None
}
