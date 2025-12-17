//! Navigation-related conversions (definition, references, highlights, symbols).

use destack_source::File;
use destack_workspace::{Session, query};
use tower_lsp_server::lsp_types as lsp;

use super::common::{byte_span_to_range, span_to_location, symbol_kind_to_lsp};

/// Convert a definition result to an LSP location.
///
/// Returns the first location if multiple exist.
pub fn definition_to_location(
    session: &Session,
    result: &query::DefinitionResult,
) -> Option<lsp::Location> {
    let span = result.locations.first()?;
    span_to_location(session, *span)
}

/// Convert a document highlight to an LSP document highlight.
pub fn document_highlight_to_lsp(
    file: &File,
    highlight: &query::DocumentHighlight,
) -> Option<lsp::DocumentHighlight> {
    let range = byte_span_to_range(file, highlight.range);
    let kind = match highlight.kind {
        query::HighlightKind::Text => Some(lsp::DocumentHighlightKind::TEXT),
        query::HighlightKind::Read => Some(lsp::DocumentHighlightKind::READ),
        query::HighlightKind::Write => Some(lsp::DocumentHighlightKind::WRITE),
    };
    Some(lsp::DocumentHighlight { range, kind })
}

/// Convert a document symbol to an LSP document symbol.
#[allow(deprecated)]
pub fn document_symbol_to_lsp(
    file: &File,
    symbol: &query::DocumentSymbol,
) -> Option<lsp::DocumentSymbol> {
    let range = byte_span_to_range(file, symbol.range);
    let selection_range = byte_span_to_range(file, symbol.selection_range);
    let kind = symbol_kind_to_lsp(symbol.kind);

    // recursively convert children
    let children = if symbol.children.is_empty() {
        None
    } else {
        let converted: Vec<_> = symbol
            .children
            .iter()
            .filter_map(|c| document_symbol_to_lsp(file, c))
            .collect();
        if converted.is_empty() {
            None
        } else {
            Some(converted)
        }
    };

    Some(lsp::DocumentSymbol {
        name: symbol.name.clone(),
        detail: symbol.detail.clone(),
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children,
    })
}
