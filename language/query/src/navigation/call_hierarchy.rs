use std::collections::HashMap;

use destack_dir::{GlobalSymbolId, SymbolKind};
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{
    ModuleQueryContext, QueryPosition, QueryTarget, WorkspaceQueryContext,
    call_candidates_for_callee, call_candidates_for_caller,
};
use crate::dir::{find_symbol_at_offset, symbol_declaration_span, symbol_definition_span};
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
    /// The target source and resolved identity.
    pub target: QueryTarget,
}

/// Kind of call hierarchy item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallHierarchyKind {
    /// A function.
    Function,
    /// A method.
    Method,
    /// A constructor.
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

/// Request the call hierarchy item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyItemRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for call hierarchy item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyItemResponse {
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

/// Return a call hierarchy item at the given position.
pub fn call_hierarchy_item(ctx: &ModuleQueryContext<'_>, offset: u32) -> Option<CallHierarchyItem> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(ctx, offset)?;

    call_hierarchy_item_from_symbol(ctx, symbol_at.symbol_id)
}

/// Get incoming calls to a call hierarchy item.
///
/// "Who calls this function?"
pub fn incoming_calls(
    ctx: &WorkspaceQueryContext<'_>,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyIncomingCall> {
    let Some(symbol_id) = item.target.symbol_id else {
        return Vec::new();
    };
    let profile_id = item.target.module.profile_id;
    let Some(module_ctx) = ctx.module_context(symbol_id.module_id, profile_id) else {
        return Vec::new();
    };
    let canonical_id = module_ctx.canonical_symbol(symbol_id);
    let mut incoming_by_caller: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
    for entry in call_candidates_for_callee(ctx, canonical_id) {
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
        if let Some(caller_item) = call_hierarchy_item_from_symbol(&module_ctx, caller_symbol) {
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
    ctx: &WorkspaceQueryContext<'_>,
    item: &CallHierarchyItem,
) -> Vec<CallHierarchyOutgoingCall> {
    let Some(symbol_id) = item.target.symbol_id else {
        return Vec::new();
    };
    let profile_id = item.target.module.profile_id;
    let Some(module_ctx) = ctx.module_context(symbol_id.module_id, profile_id) else {
        return Vec::new();
    };
    let canonical_id = module_ctx.canonical_symbol(symbol_id);
    let mut calls_with_spans: HashMap<GlobalSymbolId, Vec<Span>> = HashMap::new();
    for entry in call_candidates_for_caller(ctx, canonical_id) {
        let callee_symbol = module_ctx.canonical_symbol(entry.callee_symbol);

        calls_with_spans
            .entry(callee_symbol)
            .or_default()
            .push(entry.span);
    }

    // convert to outgoing calls
    let mut outgoing = Vec::new();
    for (target_symbol_id, call_spans) in calls_with_spans {
        // only include function calls
        let Some(target_ctx) = module_ctx.module_context(target_symbol_id.module_id) else {
            continue;
        };
        let is_function = {
            let target_symbols = target_ctx.dir().symbols();
            let target_symbol = target_symbols.get_symbol(target_symbol_id.local_id);
            target_symbol.kind == SymbolKind::Function
        };
        if !is_function {
            continue;
        }

        if let Some(target_item) = call_hierarchy_item_from_symbol(&target_ctx, target_symbol_id) {
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
    let selection_span = item.target.selection_span.unwrap_or(item.target.span);

    (
        item.target.span.file.0,
        item.target.span.start,
        item.target.span.end,
        selection_span.start,
        selection_span.end,
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
    ctx: &ModuleQueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<CallHierarchyItem> {
    let canonical_id = ctx.canonical_symbol(symbol_id);
    let canonical_ctx = ctx.module_context(canonical_id.module_id)?;
    let name = {
        let symbols = canonical_ctx.dir().symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);
        if symbol.kind != SymbolKind::Function {
            return None;
        }
        canonical_ctx.symbol_name(canonical_id)?
    };

    // resolve the selection range at the symbol name
    let selection_range = symbol_definition_span(&canonical_ctx, canonical_id)?;

    // use the selection range when no declaration range is recorded
    let range = symbol_declaration_span(&canonical_ctx, canonical_id).unwrap_or(selection_range);

    let target = QueryTarget::span(canonical_ctx.query_module(), range)
        .with_selection_span(selection_range)
        .with_symbol(canonical_id);

    Some(CallHierarchyItem {
        name,
        kind: CallHierarchyKind::Function,
        detail: None,
        target,
    })
}
