use destack_heap::{HeapReference, Root, RootSink, SharedHeapReference};

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
    fn push(&mut self, root: Root) {
        match root {
            Root::HeapReference(reference) if !reference.is_null() => {
                self.heap.push(reference);
            }
            Root::SharedHeapReference(reference) if !reference.is_null() => {
                self.shared_heap.push(reference);
            }
            _ => {}
        }
    }
}
