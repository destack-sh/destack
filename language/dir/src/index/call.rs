use crate::{Expression, GlobalNodeId, GlobalSymbolId, Postings};
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

/// Call graph index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CallIndex {
    /// The calls ordered by callee symbol.
    by_callee: Vec<CallEntry>,
    /// The calls ordered by caller symbol.
    by_caller: Vec<CallEntry>,
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
            by_callee: entries.clone(),
            by_caller: entries,
        };
        index.finish();

        index
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_callee.sort_by(CallEntry::compare_by_callee);
        self.by_callee.dedup();

        self.by_caller.sort_by(CallEntry::compare_by_caller);
        self.by_caller.dedup();
    }

    /// Iterate calls that target one callee symbol.
    pub fn callee_entries(&self, callee: GlobalSymbolId) -> impl Iterator<Item = &CallEntry> {
        let range = self.callee_range(callee);

        self.by_callee[range].iter()
    }

    /// Iterate calls that originate from one caller symbol.
    pub fn caller_entries(&self, caller: GlobalSymbolId) -> impl Iterator<Item = &CallEntry> {
        let range = self.caller_range(caller);

        self.by_caller[range].iter()
    }

    /// Return all calls ordered by callee.
    pub fn entries(&self) -> &[CallEntry] {
        &self.by_callee
    }

    /// Return the stored range for one callee symbol.
    fn callee_range(&self, callee: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_callee
            .partition_point(|entry| entry.callee < callee);
        let end = self.by_callee[start..].partition_point(|entry| entry.callee == callee) + start;

        start..end
    }

    /// Return the stored range for one caller symbol.
    fn caller_range(&self, caller: GlobalSymbolId) -> std::ops::Range<usize> {
        let caller = Some(caller);
        let start = self
            .by_caller
            .partition_point(|entry| entry.caller < caller);
        let end = self.by_caller[start..].partition_point(|entry| entry.caller == caller) + start;

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
