use dyst_ast as ast;
use dyst_dir::{
    DependencyItem, DependencyTarget as DirDependencyTarget, DependencyKind, ExportType, NodeId,
};
use dyst_source::SourceId;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Lower an export mode to a DIR export mode.
    pub fn lower_export_mode(&mut self, export_mode: ast::ExportType) -> ExportType {
        match export_mode {
            ast::ExportType::Item => ExportType::Item,
            ast::ExportType::Default => ExportType::Default,
        }
    }

    /// Lower a dependency type into a DIR dependency type.
    pub fn lower_dependency_type(
        &mut self,
        _source_id: SourceId,
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
        source_id: SourceId,
        ast: &ast::NodeTree,
        origin_id: ast::NodeId<ast::Expression>,
        ty: ast::DependencyKind,
        target: Option<&ast::DependencyTarget>,
        alias: Option<dyst_source::StringId>,
        items: Option<&[ast::NodeId<ast::DependencyItem>]>,
    ) -> Vec<NodeId<DependencyItem>> {
        let ty = self.lower_dependency_type(source_id, ast, ty);
        let target = target.map(|target| self.lower_dependency_target(source_id, ast, target));
        let alias = alias.map(|alias| self.lower_string_id(source_id, alias));

        if let Some(items) = items {
            return items
                .iter()
                .map(|item| {
                    let import_item = ast.get(*item);
                    let ty = self.lower_dependency_type(source_id, ast, import_item.kind);
                    let name = self.lower_string_id(source_id, import_item.name);
                    let alias = import_item
                        .alias
                        .map(|alias| self.lower_string_id(source_id, alias));
                    let import_item = DependencyItem::Scalar {
                        kind: ty,
                        target: target.clone(),
                        name,
                        alias,
                    };
                    self.tree.insert_from_ast(import_item, source_id, origin_id)
                })
                .collect();
        }

        if let Some(target) = target {
            let import_item = DependencyItem::Glob { kind: ty, target, alias };
            vec![self.tree.insert_from_ast(import_item, source_id, origin_id)]
        } else {
            Vec::new()
        }
    }

    /// Lower a dependency target into a DIR dependency target.
    fn lower_dependency_target(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        target: &ast::DependencyTarget,
    ) -> DirDependencyTarget {
        match target {
            ast::DependencyTarget::Path(path) => {
                let path = self.lower_path(source_id, ast, path);
                DirDependencyTarget::Path(path)
            }
            ast::DependencyTarget::Virtual(string_id) => {
                let string_id = self.lower_string_id(source_id, *string_id);
                DirDependencyTarget::Virtual(string_id)
            }
        }
    }

    /// Lower a string id into a DIR string id.
    fn lower_string_id(
        &mut self,
        source_id: SourceId,
        string_id: dyst_source::StringId,
    ) -> dyst_source::StringId {
        self.intern_string(source_id, string_id)
    }
}
