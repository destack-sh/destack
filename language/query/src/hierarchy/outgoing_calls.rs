use std::collections::hash_map::Entry;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::Span;

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

/// An outgoing calls request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsRequest {
    /// The call item to expand.
    pub item: CallItem,
}

/// An outgoing calls response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OutgoingCallsResponse {
    /// Outgoing calls.
    pub calls: Vec<OutgoingCall>,
}

impl ProgramQueryContext<'_> {
    /// Return outgoing calls from one call item.
    pub fn outgoing_calls(
        &self,
        request: OutgoingCallsRequest,
    ) -> QueryResult<OutgoingCallsResponse> {
        let item = request.item;
        let symbol_id = item.symbol_id;
        let target_id = self
            .symbol_target(symbol_id)?
            .ok_or(QueryError::invalid(format!(
                "call hierarchy symbol: {symbol_id:?}"
            )))?;
        let mut callees: FxHashMap<dir::GlobalSymbolId, (CallItem, Vec<dir::CallEntry>)> =
            FxHashMap::default();

        // collect declaration backed callees and their call sites
        for entry in self.caller_calls(target_id)? {
            let module = self.module(entry.source.module_id)?;
            let Some(item) = CallItem::from_entry(&module, self, entry)? else {
                continue;
            };
            let callee = item.symbol_id;
            match callees.entry(callee) {
                Entry::Vacant(vacant) => {
                    vacant.insert((item, vec![entry]));
                }
                Entry::Occupied(mut occupied) if occupied.get().0 == item => {
                    occupied.get_mut().1.push(entry);
                }
                Entry::Occupied(_) => {
                    return Err(QueryError::conflict(format!(
                        "outgoing call item: {callee:?}"
                    )));
                }
            }
        }

        // collect indexed callees
        let mut calls = Vec::new();
        for (_, (to, entries)) in callees {
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

        Ok(OutgoingCallsResponse { calls })
    }
}
