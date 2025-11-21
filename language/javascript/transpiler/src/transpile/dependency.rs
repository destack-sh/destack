use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};
use dyst_ast::StringId;
use dyst_dir::{self as dir, Module, NodeTree};
use dyst_javascript_ast::{DependencyItem, DependencyKind, NodeId};

impl<'a> Transpiler<'a> {
    /// Transpile a dependency kind from DIR into JS AST.
    pub fn transpile_dependency_kind(&self, kind: dir::DependencyKind) -> DependencyKind {
        match kind {
            dir::DependencyKind::Type => DependencyKind::Type,
            dir::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Transpile a dependency item from DIR into JS AST.
    pub fn transpile_dependency_items(
        &self,
        module: &'a Module,
        tree: &NodeTree,
        kind: dir::DependencyKind,
        item_ids: &[dir::NodeId<dir::DependencyItem>],
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<(Option<StringId>, Vec<NodeId<DependencyItem>>)> {
        let mut transpiled_item_ids: Vec<NodeId<DependencyItem>> = Vec::new();
        let mut default_alias: Option<StringId> = None;
        for item_id in item_ids {
            let item = tree.get(*item_id);
            match item {
                dir::DependencyItem::UnresolvedDefault {
                    kind: _,
                    alias,
                    symbol: _,
                } => {
                    let alias = unit.strings.intern_from(&module.ast_strings, *alias);
                    if default_alias.is_some() {
                        return Err(TranspileError::UnsupportedNode {
                            node: item_id.into_any(),
                            message: Some("multiple default items".to_string()),
                        });
                    }
                    default_alias = Some(alias);
                }
                dir::DependencyItem::UnresolvedItem {
                    kind: inner_kind,
                    name,
                    alias,
                    symbol: _,
                } => {
                    let name = unit.strings.intern_from(&module.ast_strings, *name);
                    let alias =
                        alias.map(|alias| unit.strings.intern_from(&module.ast_strings, alias));
                    let item = DependencyItem {
                        kind: if *inner_kind != kind {
                            Some(self.transpile_dependency_kind(*inner_kind))
                        } else {
                            None
                        },
                        name,
                        alias,
                    };
                    let item_id = unit.ast.insert_from_source(item, module.id, *item_id);
                    transpiled_item_ids.push(item_id);
                }
                _ => {
                    return Err(TranspileError::UnsupportedNode {
                        node: item_id.into_any(),
                        message: None,
                    });
                }
            };
        }
        Ok((default_alias, transpiled_item_ids))
    }
}
