use dyst_ast as ast;
use dyst_dir::{DependencyItem, DependencyKind, DependencySource, ExportType, Module, NodeId};
use dyst_source::StringId;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Lower an export type to a DIR export type.
    pub fn lower_export_type(&mut self, export_type: ast::ExportType) -> ExportType {
        match export_type {
            ast::ExportType::Item => ExportType::Item,
            ast::ExportType::Default => ExportType::Default,
            ast::ExportType::Module => ExportType::Module,
        }
    }

    /// Lower a dependency type into a DIR dependency type.
    pub fn lower_dependency_kind(
        &mut self,
        dependency_type: ast::DependencyKind,
    ) -> DependencyKind {
        match dependency_type {
            ast::DependencyKind::Type => DependencyKind::Type,
            ast::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Lower an import-like binding into DIR import items.
    /// Expressions with grouped items are flattened into scalar import items.
    pub fn lower_dependency_items(
        &mut self,
        module: &Module,
        origin_id: ast::NodeId<ast::Expression>,
        kind: ast::DependencyKind,
        _source: DependencySource,
        target: Option<StringId>,
        alias: Option<StringId>,
        items: Option<&[ast::NodeId<ast::DependencyItem>]>,
    ) -> Vec<NodeId<DependencyItem>> {
        let kind = self.lower_dependency_kind(kind);
        let alias = alias.map(|alias| self.session.strings.intern_from(&module.strings, alias));

        let mut items = if let Some(items) = items {
            items
                .iter()
                .map(|item| {
                    let dependency_item = module.get(*item);
                    let kind = dependency_item
                        .kind
                        .map(|kind| self.lower_dependency_kind(kind))
                        .unwrap_or(kind);
                    let name = self
                        .session
                        .strings
                        .intern_from(&module.strings, dependency_item.name);
                    let alias = dependency_item
                        .alias
                        .map(|alias| self.session.strings.intern_from(&module.strings, alias));
                    let dependency_item = DependencyItem::Named {
                        kind,
                        target,
                        name,
                        alias,
                    };
                    self.session
                        .tree
                        .insert_from_ast(dependency_item, module.id, origin_id)
                })
                .collect()
        } else {
            Vec::new()
        };

        if let Some(target) = target {
            if let Some(alias) = alias {
                let dependency_item = DependencyItem::Namespace {
                    kind,
                    target,
                    alias,
                };

                items.push(self.session.tree.insert_from_ast(
                    dependency_item,
                    module.id,
                    origin_id,
                ));
            } else if items.is_empty() {
                let dependency_item = DependencyItem::SideEffect { kind, target };
                items.push(self.session.tree.insert_from_ast(
                    dependency_item,
                    module.id,
                    origin_id,
                ));
            }
        }

        items
    }
}
