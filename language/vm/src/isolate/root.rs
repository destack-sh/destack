use destack_heap::{HeapReference, SharedHeapReference};

/// Sink for VM roots discovered during one scan.
pub trait RootSink {
    /// Record one local heap root.
    fn push_heap(&mut self, reference: HeapReference);

    /// Record one shared heap root.
    fn push_shared_heap(&mut self, reference: SharedHeapReference);
}

/// One collected VM root set used by VM tests.
#[cfg(test)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RootSet {
    /// The local heap roots.
    pub heap: Vec<HeapReference>,
    /// The shared heap roots.
    pub shared_heap: Vec<SharedHeapReference>,
}

#[cfg(test)]
impl RootSink for RootSet {
    fn push_heap(&mut self, reference: HeapReference) {
        if reference.is_null() {
            return;
        }

        self.heap.push(reference);
    }

    fn push_shared_heap(&mut self, reference: SharedHeapReference) {
        if reference.is_null() {
            return;
        }

        self.shared_heap.push(reference);
    }
}
