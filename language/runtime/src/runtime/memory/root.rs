use destack_heap::ManagedReference;

/// Collection of GC roots for a safepoint.
#[derive(Debug, Default)]
pub struct RootSet {
    /// Managed root references for this collection.
    handles: Vec<ManagedReference>,
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    /// Push a new managed root reference.
    pub fn push(&mut self, handle: ManagedReference) {
        self.handles.push(handle);
    }

    /// Return the managed root references for this collection.
    pub fn handles(&self) -> &[ManagedReference] {
        self.handles.as_slice()
    }
}

/// Visit GC roots for a safepoint collection.
pub trait RootVisitor: Send + Sync {
    /// Collect roots into the provided root set.
    fn collect_roots(&self, roots: &mut RootSet);
}
