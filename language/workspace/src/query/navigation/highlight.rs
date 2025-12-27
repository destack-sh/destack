use destack_dir::Expression;
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::{
    find_symbol_at_offset, get_canonical_symbol, get_dir_node_span, get_module_by_file_id,
    get_symbol_definition_span,
};

/// Kind of document highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone)]
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

/// Highlight all occurrences of the symbol at the given position in the document.
///
/// Only highlights within the same file (for cross-file, use find_references).
pub fn document_highlight(session: &Session, file: FileId, offset: u32) -> Vec<DocumentHighlight> {
    // 1. find the symbol at offset
    let Some(symbol_at) = find_symbol_at_offset(session, file, offset) else {
        return Vec::new();
    };

    // 2. get canonical symbol (resolve imports)
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);

    // 3. get the module for this file
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };

    let mut highlights = Vec::new();

    // check if the definition is in this file, add as Write highlight
    if let Some(definition_span) = get_symbol_definition_span(session, canonical_id)
        && definition_span.file == ctx.file_id
    {
        highlights.push(DocumentHighlight::write(definition_span));
    }

    // collect matching expression ids first to avoid borrow issues
    let matching_expression_ids: Vec<_> = {
        let dir_tree = ctx.tree();
        dir_tree
            .iter_nodes_of_type::<Expression>()
            .filter_map(|(expression_id, expression)| {
                if let Some(target) = expression.target_symbol() {
                    let target_canonical = get_canonical_symbol(session, target);
                    if target_canonical == canonical_id {
                        return Some(expression_id);
                    }
                }
                None
            })
            .collect()
    };

    // get spans for each matching expression
    for expression_id in matching_expression_ids {
        if let Some(span) = get_dir_node_span(ctx.ast, ctx.dir, expression_id.into()) {
            // only include if in this file (should always be true for single module)
            if span.file == ctx.file_id {
                // classify as Read (basic classification - definition was already added as Write)
                highlights.push(DocumentHighlight::read(span));
            }
        }
    }

    highlights
}
