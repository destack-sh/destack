use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::query::common::{
    ReferenceCollectionOptions, collect_symbol_references_in_context, find_symbol_at_offset,
    get_canonical_symbol, get_symbol_definition_span, sort_and_dedup_spans,
    with_query_context_for_file,
};

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
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for document highlight queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentHighlightResponse {
    /// Document highlights.
    pub highlights: Vec<DocumentHighlight>,
}

/// Highlight all occurrences of the symbol at the given position in the document.
///
/// Only highlights within the same file (for cross file, use find_references).
pub fn document_highlights(session: &Session, file: FileId, offset: u32) -> Vec<DocumentHighlight> {
    // find the symbol at offset
    let Some(symbol_at) = find_symbol_at_offset(session, file, offset) else {
        return Vec::new();
    };

    // get canonical symbol and resolve imports
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    with_query_context_for_file(session, file, |ctx| {
        // initialize highlight collection
        let mut highlights = Vec::new();

        // check if the definition is in this file, add as write highlight
        if let Some(definition_span) = get_symbol_definition_span(session, canonical_id)
            && definition_span.file == ctx.file_id
        {
            highlights.push(DocumentHighlight::write(definition_span));
        }

        // collect reference spans within the current file
        let reference_options = ReferenceCollectionOptions {
            include_expressions: true,
            include_members: true,
            include_dependencies: false,
            include_namespace_members: false,
            skip_dependency_aliases: false,
            use_dependency_name_spans: false,
            target_name: None,
            limit_to_file: Some(ctx.file_id),
        };

        let mut reference_spans =
            collect_symbol_references_in_context(session, &ctx, canonical_id, reference_options);
        sort_and_dedup_spans(&mut reference_spans);

        // convert reference spans into read highlights
        for span in reference_spans {
            highlights.push(DocumentHighlight::read(span));
        }

        highlights
    })
    .unwrap_or_default()
}
