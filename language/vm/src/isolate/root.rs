use destack_heap::{HeapReference, SharedHeapReference};

/// Visitor for VM roots discovered during one scan.
pub trait RootVisitor {
    /// Record one local heap root.
    fn push_heap(&mut self, reference: HeapReference);

    /// Record one shared heap root.
    fn push_shared(&mut self, reference: SharedHeapReference);
}

/// One collected VM root set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RootSet {
    /// The local heap roots.
    pub heap: Vec<HeapReference>,
    /// The shared heap roots.
    pub shared: Vec<SharedHeapReference>,
}

impl RootVisitor for RootSet {
    fn push_heap(&mut self, reference: HeapReference) {
        if reference.is_null() {
            return;
        }

        self.heap.push(reference);
    }

    fn push_shared(&mut self, reference: SharedHeapReference) {
        if reference.is_null() {
            return;
        }

        self.shared.push(reference);
    }
}
