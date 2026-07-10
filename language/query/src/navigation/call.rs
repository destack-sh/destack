use std::collections::HashMap;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::source::sort_and_dedup_spans;
use crate::{ModuleQueryContext, Position, ProgramQueryContext, Target};

/// One callable item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItem {
    /// The name of the item (function/method name).
    pub name: String,
    /// The kind of item.
    pub kind: CallItemKind,
    /// Detail (e.g., signature).
    pub detail: Option<String>,
    /// The target source and resolved identity.
    pub target: Target,
}

/// Kind of callable item.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum CallItemKind {
    /// A function.
    Function,
    /// A method.
    Method,
    /// A constructor.
    Constructor,
}

/// An incoming call (who calls this function).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCall {
    /// The item that contains the call sites.
    pub from: CallItem,
    /// The ranges of the actual call expressions within `from`.
    pub from_ranges: Vec<Span>,
}

/// An outgoing call (what does this function call).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCall {
    /// The item being called.
    pub to: CallItem,
    /// The ranges of the call expressions to `to`.
    pub from_ranges: Vec<Span>,
}

/// Request the call item at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for call item queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CallItemResponse {
    /// Call item, if available.
    pub item: Option<CallItem>,
}

/// Request incoming calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCallsRequest {
    /// The call item to expand.
    pub item: CallItem,
}

/// Response payload for incoming calls queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCallsResponse {
    /// Incoming calls.
    pub calls: Vec<IncomingCall>,
}

/// Request outgoing calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsRequest {
    /// The call item to expand.
    pub item: CallItem,
}

/// Response payload for outgoing calls queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsResponse {
    /// Outgoing calls.
    pub calls: Vec<OutgoingCall>,
}

impl ModuleQueryContext<'_> {
    /// Return a call item at one offset.
    pub fn call_item(&self, offset: u32) -> Option<CallItem> {
        let symbol_at = self.find_symbol_at_offset(offset)?;

        self.call_item_from_symbol(symbol_at.symbol_id)
    }

    /// Convert one callable symbol into a call item.
    pub(crate) fn call_item_from_symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<CallItem> {
        let canonical_id = self.canonical_symbol(symbol_id);
        let canonical_module = self.module_context(canonical_id.module_id);
        let (name, kind) = {
            let symbols = canonical_module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            let kind = canonical_module.call_item_kind(canonical_id, symbol)?;
            let name = canonical_module.symbol_name(canonical_id)?;

            (name, kind)
        };

        // resolve source ranges around the declaration name
        let selection_range = canonical_module.symbol_definition_span(canonical_id)?;
        let range = canonical_module.symbol_declaration_span(canonical_id)?;

        let target = Target::new(canonical_module.module(), range)
            .with_selection_span(selection_range)
            .with_symbol_id(canonical_id);

        Some(CallItem {
            name,
            kind,
            detail: None,
            target,
        })
    }
}

