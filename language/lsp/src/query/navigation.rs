use destack_lsp_types as lsp;
use destack_query as query;
use destack_source::File;
use destack_workspace::{Repository, Revision};
use serde_json::{from_value, json, to_value};

use super::common::{byte_span_to_range, span_to_location, symbol_kind_to_lsp};
use crate::uri::{lsp_uri_for_file, lsp_uri_for_path};

/// Convert one navigation target to an LSP location.
pub fn navigation_target_to_location(
    repository: &Repository,
    revision: Revision,
    target: &query::NavigationTarget,
) -> Option<lsp::Location> {
    span_to_location(repository, revision, target.target.span)
}

/// Convert navigation targets to LSP locations.
pub fn navigation_targets_to_locations(
    repository: &Repository,
    revision: Revision,
    targets: &[query::NavigationTarget],
) -> Vec<lsp::Location> {
    targets
        .iter()
        .filter_map(|target| navigation_target_to_location(repository, revision, target))
        .collect()
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

/// Convert a selection range to an LSP selection range.
pub fn selection_range_to_lsp(file: &File, range: query::SelectionRange) -> lsp::SelectionRange {
    let lsp_range = byte_span_to_range(file, range.range);
    let parent = range
        .parent
        .map(|p| Box::new(selection_range_to_lsp(file, *p)));
    lsp::SelectionRange {
        range: lsp_range,
        parent,
    }
}

/// Convert a document link to an LSP document link.
pub fn document_link_to_lsp(file: &File, link: &query::DocumentLink) -> Option<lsp::DocumentLink> {
    let range = byte_span_to_range(file, link.range);
    let target = match &link.target {
        query::DocumentLinkTarget::File { path } => lsp_uri_for_path(path),
        query::DocumentLinkTarget::Url { url } => url.parse::<lsp::Uri>().ok(),
        query::DocumentLinkTarget::Position { path, line, column } => {
            // encode position in fragment, e.g. file:///path#L10,5
            file_position_uri_from_path_string(path, *line, *column)
        }
    };
    Some(lsp::DocumentLink {
        range,
        target,
        tooltip: link.tooltip.clone(),
        data: None,
    })
}

/// Build a file URI with a line and column fragment.
fn file_position_uri_from_path_string(path: &str, line: u32, column: u32) -> Option<lsp::Uri> {
    let uri = lsp_uri_for_path(path)?;
    let uri = format!("{}#L{},{column}", uri.as_str(), line + 1);
    uri.parse::<lsp::Uri>().ok()
}

/// Convert a call hierarchy item to an LSP call hierarchy item.
pub fn call_hierarchy_item_to_lsp(
    repository: &Repository,
    revision: Revision,
    item: &query::CallHierarchyItem,
) -> Option<lsp::CallHierarchyItem> {
    let file = repository
        .file(revision, item.target.span.file)
        .ok()
        .flatten()?;
    let uri = lsp_uri_for_file(&file)?;
    let range = byte_span_to_range(&file, item.target.span);
    let selection_span = item.target.selection_span.unwrap_or(item.target.span);
    let selection_range = byte_span_to_range(&file, selection_span);
    let kind = match item.kind {
        query::CallHierarchyKind::Function => lsp::SymbolKind::FUNCTION,
        query::CallHierarchyKind::Method => lsp::SymbolKind::METHOD,
        query::CallHierarchyKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
    };

    let query_item = to_value(item).ok()?;
    let data = Some(json!({
        "revision": revision,
        "query_item": query_item,
    }));

    Some(lsp::CallHierarchyItem {
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

/// Extract a query call hierarchy item and revision from lsp item data.
pub fn call_hierarchy_query_item_from_lsp(
    item: &lsp::CallHierarchyItem,
) -> Option<(Revision, query::CallHierarchyItem)> {
    let data = item.data.as_ref()?;
    let revision = data.get("revision")?.clone();
    let revision = from_value(revision).ok()?;
    let query_item = data.get("query_item")?.clone();
    let query_item = from_value(query_item).ok()?;

    Some((revision, query_item))
}

/// Convert an incoming call to LSP format.
pub fn incoming_call_to_lsp(
    repository: &Repository,
    revision: Revision,
    call: &query::CallHierarchyIncomingCall,
) -> Option<lsp::CallHierarchyIncomingCall> {
    let from = call_hierarchy_item_to_lsp(repository, revision, &call.from)?;
    let file = repository
        .file(revision, call.from.target.span.file)
        .ok()
        .flatten()?;
    let from_ranges = call
        .from_ranges
        .iter()
        .map(|span| byte_span_to_range(&file, *span))
        .collect();

    Some(lsp::CallHierarchyIncomingCall { from, from_ranges })
}

/// Convert an outgoing call to LSP format.
pub fn outgoing_call_to_lsp(
    repository: &Repository,
    revision: Revision,
    call: &query::CallHierarchyOutgoingCall,
) -> Option<lsp::CallHierarchyOutgoingCall> {
    let to = call_hierarchy_item_to_lsp(repository, revision, &call.to)?;

    let from_ranges = call
        .from_ranges
        .iter()
        .filter_map(|span| {
            repository
                .file(revision, span.file)
                .ok()
                .flatten()
                .map(|file| byte_span_to_range(&file, *span))
        })
        .collect();

    Some(lsp::CallHierarchyOutgoingCall { to, from_ranges })
}

/// Convert a type hierarchy item to an LSP type hierarchy item.
pub fn type_hierarchy_item_to_lsp(
    repository: &Repository,
    revision: Revision,
    item: &query::TypeHierarchyItem,
) -> Option<lsp::TypeHierarchyItem> {
    let file = repository
        .file(revision, item.target.span.file)
        .ok()
        .flatten()?;
    let uri = lsp_uri_for_file(&file)?;
    let range = byte_span_to_range(&file, item.target.span);
    let selection_span = item.target.selection_span.unwrap_or(item.target.span);
    let selection_range = byte_span_to_range(&file, selection_span);
    let kind = match item.kind {
        query::TypeHierarchyKind::Class => lsp::SymbolKind::CLASS,
        query::TypeHierarchyKind::Interface => lsp::SymbolKind::INTERFACE,
        query::TypeHierarchyKind::Enum => lsp::SymbolKind::ENUM,
        query::TypeHierarchyKind::Struct => lsp::SymbolKind::STRUCT,
        query::TypeHierarchyKind::TypeAlias => lsp::SymbolKind::TYPE_PARAMETER,
    };

    let query_item = to_value(item).ok()?;
    let data = Some(json!({
        "revision": revision,
        "query_item": query_item,
    }));

    Some(lsp::TypeHierarchyItem {
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

/// Extract a query type hierarchy item and revision from lsp item data.
pub fn type_hierarchy_query_item_from_lsp(
    item: &lsp::TypeHierarchyItem,
) -> Option<(Revision, query::TypeHierarchyItem)> {
    let data = item.data.as_ref()?;
    let revision = data.get("revision")?.clone();
    let revision = from_value(revision).ok()?;
    let query_item = data.get("query_item")?.clone();
    let query_item = from_value(query_item).ok()?;

    Some((revision, query_item))
}

/// Convert a workspace symbol to an LSP workspace symbol.
#[allow(deprecated)]
pub fn workspace_symbol_to_lsp(
    repository: &Repository,
    revision: Revision,
    symbol: &query::WorkspaceSymbol,
) -> Option<lsp::SymbolInformation> {
    let file = repository
        .file(revision, symbol.target.span.file)
        .ok()
        .flatten()?;
    let uri = lsp_uri_for_file(&file)?;
    let range = byte_span_to_range(&file, symbol.target.span);
    let kind = symbol_kind_to_lsp(symbol.kind);

    Some(lsp::SymbolInformation {
        name: symbol.name.clone(),
        kind,
        tags: None,
        deprecated: None,
        location: lsp::Location { uri, range },
        container_name: symbol.container.clone(),
    })
}
