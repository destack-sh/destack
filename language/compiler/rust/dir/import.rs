use dyst_ast as ast;
use dyst_dir::{ExportMode, ImportItem, ImportTarget as DirImportTarget, NodeId};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower an export mode to a DIR export mode.
    pub fn lower_export_mode(&mut self, export_mode: ast::ExportMode) -> ExportMode {
        match export_mode {
            ast::ExportMode::Item => ExportMode::Item,
            ast::ExportMode::Default => ExportMode::Default,
        }
    }

    /// Lower an import-like binding into DIR import items.
    /// Expressions with grouped items are flattened into scalar import items.
    pub fn lower_import_binding(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        origin_id: ast::NodeId<ast::Expression>,
        target: Option<&ast::ImportTarget>,
        alias: Option<dyst_source::StringId>,
        items: Option<&[ast::NodeId<ast::ImportItem>]>,
    ) -> Vec<NodeId<ImportItem>> {
        let lowered_target = target.map(|target| self.lower_import_target(source_id, ast, target));
        let lowered_alias = alias.map(|alias| self.lower_string_id(source_id, alias));

        if let Some(items) = items {
            return items
                .iter()
                .map(|item| {
                    let import_item = ast.get(*item);
                    let name = self.lower_string_id(source_id, import_item.name);
                    let alias = import_item
                        .alias
                        .map(|alias| self.lower_string_id(source_id, alias));
                    let import_item = ImportItem::Scalar {
                        target: lowered_target.clone(),
                        name,
                        alias,
                    };
                    self.tree.insert_from_ast(import_item, source_id, origin_id)
                })
                .collect();
        }

        if let Some(target) = lowered_target {
            let import_item = ImportItem::Glob {
                target,
                alias: lowered_alias,
            };
            vec![self.tree.insert_from_ast(import_item, source_id, origin_id)]
        } else {
            Vec::new()
        }
    }

    fn lower_import_target(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        target: &ast::ImportTarget,
    ) -> DirImportTarget {
        match target {
            ast::ImportTarget::Path(path) => {
                let path = self.lower_path(source_id, ast, path);
                DirImportTarget::Path(path)
            }
            ast::ImportTarget::Virtual(string_id) => {
                let string_id = self.lower_string_id(source_id, *string_id);
                DirImportTarget::Virtual(string_id)
            }
        }
    }

    fn lower_string_id(
        &mut self,
        source_id: SourceId,
        string_id: dyst_source::StringId,
    ) -> dyst_source::StringId {
        self.intern_string(source_id, string_id)
    }
}
