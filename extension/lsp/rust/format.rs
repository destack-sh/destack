use crate::{Document, Workspace};
use dyst_ast::{DystFormatContext, DystFormatOptions, NodeParentIndex};
use dyst_diagnostic::Severity;
use dyst_fir::format;

impl Workspace {
    /// Format a document.
    pub fn format_document(&self, document: &Document) -> String {
        // bail if document is malformed
        if document.module_id.is_none()
            || self
                .get_diagnostics_for_source(document.source.id)
                .iter()
                .any(|d| d.severity == Severity::Error)
        {
            return document.source.content.clone();
        }

        // format with default options
        // NOTE @Incomplete: configure LSP formatting options from Workspace
        let options = DystFormatOptions::default();
        let context = DystFormatContext {
            options,
            source: &document.source,
            session: &self.session,
            tree: &document.ast,
            spans: &document.ast.spans,
            parents: NodeParentIndex::from_tree(&document.ast),
        };
        let formatted = format!(context, [document.module_id]).unwrap();
        let printed = formatted.print();
        printed.unwrap().as_str().to_string()
    }
}
