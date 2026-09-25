use tspp_dir as dir;

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
        dependency_item: &dir::DependencyItem,
    ) {
        // ignore non binding items
        let Some(key) = dependency_item.symbol_key() else {
            return;
        };
        let Some(kind) = dependency_item.symbol_kind() else {
            return;
        };

        // declare imported symbol
        let symbol_id = state.insert_symbol(
            dir::SymbolRole::Local,
            kind,
            Some(key),
            None,
            dir::SymbolVisibility::Scope,
        );

        state.declare_symbol(symbol_id, node_id);
    }

    /// Bind one exported dependency item.
    pub(in crate::bind) fn bind_export_item(
        &self,
        state: &mut BindState<'_>,
        node_id: dir::LocalNodeId<dir::DependencyItem>,
        dependency_item: &dir::DependencyItem,
    ) {
        let symbol_id = match dependency_item {
            // explicit aliases declare public module members
            dir::DependencyItem::Binding {
                alias: Some(alias), ..
            } => state.insert_symbol(
                dir::SymbolRole::Item,
                dir::SymbolKind::ExportAlias,
                Some(dir::StaticKey::Name(*alias)),
                None,
                dir::SymbolVisibility::Member,
            ),

            // default value expressions declare anonymous local values
            dir::DependencyItem::Binding {
                binding: dir::DependencyBinding::Default,
                value: Some(_),
                ..
            } => state.insert_symbol(
                dir::SymbolRole::Local,
                dir::SymbolKind::Variable,
                None,
                Some(dir::ExportKind::Default),
                dir::SymbolVisibility::Scope,
            ),

            // other export items select declarations without introducing one
            dir::DependencyItem::Binding { .. } | dir::DependencyItem::Error => return,
        };

        state.declare_symbol(symbol_id, node_id);
    }
}
