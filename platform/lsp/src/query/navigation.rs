use destack_dir::GlobalSymbolId;
use destack_source::{File, ModuleId, PackageId};
use destack_workspace::{Session, query};
use serde_json::json;
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

/// Convert an implementation result to an LSP location.
///
/// Returns the first location if multiple exist.
pub fn implementation_to_location(
    session: &Session,
    result: &query::ImplementationResult,
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
        query::DocumentLinkTarget::File { path } => {
            let uri_str = format!("file://{path}");
            uri_str.parse::<lsp::Uri>().ok()
        }
        query::DocumentLinkTarget::Url { url } => url.parse::<lsp::Uri>().ok(),
        query::DocumentLinkTarget::Position { path, line, column } => {
            // encode position in fragment, e.g. file:///path#L10,5
            let uri_str = format!("file://{path}#L{},{column}", line + 1);
            uri_str.parse::<lsp::Uri>().ok()
        }
    };
    Some(lsp::DocumentLink {
        range,
        target,
        tooltip: link.tooltip.clone(),
        data: None,
    })
}

/// Convert a call hierarchy item to an LSP call hierarchy item.
pub fn call_hierarchy_item_to_lsp(
    session: &Session,
    item: &query::CallHierarchyItem,
) -> Option<lsp::CallHierarchyItem> {
    let file = session.files.get(item.file);
    let uri = file.uri.as_ref().parse::<lsp::Uri>().ok()?;
    let range = byte_span_to_range(&file, item.range);
    let selection_range = byte_span_to_range(&file, item.selection_range);
    let kind = match item.kind {
        query::CallHierarchyKind::Function => lsp::SymbolKind::FUNCTION,
        query::CallHierarchyKind::Method => lsp::SymbolKind::METHOD,
        query::CallHierarchyKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
    };

    // store symbol_id in data for incoming/outgoing calls
    let data = Some(json!({
        "package": item.symbol_id.module_id.package.0,
        "module": item.symbol_id.module_id.local,
        "symbol": item.symbol_id.local_id.id,
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

/// Extract symbol_id from LSP call hierarchy item data.
pub fn call_hierarchy_item_symbol_id(item: &lsp::CallHierarchyItem) -> Option<GlobalSymbolId> {
    let data = item.data.as_ref()?;
    let package = data.get("package")?.as_u64()?;
    let module = data.get("module")?.as_u64()? as u32;
    let symbol = data.get("symbol")?.as_u64()? as u32;

    Some(GlobalSymbolId {
        module_id: ModuleId::new(PackageId(package), module),
        local_id: destack_dir::LocalSymbolId::new(symbol),
    })
}

/// Convert an incoming call to LSP format.
pub fn incoming_call_to_lsp(
    session: &Session,
    call: &query::CallHierarchyIncomingCall,
) -> Option<lsp::CallHierarchyIncomingCall> {
    let from = call_hierarchy_item_to_lsp(session, &call.from)?;
    let file = session.files.get(call.from.file);
    let from_ranges = call
        .from_ranges
        .iter()
        .map(|span| byte_span_to_range(&file, *span))
        .collect();

    Some(lsp::CallHierarchyIncomingCall { from, from_ranges })
}

/// Convert an outgoing call to LSP format.
pub fn outgoing_call_to_lsp(
    session: &Session,
    call: &query::CallHierarchyOutgoingCall,
) -> Option<lsp::CallHierarchyOutgoingCall> {
    let to = call_hierarchy_item_to_lsp(session, &call.to)?;

    // from_ranges are in the caller's file, need to look up via symbol
    // for now, just convert the spans as-is (they should have file info)
    let from_ranges = call
        .from_ranges
        .iter()
        .map(|span| {
            let file = session.files.get(span.file);
            byte_span_to_range(&file, *span)
        })
        .collect();

    Some(lsp::CallHierarchyOutgoingCall { to, from_ranges })
}

/// Convert a type hierarchy item to an LSP type hierarchy item.
pub fn type_hierarchy_item_to_lsp(
    session: &Session,
    item: &query::TypeHierarchyItem,
) -> Option<lsp::TypeHierarchyItem> {
    let file = session.files.get(item.file);
    let uri = file.uri.as_ref().parse::<lsp::Uri>().ok()?;
    let range = byte_span_to_range(&file, item.range);
    let selection_range = byte_span_to_range(&file, item.selection_range);
    let kind = match item.kind {
        query::TypeHierarchyKind::Class => lsp::SymbolKind::CLASS,
        query::TypeHierarchyKind::Interface => lsp::SymbolKind::INTERFACE,
        query::TypeHierarchyKind::Enum => lsp::SymbolKind::ENUM,
        query::TypeHierarchyKind::Struct => lsp::SymbolKind::STRUCT,
        query::TypeHierarchyKind::TypeAlias => lsp::SymbolKind::TYPE_PARAMETER,
    };

    // store symbol_id in data for supertypes/subtypes
    let data = Some(json!({
        "package": item.symbol_id.module_id.package.0,
        "module": item.symbol_id.module_id.local,
        "symbol": item.symbol_id.local_id.id,
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

/// Extract symbol_id from LSP type hierarchy item data.
pub fn type_hierarchy_item_symbol_id(item: &lsp::TypeHierarchyItem) -> Option<GlobalSymbolId> {
    let data = item.data.as_ref()?;
    let package = data.get("package")?.as_u64()?;
    let module = data.get("module")?.as_u64()? as u32;
    let symbol = data.get("symbol")?.as_u64()? as u32;

    Some(GlobalSymbolId {
        module_id: ModuleId::new(PackageId(package), module),
        local_id: destack_dir::LocalSymbolId::new(symbol),
    })
}

/// Convert a workspace symbol to an LSP workspace symbol.
#[allow(deprecated)]
pub fn workspace_symbol_to_lsp(
    session: &Session,
    symbol: &query::WorkspaceSymbol,
) -> Option<lsp::SymbolInformation> {
    let file = session.files.get(symbol.file);
    let uri = file.uri.as_ref().parse::<lsp::Uri>().ok()?;
    let range = byte_span_to_range(&file, symbol.range);
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
