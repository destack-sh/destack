use destack_vm::Isolate;

/// VM entrypoint bridge for isolate orchestration.
#[derive(Debug)]
pub struct VmBridge {
    /// VM isolate managed by this bridge.
    isolate: Isolate,
}

impl VmBridge {
    /// Create a new bridge for a VM isolate.
    pub fn new(isolate: Isolate) -> Self {
        // wrap the isolate for runtime orchestration
        Self { isolate }
    }

    /// Get a reference to the VM isolate.
    pub fn isolate(&self) -> &Isolate {
        &self.isolate
    }

    /// Get a mutable reference to the VM isolate.
    pub fn isolate_mut(&mut self) -> &mut Isolate {
        &mut self.isolate
    }

    /// Consume the bridge and return the VM isolate.
    pub fn into_isolate(self) -> Isolate {
        self.isolate
    }
}
