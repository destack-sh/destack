use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::source::sort_and_dedup_spans;
use crate::{CallItem, ProgramQueryContext, QueryError, QueryResult};

/// One outgoing call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCall {
    /// The item being called.
    pub to: CallItem,
    /// The call expression ranges in the source item.
    pub from_ranges: Vec<Span>,
}

/// Request outgoing calls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsRequest {
    /// The call item to expand.
    pub item: CallItem,
}

/// Response payload for outgoing call queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsResponse {
    /// Outgoing calls.
    pub calls: Vec<OutgoingCall>,
}

impl ProgramQueryContext<'_> {
    /// Return outgoing calls from one call item.
    pub fn outgoing_calls(&self, item: &CallItem) -> QueryResult<Vec<OutgoingCall>> {
        let symbol_id = item.symbol_id;
        let canonical_id = self
            .canonical_symbol(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "call hierarchy symbol: {symbol_id:?}"
            )))?;
        let mut callees: FxHashMap<dir::GlobalSymbolId, Vec<dir::CallEntry>> = FxHashMap::default();

        // collect call sites grouped by their exact canonical callee
        for entry in self.caller_calls(canonical_id)? {
            let callee = self
                .canonical_symbol(entry.callee)?
                .ok_or(QueryError::invalid(format!(
                    "call hierarchy symbol: {:?}",
                    entry.callee
                )))?;
            callees.entry(callee).or_default().push(entry);
        }

        // transcribe exact indexed callees
        let mut calls = Vec::new();
        for (callee, entries) in callees {
            let first_source = entries
                .first()
                .map(|entry| entry.source)
                .ok_or(QueryError::missing(format!("outgoing call: {callee:?}")))?;
            let module = self.module(first_source.module_id)?;
            let to = CallItem::from_call(&module, self, first_source.local_id, callee)?.ok_or(
                QueryError::invalid(format!("call hierarchy symbol: {callee:?}")),
            )?;

            // require one stable item across every grouped call selection
            for entry in &entries[1..] {
                let source = entry.source;
                let module = self.module(source.module_id)?;
                let selected = CallItem::from_call(&module, self, source.local_id, callee)?.ok_or(
                    QueryError::invalid(format!("call hierarchy symbol: {callee:?}")),
                )?;
                if selected != to {
                    return Err(QueryError::conflict(format!(
                        "outgoing call item: {callee:?}"
                    )));
                }
            }

            let mut ranges = entries.iter().map(|entry| entry.span).collect::<Vec<_>>();
            sort_and_dedup_spans(&mut ranges);

            calls.push(OutgoingCall {
                to,
                from_ranges: ranges,
            });
        }

        // order distinct call sites by source and shared sites by target
        calls.sort_by(|left, right| {
            let left_range = left.from_ranges[0];
            let right_range = right.from_ranges[0];
            let left_range = (left_range.file, left_range.start, left_range.end);
            let right_range = (right_range.file, right_range.start, right_range.end);

            left_range
                .cmp(&right_range)
                .then_with(|| left.to.order().cmp(&right.to.order()))
        });

        Ok(calls)
    }
}
