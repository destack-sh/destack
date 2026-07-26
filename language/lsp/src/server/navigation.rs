use std::path::{Path, PathBuf};

use destack_lsp_server::jsonrpc;
use destack_lsp_types as lsp;
use destack_query as query;
use destack_repository::Revision;
use destack_source::{File, Span};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value};

use super::error::{internal_error, workspace_error};
use super::source::SourceFiles;
use super::{position, symbol};
use crate::uri;

/// State carried between hierarchy requests.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct HierarchyContinuation<T> {
    /// The workspace path that anchors the query program.
    pub(super) path: PathBuf,
    /// The semantic revision that produced the item.
    pub(super) revision: Revision,
    /// The query item expanded by the next request.
    pub(super) item: T,
}

impl<T> HierarchyContinuation<T> {
    /// Create hierarchy continuation state.
    fn new(path: &Path, revision: Revision, item: T) -> Self {
        Self {
            path: path.to_path_buf(),
            revision,
            item,
        }
    }
}

impl<T: Serialize> HierarchyContinuation<T> {
    /// Encode hierarchy continuation state for an LSP data field.
    fn encode(self) -> jsonrpc::Result<serde_json::Value> {
        to_value(self).map_err(internal_error)
    }
}

impl<T: DeserializeOwned> HierarchyContinuation<T> {
    /// Decode hierarchy continuation state from an LSP data field.
    pub(super) fn decode(data: Option<&serde_json::Value>) -> Result<Self, String> {
        let data = data.ok_or_else(|| "hierarchy item has no query payload".to_string())?;
        from_value(data.clone())
            .map_err(|error| format!("hierarchy item has an invalid query payload: {error}"))
    }
}

