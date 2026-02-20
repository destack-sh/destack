use destack_heap::HeapHandle;

/// Root reference for GC tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RootRef {
    /// Managed heap root reference.
    Managed(HeapHandle),
    /// Native-managed object participating in the heap.
    Native(u64),
}

impl RootRef {
    /// Create a managed heap root reference.
    pub fn managed(handle: HeapHandle) -> Self {
        Self::Managed(handle)
    }

    /// Create a native heap root reference.
    pub fn native(handle: u64) -> Self {
        Self::Native(handle)
    }
}

/// Collection of GC roots for a safepoint.
#[derive(Debug, Default)]
pub struct RootSet {
    /// Root references for this collection.
    roots: Vec<RootRef>,
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    /// Push a new root reference.
    pub fn push(&mut self, root: RootRef) {
        self.roots.push(root);
    }

    /// Return the root references for this collection.
    pub fn roots(&self) -> &[RootRef] {
        self.roots.as_slice()
    }

    /// Collect managed heap handles from this root set.
    pub fn managed_handles(&self) -> Vec<HeapHandle> {
        let mut handles = Vec::new();
        for root in &self.roots {
            if let RootRef::Managed(handle) = root {
                handles.push(*handle);
            }
        }
        handles
    }
}

/// Visit GC roots for a safepoint collection.
pub trait RootVisitor: Send + Sync {
    /// Collect roots into the provided root set.
    fn collect_roots(&self, roots: &mut RootSet);
}
