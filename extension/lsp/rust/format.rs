use crate::document::DocumentContent;
use crate::{Document, Workspace};
use dyst_ast::{DystFormatContext, DystFormatOptions, NodeParentIndex};
use dyst_diagnostic::Severity;
use dyst_fir::format;

impl Workspace {
    /// Format a (text) document.
    pub fn format_document(&self, document: &Document) -> Option<String> {
        let DocumentContent::Text {
            source,
            tokens,
            side_tokens,
            side_span,
            ast,
            root_definition_id: module_id,
            ..
        } = &document.content
        else {
            return None;
        };

        // bail if document is malformed
        let Some(module_id) = module_id else {
            return None;
        };
        if self
            .get_diagnostics_for_source(source.id)
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            return None;
        }

        // format with default options
        // NOTE #Incomplete: configure LSP formatting options from Workspace
        let options = DystFormatOptions::default();
        let context = DystFormatContext {
            options,
            source,
            tokens,
            side_tokens,
            side_span,
            tree: ast,
            spans: &ast.spans,
            parents: NodeParentIndex::from_tree(ast),
            session: &self.session,
        };
        let formatted = format!(context, [module_id]).unwrap();
        let printed = formatted.print();
        Some(printed.unwrap().as_str().to_string())
    }
}