/// Convert one navigation target to an LSP location link.
fn link(
    target: &query::NavigationTarget,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::LocationLink> {
    // map the authored origin
    let origin_file = files
        .file(target.origin.span.file)
        .map_err(workspace_error)?;
    let origin_selection_range = position::range(&origin_file, target.origin.span)?;

    // map the destination ranges
    let file = files
        .file(target.target.span.file)
        .map_err(workspace_error)?;
    let target_uri = uri::file(&file)?;
    let (target_range, target_selection_range) =
        ranges(&file, target.target.span, target.target.selection_span)?;

    Ok(lsp::LocationLink {
        origin_selection_range: Some(origin_selection_range),
        target_uri,
        target_range,
        target_selection_range,
    })
}

/// Convert navigation targets to LSP location links.
pub(super) fn links(
    targets: &[query::NavigationTarget],
    files: &SourceFiles,
) -> jsonrpc::Result<Vec<lsp::LocationLink>> {
    targets.iter().map(|target| link(target, files)).collect()
}

/// Convert a document highlight to an LSP document highlight.
pub(super) fn highlight(
    file: &File,
    highlight: &query::Highlight,
) -> jsonrpc::Result<lsp::DocumentHighlight> {
    let range = position::range(file, highlight.range)?;
    let kind = match highlight.kind {
        query::HighlightKind::Text => lsp::DocumentHighlightKind::TEXT,
        query::HighlightKind::Read => lsp::DocumentHighlightKind::READ,
        query::HighlightKind::Write => lsp::DocumentHighlightKind::WRITE,
    };

    Ok(lsp::DocumentHighlight {
        range,
        kind: Some(kind),
    })
}

/// Convert a document symbol to an LSP document symbol.
#[allow(deprecated)]
pub(super) fn outline(
    file: &File,
    symbol: &query::OutlineSymbol,
) -> jsonrpc::Result<lsp::DocumentSymbol> {
    let (range, selection_range) = ranges(file, symbol.range, symbol.selection_range)?;
    let kind = symbol::kind(symbol.kind);

    // recursively convert children
    let children = if symbol.children.is_empty() {
        None
    } else {
        let converted = symbol
            .children
            .iter()
            .map(|child| outline(file, child))
            .collect::<jsonrpc::Result<Vec<_>>>()?;

        Some(converted)
    };

    Ok(lsp::DocumentSymbol {
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

/// Convert a selection range to an LSP selection range.
pub(super) fn selection(
    file: &File,
    range: query::SelectionRange,
) -> jsonrpc::Result<lsp::SelectionRange> {
    let lsp_range = position::range(file, range.range)?;
    let parent = range
        .parent
        .map(|parent| selection(file, *parent).map(Box::new))
        .transpose()?;

    Ok(lsp::SelectionRange {
        range: lsp_range,
        parent,
    })
}

/// Convert a document link to an LSP document link.
pub(super) fn document_link(file: &File, link: &query::Link) -> jsonrpc::Result<lsp::DocumentLink> {
    let range = position::range(file, link.range)?;
    let target = uri::path(&link.path).ok_or_else(|| {
        internal_error(format!(
            "document link path has no representable LSP URI: {}",
            link.path.display()
        ))
    })?;

    Ok(lsp::DocumentLink {
        range,
        target: Some(target),
        tooltip: Some(link.tooltip.clone()),
        data: None,
    })
}

/// Convert a call item to an LSP call item.
pub(super) fn call_item(
    path: &Path,
    revision: Revision,
    item: &query::CallItem,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::CallHierarchyItem> {
    let file = files.file(item.target.span.file).map_err(workspace_error)?;
    let uri = uri::file(&file)?;
    let (range, selection_range) = ranges(&file, item.target.span, item.target.selection_span)?;
    let kind = match item.kind {
        query::CallItemKind::Function => lsp::SymbolKind::FUNCTION,
        query::CallItemKind::Method => lsp::SymbolKind::METHOD,
        query::CallItemKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
    };

    let continuation = HierarchyContinuation::new(path, revision, item.clone());
    let data = Some(continuation.encode()?);

    Ok(lsp::CallHierarchyItem {
        name: item.name.clone(),
        kind,
        tags: None,
        detail: item.detail.clone(),
        uri,
        range,
        selection_range,
        data,
    })
}

/// Convert an incoming call to LSP format.
pub(super) fn incoming_call(
    path: &Path,
    revision: Revision,
    call: &query::IncomingCall,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::CallHierarchyIncomingCall> {
    let from = call_item(path, revision, &call.from, files)?;
    let file = files
        .file(call.from.target.span.file)
        .map_err(workspace_error)?;
    if let Some(span) = call.from_ranges.iter().find(|span| span.file != file.id) {
        return Err(internal_error(format!(
            "incoming call range belongs to another file: {span:?}"
        )));
    }
    let from_ranges = call
        .from_ranges
        .iter()
        .map(|span| position::range(&file, *span))
        .collect::<jsonrpc::Result<Vec<_>>>()?;

    Ok(lsp::CallHierarchyIncomingCall { from, from_ranges })
}

/// Convert an outgoing call to LSP format.
pub(super) fn outgoing_call(
    path: &Path,
    revision: Revision,
    call: &query::OutgoingCall,
    source_file: &File,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::CallHierarchyOutgoingCall> {
    let to = call_item(path, revision, &call.to, files)?;
    if let Some(span) = call
        .from_ranges
        .iter()
        .find(|span| span.file != source_file.id)
    {
        return Err(internal_error(format!(
            "outgoing call range belongs to another file: {span:?}"
        )));
    }

    let from_ranges = call
        .from_ranges
        .iter()
        .map(|span| position::range(source_file, *span))
        .collect::<jsonrpc::Result<Vec<_>>>()?;

    Ok(lsp::CallHierarchyOutgoingCall { to, from_ranges })
}

/// Convert a type item to an LSP type hierarchy item.
pub(super) fn type_item(
    path: &Path,
    revision: Revision,
    item: &query::TypeItem,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::TypeHierarchyItem> {
    let file = files.file(item.target.span.file).map_err(workspace_error)?;
    let uri = uri::file(&file)?;
    let (range, selection_range) = ranges(&file, item.target.span, item.target.selection_span)?;
    let kind = symbol::kind(item.kind);

    let continuation = HierarchyContinuation::new(path, revision, item.clone());
    let data = Some(continuation.encode()?);

    Ok(lsp::TypeHierarchyItem {
        name: item.name.clone(),
        kind,
        tags: None,
        detail: item.detail.clone(),
        uri,
        range,
        selection_range,
        data,
    })
}

/// Convert a symbol search to an LSP symbol search.
#[allow(deprecated)]
pub(super) fn search_symbol(
    symbol: &query::SearchSymbol,
    files: &SourceFiles,
) -> jsonrpc::Result<lsp::SymbolInformation> {
    let file = files
        .file(symbol.target.span.file)
        .map_err(workspace_error)?;
    let uri = uri::file(&file)?;
    let range = position::range(&file, symbol.target.span)?;
    let kind = symbol::kind(symbol.kind);

    Ok(lsp::SymbolInformation {
        name: symbol.name.clone(),
        kind,
        tags: None,
        deprecated: None,
        location: lsp::Location { uri, range },
        container_name: symbol.container.clone(),
    })
}

/// Convert one full source range and its contained selection range.
fn ranges(
    file: &File,
    range: Span,
    selection_range: Span,
) -> jsonrpc::Result<(lsp::Range, lsp::Range)> {
    let is_contained = range.file == selection_range.file
        && range.start <= selection_range.start
        && selection_range.end <= range.end;
    if !is_contained {
        return Err(internal_error(format!(
            "selection range {selection_range:?} is not contained by target range {range:?}"
        )));
    }

    let range = position::range(file, range)?;
    let selection_range = position::range(file, selection_range)?;

    Ok((range, selection_range))
}
