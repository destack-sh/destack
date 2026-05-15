use destack_dir as dir;

use super::super::state::BindState;

use crate::Compiler;

impl Compiler {
    /// Bind one dependency item.
    pub(in crate::bind) fn bind_dependency_item(
        &self,
        state: &mut BindState<'_>,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        // bind item scope
        state.bind_node(id.into_any());

        // visit dependency payload
        dir::walk_dependency_item(state, tree, id, dependency_item);
    }

    /// Bind one imported dependency item.
    pub(in crate::bind) fn bind_import_item(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::DependencyItem>,
        import_space: dir::DependencySpace,
        dependency_item: &dir::DependencyItem,
    ) {
        // ignore non binding items
        let Some(key) = dependency_item.symbol_key() else {
            return;
        };
        let Some(form) = dependency_item.symbol_form(import_space) else {
            return;
        };

        // declare imported symbol
        let symbol_id = state.insert_symbol(dir::SymbolRole::Local, form, Some(key), None);

        state.declare_symbol(symbol_id, node_id);
    }
}
