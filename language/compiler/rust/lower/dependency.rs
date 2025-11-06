use dyst_ast as ast;
use dyst_dir::{
    DependencyItem, DependencyKind, DependencyTarget as DirDependencyTarget, ExportType, NodeId,
};
use dyst_source::FileId;

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
        _file_id: FileId,
        _ast: &ast::NodeTree,
        dependency_type: ast::DependencyKind,
    ) -> DependencyKind {
        match dependency_type {
            ast::DependencyKind::Type => DependencyKind::Type,
            ast::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Lower an import-like binding into DIR import items.
    /// Expressions with grouped items are flattened into scalar import items.
    pub fn lower_dependency_binding(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        origin_id: ast::NodeId<ast::Expression>,
        kind: ast::DependencyKind,
        target: Option<&ast::DependencyTarget>,
        alias: Option<dyst_source::StringId>,
        items: Option<&[ast::NodeId<ast::DependencyItem>]>,
    ) -> Vec<NodeId<DependencyItem>> {
        let kind = self.lower_dependency_kind(file_id, ast, kind);
        let target = target.map(|target| self.lower_dependency_target(file_id, ast, target));
        let alias = alias.map(|alias| self.lower_string_id(file_id, alias));

        let mut items = if let Some(items) = items {
            items
                .iter()
                .map(|item| {
                    let dependency_item = ast.get(*item);
                    let kind = dependency_item
                        .kind
                        .map(|kind| self.lower_dependency_kind(file_id, ast, kind))
                        .unwrap_or(kind);
                    let name = self.lower_string_id(file_id, dependency_item.name);
                    let alias = dependency_item
                        .alias
                        .map(|alias| self.lower_string_id(file_id, alias));
                    let dependency_item = DependencyItem::Scalar {
                        kind,
                        target: target.clone(),
                        name,
                        alias,
                    };
                    self.tree
                        .insert_from_ast(dependency_item, file_id, origin_id)
                })
                .collect()
        } else {
            Vec::new()
        };

        if let Some(target) = target
            && (items.is_empty() || alias.is_some())
        {
            let dependency_item = DependencyItem::Glob {
                kind,
                target,
                alias,
            };
            items.push(
                self.tree
                    .insert_from_ast(dependency_item, file_id, origin_id),
            );
        }

        items
    }

    /// Lower a dependency target into a DIR dependency target.
    fn lower_dependency_target(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        target: &ast::DependencyTarget,
    ) -> DirDependencyTarget {
        match target {
            ast::DependencyTarget::Path(path) => {
                let path = self.lower_path(file_id, ast, path);
                DirDependencyTarget::Path(path)
            }
            ast::DependencyTarget::String(string_id) => {
                let string_id = self.lower_string_id(file_id, *string_id);
                DirDependencyTarget::String(string_id)
            }
        }
    }

    /// Lower a string id into a DIR string id.
    fn lower_string_id(
        &mut self,
        file_id: FileId,
        string_id: dyst_source::StringId,
    ) -> dyst_source::StringId {
        self.intern_string(file_id, string_id)
    }
}
