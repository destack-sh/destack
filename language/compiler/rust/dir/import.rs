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

    /// Lower a import clause to a DIR import items.
    /// Import clauses with multiple items are flattened into multiple import items.
    pub fn lower_import_clause(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        import_clause_id: ast::NodeId<ast::ImportClause>,
    ) -> Vec<NodeId<ImportItem>> {
        let import_clause = ast.get(import_clause_id);
        let target = self.lower_import_target(source_id, ast, &import_clause.target);

        match &import_clause.items {
            Some(items) => items
                .iter()
                .map(|item| {
                    let import_item = ast.get(*item);
                    let name = self.lower_string_id(source_id, import_item.name);
                    let alias = import_item
                        .alias
                        .map(|alias| self.lower_string_id(source_id, alias));
                    let import_item = ImportItem::Scalar {
                        target: target.clone(),
                        name,
                        alias,
                    };
                    self.tree.insert(import_item, source_id, import_clause_id)
                })
                .collect(),
            None => {
                let alias = import_clause
                    .alias
                    .map(|alias| self.lower_string_id(source_id, alias));
                let import_item = ImportItem::Glob { target, alias };
                vec![self.tree.insert(import_item, source_id, import_clause_id)]
            }
        }
    }

    fn lower_import_target(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        target: &ast::ImportTarget,
    ) -> DirImportTarget {
        match target {
            ast::ImportTarget::Virtual(path) => {
                let path = self.lower_path(source_id, ast, path);
                DirImportTarget::Virtual(path)
            }
            ast::ImportTarget::Physical(string_id) => {
                let string_id = self.lower_string_id(source_id, *string_id);
                DirImportTarget::Physical(string_id)
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
