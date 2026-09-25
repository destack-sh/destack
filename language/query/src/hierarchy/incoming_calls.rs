use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::Span;

use crate::source::sort_and_dedup_spans;
use crate::{CallItem, ProgramQueryContext, QueryError, QueryResult};

/// One incoming call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCall {
    /// The item that contains the call sites.
    pub from: CallItem,
    /// The call expression ranges within `from`.
    pub from_ranges: Vec<Span>,
}

/// An incoming calls request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCallsRequest {
    /// The call item to expand.
    pub item: CallItem,
}

/// An incoming calls response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IncomingCallsResponse {
    /// Incoming calls.
    pub calls: Vec<IncomingCall>,
}

impl ProgramQueryContext<'_> {
    /// Return incoming calls to one call item.
    pub fn incoming_calls(
        &self,
        request: IncomingCallsRequest,
    ) -> QueryResult<IncomingCallsResponse> {
        let item = request.item;
        let symbol_id = item.symbol_id;
        let target_id = self
            .symbol_target(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "call hierarchy symbol: {symbol_id:?}"
            )))?;
        let mut spans_by_caller: FxHashMap<dir::GlobalSymbolId, Vec<Span>> = FxHashMap::default();

        // collect call sites grouped by their exact caller
        for entry in self.callee_calls(target_id)? {
            let Some(caller) = entry.caller else {
                continue;
            };
            spans_by_caller.entry(caller).or_default().push(entry.span);
        }

        // collect indexed callers
        let mut calls = Vec::new();
        for (caller, mut ranges) in spans_by_caller {
            sort_and_dedup_spans(&mut ranges);
            let from = CallItem::from_symbol(self, caller)?.ok_or(QueryError::invalid(format!(
                "call hierarchy symbol: {caller:?}"
            )))?;
            calls.push(IncomingCall {
                from,
                from_ranges: ranges,
            });
        }

        calls.sort_by(|left, right| left.from.order().cmp(&right.from.order()));

        Ok(IncomingCallsResponse { calls })
    }
}
