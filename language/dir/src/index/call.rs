use crate::{Expression, GlobalNodeId, GlobalSymbolId, Postings};
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::Span;

/// Call graph index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallIndex {
    /// The calls ordered by callee symbol.
    entries: Vec<CallEntry>,
    /// Call ordinals ordered by caller symbol.
    by_caller: Vec<u32>,
}

/// Call postings by caller and callee symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct CallPostings {
    /// Caller symbol postings.
    pub callers: Postings<GlobalSymbolId>,
    /// Callee symbol postings.
    pub callees: Postings<GlobalSymbolId>,
}

impl CallIndex {
    /// Create a call index from entries.
    pub fn new(entries: Vec<CallEntry>) -> Self {
        let mut index = Self {
            entries,
            by_caller: Vec::new(),
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        // normalize calls in callee order
        self.entries.sort_by(CallEntry::compare_by_callee);
        self.entries.dedup();

        // index calls in caller order
        self.by_caller = (0..self.entries.len() as u32).collect();
        self.by_caller
            .retain(|ordinal| self.entries[*ordinal as usize].caller.is_some());
        self.by_caller.sort_by(|left, right| {
            self.entries[*left as usize].compare_by_caller(&self.entries[*right as usize])
        });
    }

    /// Iterate calls that target one callee symbol.
    pub fn callee_entries(&self, callee: GlobalSymbolId) -> impl Iterator<Item = &CallEntry> {
        let range = self.callee_range(callee);

        self.entries[range].iter()
    }

    /// Iterate calls that originate from one caller symbol.
    pub fn caller_entries(&self, caller: GlobalSymbolId) -> impl Iterator<Item = &CallEntry> {
        let range = self.caller_range(caller);

        self.by_caller[range]
            .iter()
            .map(|ordinal| &self.entries[*ordinal as usize])
    }

    /// Return all calls ordered by callee.
    pub fn entries(&self) -> &[CallEntry] {
        &self.entries
    }

    /// Return the stored range for one callee symbol.
    fn callee_range(&self, callee: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self.entries.partition_point(|entry| entry.callee < callee);
        let end = self.entries[start..].partition_point(|entry| entry.callee == callee) + start;

        start..end
    }

    /// Return the stored range for one caller symbol.
    fn caller_range(&self, caller: GlobalSymbolId) -> std::ops::Range<usize> {
        let caller = Some(caller);
        let start = self
            .by_caller
            .partition_point(|ordinal| self.entries[*ordinal as usize].caller < caller);
        let end = self.by_caller[start..]
            .partition_point(|ordinal| self.entries[*ordinal as usize].caller == caller)
            + start;

        start..end
    }
}

impl CallPostings {
    /// Build call postings from module index sections.
    pub fn build(indexes: &[&CallIndex]) -> Self {
        let callers = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .filter_map(move |entry| entry.caller.map(|caller| (caller, module)))
        }));
        let callees = Postings::from_pairs((0..indexes.len()).flat_map(|ordinal| {
            let module = ordinal as u32;

            indexes[ordinal]
                .entries()
                .iter()
                .map(move |entry| (entry.callee, module))
        }));

        Self { callers, callees }
    }

    /// Replace postings for one module call index.
    pub fn update(&mut self, module: u32, index: &CallIndex) {
        self.callers.replace(
            module,
            index.entries().iter().filter_map(|entry| entry.caller),
        );
        self.callees
            .replace(module, index.entries().iter().map(|entry| entry.callee));
    }
}

/// One call graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallEntry {
    /// The call-like expression node.
    pub source: GlobalNodeId<Expression>,
    /// The kind of call-like operation.
    pub kind: CallKind,
    /// The containing function symbol when known.
    pub caller: Option<GlobalSymbolId>,
    /// The called function symbol.
    pub callee: GlobalSymbolId,
    /// The call source range.
    pub span: Span,
}

impl CallEntry {
    /// Compare two calls in callee lookup order.
    fn compare_by_callee(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.callee,
            self.source.module_id,
            self.span.file,
            self.span.start,
            self.span.end,
            self.source.local_id.id,
            self.caller,
            self.kind,
        );
        let right = (
            other.callee,
            other.source.module_id,
            other.span.file,
            other.span.start,
            other.span.end,
            other.source.local_id.id,
            other.caller,
            other.kind,
        );

        left.cmp(&right)
    }

    /// Compare two calls in caller lookup order.
    fn compare_by_caller(&self, other: &Self) -> std::cmp::Ordering {
        let left = (
            self.caller,
            self.source.module_id,
            self.span.file,
            self.span.start,
            self.span.end,
            self.source.local_id.id,
            self.callee,
            self.kind,
        );
        let right = (
            other.caller,
            other.source.module_id,
            other.span.file,
            other.span.start,
            other.span.end,
            other.source.local_id.id,
            other.callee,
            other.kind,
        );

        left.cmp(&right)
    }
}

/// Kind of call-like operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum CallKind {
    /// Ordinary call expression.
    Call,
    /// Constructor call expression.
    Construct,
}
