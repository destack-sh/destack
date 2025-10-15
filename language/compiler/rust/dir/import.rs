use dyst_ast as ast;
use dyst_dir::{ExportMode, ImportItem, NodeId};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower an export mode to a DIR export mode.
    pub fn lower_export_mode(
        &mut self,
        _source_id: SourceId,
        _ast: &ast::NodeTree,
        export_mode: ast::ExportMode,
    ) -> ExportMode {
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
        let path = self.lower_path(source_id, ast, &import_clause.target);

        // flatten items
        if let Some(items) = &import_clause.items {
            items
                .iter()
                .map(|item| {
                    let import_item = ast.get(*item);
                    let alias = import_item
                        .alias
                        .map(|alias| self.intern_string(source_id, alias));
                    let import_item = ImportItem {
                        target: path.clone(),
                        alias,
                    };
                    self.tree.insert(import_item, source_id, import_clause_id)
                })
                .collect()
        }
        // single item
        else {
            let alias = import_clause
                .alias
                .map(|alias| self.intern_string(source_id, alias));
            let import_item = ImportItem {
                target: path,
                alias,
            };
            vec![self.tree.insert(import_item, source_id, import_clause_id)]
        }
    }
}
