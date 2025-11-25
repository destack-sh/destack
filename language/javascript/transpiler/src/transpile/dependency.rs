use crate::{TranspileResult, TranspileResultExt, Transpiler, TranspilerUnit};
use dyst_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};
use dyst_javascript_ast::{
    DependencyItem, DependencyKind, DependencyMode, Expression, LocalNodeId,
};

#[allow(clippy::too_many_arguments)]
impl Transpiler {
    /// Transpile a dependency kind from DIR into JS AST.
    pub fn transpile_dependency_kind(&self, kind: dir::DependencyKind) -> DependencyKind {
        match kind {
            dir::DependencyKind::Type => DependencyKind::Type,
            dir::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Transpile a dependency mode from DIR into JS AST.
    pub fn transpile_dependency_mode(&self, mode: dir::DependencyMode) -> DependencyMode {
        match mode {
            dir::DependencyMode::Item => DependencyMode::Item,
            dir::DependencyMode::Default => DependencyMode::Default,
            dir::DependencyMode::Namespace => DependencyMode::Namespace,
        }
    }

    /// Transpile dependency items from DIR into JS AST.
    pub fn transpile_dependency_items(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        kind: dir::DependencyKind,
        item_ids: &[dir::LocalNodeId<dir::DependencyItem>],
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut transpiled_item_ids: Vec<LocalNodeId<DependencyItem>> = Vec::new();
        for item_id in item_ids {
            let item = tree.get(*item_id);
            let transpiled_item = match item {
                dir::DependencyItem::UnresolvedRemote {
                    mode,
                    source: _,
                    kind: item_kind,
                    name,
                    alias,
                    target: _,
                    module: _,
                    symbol: _,
                } => {
                    let mode = self.transpile_dependency_mode(*mode);
                    let name = name.map(|name| unit.strings.intern_from(&module.ast_strings, name));
                    let alias =
                        alias.map(|alias| unit.strings.intern_from(&module.ast_strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.transpile_dependency_kind(*item_kind))
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
                } => {
                    let mode = self.transpile_dependency_mode(*mode);
                    let name = unit.strings.intern_from(&module.ast_strings, *name);
                    let alias =
                        alias.map(|alias| unit.strings.intern_from(&module.ast_strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.transpile_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name: Some(name),
                        alias,
                        value: None,
                    }
                }
                dir::DependencyItem::Value { value } => {
                    let value_id = self
                        .transpile_expression(module, tree, symbols, types, *value, unit)
                        .expect_node::<Expression>(value.into_global_any(module.id), unit)?;
                    DependencyItem {
                        mode: DependencyMode::Namespace,
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
                    target_symbol: _,
                } => {
                    let mode = self.transpile_dependency_mode(*mode);
                    let name = unit.strings.intern_from(&module.ast_strings, *name);
                    let alias =
                        alias.map(|alias| unit.strings.intern_from(&module.ast_strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.transpile_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name: Some(name),
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
                    module: _,
                    symbol: _,
                    target_symbol: _,
                } => {
                    let mode = self.transpile_dependency_mode(*mode);
                    let name = name.map(|name| unit.strings.intern_from(&module.ast_strings, name));
                    let alias =
                        alias.map(|alias| unit.strings.intern_from(&module.ast_strings, alias));
                    DependencyItem {
                        mode,
                        kind: if *item_kind != kind {
                            Some(self.transpile_dependency_kind(*item_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                        value: None,
                    }
                }
            };
            let item_id = unit
                .ast
                .insert_from_source(transpiled_item, module.id, *item_id);
            transpiled_item_ids.push(item_id);
        }
        Ok(transpiled_item_ids)
    }
}
