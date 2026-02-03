/// Entry point handle for VM engines.
#[derive(Debug, Clone)]
pub struct VmEntry {
    /// Fully qualified entry name.
    pub name: String,
}

impl VmEntry {
    /// Create a VM entry point by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Entry point handle for native engines.
#[derive(Debug, Clone)]
pub struct NativeEntry {
    /// Fully qualified entry name.
    pub name: String,
    /// Raw function symbol pointer.
    pub symbol: *const (),
}

impl NativeEntry {
    /// Create a native entry point by symbol.
    pub fn new(name: impl Into<String>, symbol: *const ()) -> Self {
        Self {
            name: name.into(),
            symbol,
        }
    }
}
