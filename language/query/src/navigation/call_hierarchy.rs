use std::collections::HashMap;

use destack_dir::{GlobalSymbolId, SymbolForm};
use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::core::{call_candidates_for_callee, call_candidates_for_caller, query_context};
use crate::dir::{
    find_symbol_at_offset, get_canonical_symbol, get_symbol_declaration_span,
    get_symbol_definition_span, resolve_symbol_name,
};
use crate::source::sort_and_dedup_spans;
/// An item in the call hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    /// The name of the item (function/method name).
    pub name: String,
    /// The kind of item.
    pub kind: CallHierarchyKind,
    /// Detail (e.g., signature).
    pub detail: Option<String>,
    /// The file containing this item.
    pub file: FileId,
    /// The full range of the item.
    pub range: Span,
    /// The range of the item's name.
    pub selection_range: Span,
    /// The symbol ID (internal use for follow-up queries).
    pub symbol_id: GlobalSymbolId,
}

/// Kind of call hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallHierarchyKind {
    Function,
    Method,
    Constructor,
}

/// An incoming call (who calls this function).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingCall {
    /// The item that contains the call sites.
    pub from: CallHierarchyItem,
    /// The ranges of the actual call expressions within `from`.
    pub from_ranges: Vec<Span>,
}

/// An outgoing call (what does this function call).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingCall {
    /// The item being called.
    pub to: CallHierarchyItem,
    /// The ranges of the call expressions to `to`.
    pub from_ranges: Vec<Span>,
}

/// Request prepare call hierarchy at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareCallHierarchyRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for prepare call hierarchy queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareCallHierarchyResponse {
    /// Call hierarchy item, if available.
    pub item: Option<CallHierarchyItem>,
}

/// Request incoming call hierarchy edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingRequest {
    /// The call hierarchy item to expand.
    pub item: CallHierarchyItem,
}

/// Response payload for call hierarchy incoming queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyIncomingResponse {
    /// Incoming calls.
    pub calls: Vec<CallHierarchyIncomingCall>,
}

/// Request outgoing call hierarchy edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingRequest {
    /// The call hierarchy item to expand.
    pub item: CallHierarchyItem,
}

/// Response payload for call hierarchy outgoing queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOutgoingResponse {
    /// Outgoing calls.
    pub calls: Vec<CallHierarchyOutgoingCall>,
}

/// Prepare a call hierarchy item at the given position.
///
/// Returns the item if the position is on a callable (function, method).
pub fn prepare_call_hierarchy(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<CallHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(repository, revision, file, offset)?;
    let canonical_id = get_canonical_symbol(repository, revision, symbol_at.symbol_id);
    let ctx = query_context(repository, revision, canonical_id.module_id)?;
    let name = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        // check if it's a function
        if symbol.form != SymbolForm::Function {
            return None;
        }

        // get the name
        resolve_symbol_name(repository, revision, canonical_id)?
    };

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(repository, revision, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range =
        get_symbol_declaration_span(repository, revision, canonical_id).unwrap_or(selection_range);

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}

/// Get incoming calls to a call hierarchy item.
///
/// "Who calls this function?"
pub fn incoming_calls(
    repository: &Repository,
    revision: Revision,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyIncomingCall> {
    let canonical_id = get_canonical_symbol(repository, revision, item.symbol_id);
    let mut incoming_by_caller: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
    for entry in call_candidates_for_callee(repository, revision, canonical_id) {
        let Some(caller_symbol) = entry.caller_symbol else {
            continue;
        };

        incoming_by_caller
            .entry(caller_symbol)
            .or_default()
            .push(entry.span);
    }

    let mut incoming = Vec::new();
    for (caller_symbol, call_spans) in incoming_by_caller {
        if let Some(caller_item) =
            call_hierarchy_item_from_symbol(repository, revision, caller_symbol)
        {
            incoming.push(CallHierarchyIncomingCall {
                from: caller_item,
                from_ranges: call_spans,
            });
        }
    }

    // sort call ranges and incoming callers for stable protocol output
    for call in &mut incoming {
        sort_and_dedup_spans(&mut call.from_ranges);
    }

    incoming.sort_by(|left, right| call_item_key(&left.from).cmp(&call_item_key(&right.from)));

    incoming
}

/// Get outgoing calls from a call hierarchy item.
///
/// "What does this function call?"
pub fn outgoing_calls(
    repository: &Repository,
    revision: Revision,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyOutgoingCall> {
    let canonical_id = get_canonical_symbol(repository, revision, item.symbol_id);
    let mut calls_with_spans: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
    for entry in call_candidates_for_caller(repository, revision, canonical_id) {
        let callee_symbol = get_canonical_symbol(repository, revision, entry.callee_symbol);

        calls_with_spans
            .entry(callee_symbol)
            .or_default()
            .push(entry.span);
    }

    // convert to outgoing calls
    let mut outgoing = Vec::new();
    for (target_symbol_id, call_spans) in calls_with_spans {
        // only include function calls
        let Some(target_ctx) = query_context(repository, revision, target_symbol_id.module_id)
        else {
            continue;
        };
        let is_function = {
            let target_symbols = target_ctx.dir().symbols();
            let target_symbol = target_symbols.get_symbol(target_symbol_id.local_id);
            target_symbol.form == SymbolForm::Function
        };
        if !is_function {
            continue;
        }

        if let Some(target_item) =
            call_hierarchy_item_from_symbol(repository, revision, target_symbol_id)
        {
            outgoing.push(CallHierarchyOutgoingCall {
                to: target_item,
                from_ranges: call_spans,
            });
        }
    }

    // sort call ranges and outgoing callees for stable protocol output
    for call in &mut outgoing {
        sort_and_dedup_spans(&mut call.from_ranges);
    }

    outgoing.sort_by(|left, right| call_item_key(&left.to).cmp(&call_item_key(&right.to)));

    outgoing
}

/// Build a stable ordering key for a call hierarchy item.
fn call_item_key(item: &CallHierarchyItem) -> (u128, u32, u32, u32, u32, u8, &str) {
    (
        item.file.0,
        item.range.start,
        item.range.end,
        item.selection_range.start,
        item.selection_range.end,
        call_kind_rank(item.kind),
        item.name.as_str(),
    )
}

/// Rank call hierarchy kinds for stable ordering.
fn call_kind_rank(kind: CallHierarchyKind) -> u8 {
    match kind {
        CallHierarchyKind::Function => 0,
        CallHierarchyKind::Method => 1,
        CallHierarchyKind::Constructor => 2,
    }
}

/// Convert a symbol ID to a CallHierarchyItem.
fn call_hierarchy_item_from_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<CallHierarchyItem> {
    let canonical_id = get_canonical_symbol(repository, revision, symbol_id);
    let ctx = query_context(repository, revision, canonical_id.module_id)?;
    let name = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        if symbol.form != SymbolForm::Function {
            return None;
        }
        resolve_symbol_name(repository, revision, canonical_id)?
    };

    // resolve the selection range at the symbol name
    let selection_range = get_symbol_definition_span(repository, revision, canonical_id)?;

    // resolve the full declaration range, fall back to the selection range
    let range =
        get_symbol_declaration_span(repository, revision, canonical_id).unwrap_or(selection_range);

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        file: selection_range.file,
        range,
        selection_range,
        symbol_id: canonical_id,
    })
}
