use crate::{
    CodegenJsResult, CodegenJsResultExt, DependencyItem, DependencyKind, DependencyMode,
    Expression, LocalNodeId, ModuleLowerer,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a dependency kind from DIR into JS AST.
    pub fn lower_dependency_kind(&self, kind: dir::DependencyKind) -> DependencyKind {
        match kind {
            dir::DependencyKind::Type => DependencyKind::Type,
            dir::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Lower a dependency mode from DIR into JS AST.
    pub fn lower_dependency_mode(&self, mode: dir::DependencyMode) -> DependencyMode {
        match mode {
            dir::DependencyMode::Item => DependencyMode::Item,
            dir::DependencyMode::Default => DependencyMode::Default,
            dir::DependencyMode::Namespace => DependencyMode::Namespace,
        }
    }

    /// Lower dependency items from DIR into JS AST.
    pub fn lower_dependency_items(
        &mut self,
        kind: dir::DependencyKind,
        item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> CodegenJsResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut lowered_item_ids: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        for item_id in item_ids {
            let item = self.dir_tree.get(*item_id);
            let lowered_item = match item {
                dir::DependencyItem::UnresolvedRemote {
                    mode,
                    source: _,
                    kind: item_kind,
                    name,
                    alias,
                    target: _,
                    target_module: _,
                    symbol: _,
                } => {
                    let mode = self.lower_dependency_mode(*mode);
                    let name = name.map(|name| self.strings.intern_from(&self.ast.strings, name));
                    let alias =
                        alias.map(|alias| self.strings.intern_from(&self.ast.strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.lower_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    }
                }
                dir::DependencyItem::UnresolvedLocal {
                    mode,
                    kind: item_kind,
                    name,
                    alias,
                    symbol: _,
                } => {
                    let mode = self.lower_dependency_mode(*mode);
                    let name = name.map(|name| self.strings.intern_from(&self.ast.strings, name));
                    let alias =
                        alias.map(|alias| self.strings.intern_from(&self.ast.strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.lower_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    }
                }
                dir::DependencyItem::Value { mode, value } => {
                    let mode = self.lower_dependency_mode(*mode);
                    let value_id = self
                        .lower_expression(*value)
                        .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                    DependencyItem {
                        mode,
                        kind: None,
                        name: None,
                        alias: None,
                        value: Some(value_id),
                    }
                }
                dir::DependencyItem::Local {
                    mode,
                    kind: item_kind,
                    name,
                    alias,
                    symbol: _,
                    target_symbol: _,
                } => {
                    let mode = self.lower_dependency_mode(*mode);
                    let name = name.map(|name| self.strings.intern_from(&self.ast.strings, name));
                    let alias =
                        alias.map(|alias| self.strings.intern_from(&self.ast.strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.lower_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    }
                }
                dir::DependencyItem::Remote {
                    mode,
                    kind: item_kind,
                    name,
                    alias,
                    target: _,
                    target_module: _,
                    symbol: _,
                    target_symbol: _,
                } => {
                    let mode = self.lower_dependency_mode(*mode);
                    let name = name.map(|name| self.strings.intern_from(&self.ast.strings, name));
                    let alias =
                        alias.map(|alias| self.strings.intern_from(&self.ast.strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.lower_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    }
                }
            };
            let item_id = self
                .tree
                .insert_from_source(lowered_item, self.module.id, *item_id);
            lowered_item_ids.push(item_id);
        }
        Ok(lowered_item_ids)
    }
}
