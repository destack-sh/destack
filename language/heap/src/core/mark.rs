use std::collections::{BTreeSet, VecDeque};

use crate::{HeapReference, SharedHeapReference};

/// One reference type with one stable address key.
pub(crate) trait ReferenceKey: Copy + Ord {
    /// Report whether this reference is null.
    fn is_null(self) -> bool;
}

impl ReferenceKey for HeapReference {
    fn is_null(self) -> bool {
        HeapReference::is_null(&self)
    }
}

impl ReferenceKey for SharedHeapReference {
    fn is_null(self) -> bool {
        SharedHeapReference::is_null(&self)
    }
}

/// Collector-owned mark set keyed by reference address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkSet<R> {
    /// The marked references in the current collection.
    marked: BTreeSet<R>,
}

impl<R> Default for MarkSet<R> {
    fn default() -> Self {
        Self {
            marked: BTreeSet::new(),
        }
    }
}

impl<R: ReferenceKey> MarkSet<R> {
    /// Start one fresh mark cycle.
    pub(crate) fn start_cycle(&mut self) {
        self.marked.clear();
    }

    /// Return whether this set contains the reference.
    pub(crate) fn contains(&self, reference: R) -> bool {
        if reference.is_null() {
            return false;
        }

        self.marked.contains(&reference)
    }

    /// Mark the reference and return whether this was the first mark.
    pub(crate) fn mark(&mut self, reference: R) -> bool {
        if reference.is_null() {
            return false;
        }

        self.marked.insert(reference)
    }
}

/// Trace queue for reachable references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraceQueue<R> {
    /// The pending references whose outgoing edges still need scanning.
    pending: VecDeque<R>,
}

impl<R> Default for TraceQueue<R> {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }
}

impl<R: ReferenceKey> TraceQueue<R> {
    /// Push one pending reference.
    pub(crate) fn push(&mut self, reference: R) {
        if !reference.is_null() {
            self.pending.push_back(reference);
        }
    }

    /// Extend the pending queue with more references.
    pub(crate) fn extend(&mut self, references: impl IntoIterator<Item = R>) {
        for reference in references {
            self.push(reference);
        }
    }

    /// Pop the next pending reference.
    pub(crate) fn pop(&mut self) -> Option<R> {
        self.pending.pop_front()
    }

    /// Return whether the queue is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Clear every pending reference.
    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}
