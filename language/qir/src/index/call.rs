use destack_dir::GlobalSymbolId;
use destack_source::{FileId, ModuleId, Span};
use serde::{Deserialize, Serialize};

/// Call graph index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallIndex {
    /// The call entries ordered by callee symbol.
    by_callee: Vec<CallEntry>,
    /// The call entries ordered by caller symbol.
    by_caller: Vec<CallEntry>,
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

    /// Extend this index with another call index.
    pub fn extend(&mut self, index: Self) {
        self.by_callee.extend(index.by_callee);
        self.by_caller.extend(index.by_caller);
    }

    /// Sort and deduplicate this index.
    pub fn finish(&mut self) {
        self.by_callee.sort_by_key(call_callee_key);
        self.by_callee.dedup();

        self.by_caller.sort_by_key(call_caller_key);
        self.by_caller.dedup();
    }

    /// Return calls that target one callee symbol.
    pub fn to(&self, callee_symbol: GlobalSymbolId) -> Vec<CallEntry> {
        let range = self.callee_range(callee_symbol);

        self.by_callee[range].to_vec()
    }

    /// Return calls that originate from one caller symbol.
    pub fn from(&self, caller_symbol: GlobalSymbolId) -> Vec<CallEntry> {
        let range = self.caller_range(caller_symbol);

        self.by_caller[range].to_vec()
    }

    /// Return all call entries ordered by callee.
    pub fn entries(&self) -> &[CallEntry] {
        &self.by_callee
    }

    /// Return the stored range for one callee symbol.
    fn callee_range(&self, callee_symbol: GlobalSymbolId) -> std::ops::Range<usize> {
        let start = self
            .by_callee
            .partition_point(|entry| entry.callee_symbol < callee_symbol);
        let end = self.by_callee[start..]
            .partition_point(|entry| entry.callee_symbol == callee_symbol)
            + start;

        start..end
    }

    /// Return the stored range for one caller symbol.
    fn caller_range(&self, caller_symbol: GlobalSymbolId) -> std::ops::Range<usize> {
        let caller_symbol = Some(caller_symbol);
        let start = self
            .by_caller
            .partition_point(|entry| entry.caller_symbol < caller_symbol);
        let end = self.by_caller[start..]
            .partition_point(|entry| entry.caller_symbol == caller_symbol)
            + start;

        start..end
    }
}

/// Call graph edge entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEntry {
    /// The module containing the call.
    pub module_id: ModuleId,
    /// The containing function symbol when known.
    pub caller_symbol: Option<GlobalSymbolId>,
    /// The called function symbol.
    pub callee_symbol: GlobalSymbolId,
    /// The call source range.
    pub span: Span,
}

/// Return the stable callee ordering key for one call entry.
fn call_callee_key(
    entry: &CallEntry,
) -> (
    GlobalSymbolId,
    ModuleId,
    Option<GlobalSymbolId>,
    FileId,
    u32,
    u32,
) {
    (
        entry.callee_symbol,
        entry.module_id,
        entry.caller_symbol,
        entry.span.file,
        entry.span.start,
        entry.span.end,
    )
}

/// Return the stable caller ordering key for one call entry.
fn call_caller_key(
    entry: &CallEntry,
) -> (
    Option<GlobalSymbolId>,
    GlobalSymbolId,
    ModuleId,
    FileId,
    u32,
    u32,
) {
    (
        entry.caller_symbol,
        entry.callee_symbol,
        entry.module_id,
        entry.span.file,
        entry.span.start,
        entry.span.end,
    )
}
