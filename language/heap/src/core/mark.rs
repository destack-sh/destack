use std::collections::VecDeque;

use crate::{HeapReference, SharedHeapReference};

/// One reference type that may appear in one trace queue.
pub(crate) trait TraceReference: Copy {
    /// Report whether this reference is null.
    fn is_null(self) -> bool;
}

impl TraceReference for HeapReference {
    fn is_null(self) -> bool {
        HeapReference::is_null(&self)
    }
}

impl TraceReference for SharedHeapReference {
    fn is_null(self) -> bool {
        SharedHeapReference::is_null(&self)
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

impl<R: TraceReference> TraceQueue<R> {
    /// Push one pending reference.
    pub(crate) fn push(&mut self, reference: R) {
        if !reference.is_null() {
            self.pending.push_back(reference);
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
