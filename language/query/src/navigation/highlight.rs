use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryPosition};
use crate::dir::SymbolReferenceSearch;
use crate::source::sort_and_dedup_spans;

/// Kind of document highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum HighlightKind {
    /// A textual occurrence.
    #[default]
    Text,
    /// Read access to a symbol.
    Read,
    /// Write access to a symbol.
    Write,
}

/// A highlighted range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHighlight {
    /// The highlighted range.
    pub range: Span,
    /// The kind of highlight.
    pub kind: HighlightKind,
}

impl DocumentHighlight {
    /// Create a text highlight.
    pub fn text(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Text,
        }
    }

    /// Create a read highlight.
    pub fn read(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Read,
        }
    }

    /// Create a write highlight.
    pub fn write(range: Span) -> Self {
        Self {
            range,
            kind: HighlightKind::Write,
        }
    }
}

/// Request highlights at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHighlightRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for document highlight queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHighlightResponse {
    /// Document highlights.
    pub highlights: Vec<DocumentHighlight>,
}

impl ModuleQueryContext<'_> {
    /// Highlight all occurrences of the symbol at the given position in the document.
    ///
    /// Only highlights within the same file (for cross file, use find_references).
    pub fn document_highlights(&self, offset: u32) -> Vec<DocumentHighlight> {
        let Some(symbol_at) = self.find_symbol_at_offset(offset) else {
            return Vec::new();
        };

        let canonical_id = self.canonical_symbol(symbol_at.symbol_id);
        let mut highlights = Vec::new();

        // add definition highlight when it belongs to this file
        if let Some(definition_span) = self.symbol_definition_span(canonical_id)
            && definition_span.file == self.file_id()
        {
            highlights.push(DocumentHighlight::write(definition_span));
        }

        // collect reference highlights inside the current file
        let reference_search = SymbolReferenceSearch {
            include_expressions: true,
            include_members: true,
            include_dependency_items: true,
            include_namespace_receivers: false,
            skip_dependency_aliases: false,
            use_dependency_name_spans: true,
            target_name: None,
            require_target_name_match: false,
            limit_file: Some(self.file_id()),
        };
        let mut reference_spans = self.dir().symbol_references(canonical_id, reference_search);
        sort_and_dedup_spans(&mut reference_spans);

        for span in reference_spans {
            highlights.push(DocumentHighlight::read(span));
        }

        highlights
    }
}