impl ProgramQueryContext<'_> {
    /// Return incoming calls to one call item.
    pub fn incoming_calls(&self, item: &CallItem) -> Vec<IncomingCall> {
        let Some(symbol_id) = item.target.symbol_id else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let module = self.module_context(symbol_id.module_id, profile_id);
        let canonical_id = module.canonical_symbol(symbol_id);
        let mut incoming_by_caller: HashMap<dir::GlobalSymbolId, Vec<Span>> = HashMap::new();

        // collect call sites grouped by caller
        for entry in self.callee_calls(canonical_id) {
            let Some(caller_symbol) = entry.caller else {
                continue;
            };

            incoming_by_caller
                .entry(caller_symbol)
                .or_default()
                .push(entry.span);
        }

        let mut incoming = Vec::new();

        // build caller items
        for (caller_symbol, call_spans) in incoming_by_caller {
            if let Some(caller_item) = module.call_item_from_symbol(caller_symbol) {
                incoming.push(IncomingCall {
                    from: caller_item,
                    from_ranges: call_spans,
                });
            }
        }

        // sort call ranges and incoming callers for stable protocol output
        for call in &mut incoming {
            sort_and_dedup_spans(&mut call.from_ranges);
        }

        incoming.sort_by(|left, right| left.from.order().cmp(&right.from.order()));

        incoming
    }

    /// Return outgoing calls from one call item.
    pub fn outgoing_calls(&self, item: &CallItem) -> Vec<OutgoingCall> {
        let Some(symbol_id) = item.target.symbol_id else {
            return Vec::new();
        };
        let profile_id = item.target.module.profile_id;
        let module = self.module_context(symbol_id.module_id, profile_id);
        let canonical_id = module.canonical_symbol(symbol_id);
        let mut calls_with_spans: HashMap<dir::GlobalSymbolId, Vec<Span>> = HashMap::new();

        // collect call sites grouped by callee
        for entry in self.caller_calls(canonical_id) {
            let callee_symbol = module.canonical_symbol(entry.callee);

            calls_with_spans
                .entry(callee_symbol)
                .or_default()
                .push(entry.span);
        }

        let mut outgoing = Vec::new();

        // build callee items
        for (target_symbol_id, call_spans) in calls_with_spans {
            let target_module = module.module_context(target_symbol_id.module_id);
            if let Some(target_item) = target_module.call_item_from_symbol(target_symbol_id) {
                outgoing.push(OutgoingCall {
                    to: target_item,
                    from_ranges: call_spans,
                });
            }
        }

        // sort call ranges and outgoing callees for stable protocol output
        for call in &mut outgoing {
            sort_and_dedup_spans(&mut call.from_ranges);
        }

        outgoing.sort_by(|left, right| left.to.order().cmp(&right.to.order()));

        outgoing
    }
}

impl CallItem {
    /// Return the stable protocol ordering for this item.
    fn order(&self) -> CallItemOrder<'_> {
        let selection_span = self.target.selection_span.unwrap_or(self.target.span);

        CallItemOrder {
            file: self.target.span.file.0,
            start: self.target.span.start,
            end: self.target.span.end,
            selection_start: selection_span.start,
            selection_end: selection_span.end,
            kind: self.kind,
            name: self.name.as_str(),
        }
    }
}

/// Stable protocol ordering for one call item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CallItemOrder<'a> {
    /// The source file id.
    file: u64,
    /// The item start offset.
    start: u32,
    /// The item end offset.
    end: u32,
    /// The selection start offset.
    selection_start: u32,
    /// The selection end offset.
    selection_end: u32,
    /// The item kind.
    kind: CallItemKind,
    /// The rendered item name.
    name: &'a str,
}

impl ModuleQueryContext<'_> {
    /// Return the call item kind for one checked symbol.
    fn call_item_kind(
        &self,
        symbol_id: dir::GlobalSymbolId,
        symbol: &dir::Symbol,
    ) -> Option<CallItemKind> {
        if matches!(
            symbol.kind,
            dir::SymbolKind::Class | dir::SymbolKind::EnumField | dir::SymbolKind::Newtype
        ) {
            return Some(CallItemKind::Constructor);
        }

        if symbol.kind != dir::SymbolKind::Function {
            return None;
        }

        // classify member functions by their declaration slot
        if let Some(kind) = self.member_call_item_kind(symbol_id, symbol) {
            return Some(kind);
        }

        Some(CallItemKind::Function)
    }

    /// Return the call item kind for one member function symbol.
    fn member_call_item_kind(
        &self,
        symbol_id: dir::GlobalSymbolId,
        symbol: &dir::Symbol,
    ) -> Option<CallItemKind> {
        let declaration = symbol.declaration?;
        if declaration.module_id != symbol_id.module_id
            || declaration.local_id.ty != dir::NodeType::Member
        {
            return None;
        }

        let member_id = declaration
            .local_id
            .try_into_typed::<dir::Member>()
            .unwrap_or_else(|_| panic!("function member symbol has non-member declaration"));
        let member = self.view().get(member_id);

        match member.slot()? {
            dir::MemberSlot::Constructor | dir::MemberSlot::New => Some(CallItemKind::Constructor),
            dir::MemberSlot::Call | dir::MemberSlot::Key(_) => Some(CallItemKind::Method),
        }
    }
}
