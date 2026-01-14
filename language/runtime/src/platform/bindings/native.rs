use super::BindingDescriptor;

/// Native binding metadata for typed host calls.
#[derive(Debug, Clone, Copy)]
pub struct NativeBinding {
    /// Binding descriptor for policy checks.
    pub spec: BindingDescriptor,
    /// Exported symbol name for native linkage.
    pub symbol: &'static str,
}

impl NativeBinding {
    /// Create native binding metadata.
    pub const fn new(spec: BindingDescriptor, symbol: &'static str) -> Self {
        Self { spec, symbol }
    }
}

/// Set of native bindings for a host domain.
#[derive(Debug, Clone, Copy)]
pub struct NativeBindingSet {
    /// Domain name for diagnostics and registration.
    pub name: &'static str,
    /// Bindings in this set.
    pub bindings: &'static [NativeBinding],
}
