use crate::diagnostic::{RuntimeResult, RuntimeStatus};
use crate::runtime::binding::BindingDescriptor;
use crate::runtime::{BindingCallContext, with_binding_call_context};

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

// safety: native bindings are immutable metadata and the function pointer targets static code
unsafe impl Send for NativeBinding {}

// safety: native bindings are immutable metadata and the function pointer targets static code
unsafe impl Sync for NativeBinding {}

/// Set of native bindings for a platform domain.
#[derive(Debug, Clone, Copy)]
pub struct NativeBindingSet {
    /// Domain name for diagnostics and registration.
    pub name: &'static str,
    /// Bindings in this set.
    pub bindings: &'static [NativeBinding],
}

/// Execute a native binding call with runtime context handling.
#[inline]
pub fn native_call<T>(f: impl FnOnce(&BindingCallContext) -> RuntimeResult<T>) -> RuntimeStatus {
    match with_binding_call_context(|context| {
        let result = f(context);
        Ok(RuntimeStatus::from_result(result, Some(context)))
    }) {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}
