use crate::platform::bindings::BindingDescriptor;

/// Native binding metadata for typed platform calls.
#[derive(Debug, Clone, Copy)]
pub struct NativeBinding {
    /// Binding descriptor for policy checks.
    pub spec: BindingDescriptor,
    /// Exported symbol name for native linkage.
    pub symbol: &'static str,
    /// Native function address for direct linkage.
    pub function: *const (),
}

impl NativeBinding {
    /// Create native binding metadata.
    pub const fn new(spec: BindingDescriptor, symbol: &'static str, function: *const ()) -> Self {
        Self {
            spec,
            symbol,
            function,
        }
    }
}

/// Set of native bindings for a platform domain.
#[derive(Debug, Clone, Copy)]
pub struct NativeBindingSet {
    /// Domain name for diagnostics and registration.
    pub name: &'static str,
    /// Bindings in this set.
    pub bindings: &'static [NativeBinding],
}
