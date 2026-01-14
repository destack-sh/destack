use std::cell::Cell;
use std::ptr;

use destack_vm::Error;

use super::context::HostContext;
use super::error::{HostError, HostResult};
use crate::platform::bindings::{BindingDescriptor, BindingPolicy};

thread_local! {
    /// TLS slot for the current host call context.
    static HOST_CALL_CONTEXT: Cell<*const HostCallContext> = const { Cell::new(ptr::null()) };
}

/// FFI string reference for native bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HostStringRef {
    /// Pointer to UTF-8 bytes.
    pub data: *const u8,
    /// Length of the UTF-8 byte slice.
    pub len: u64,
}

// safety: points into HostContext-owned strings that stay immutable for the host lifetime
unsafe impl Send for HostStringRef {}
// safety: points into HostContext-owned strings that stay immutable for the host lifetime
unsafe impl Sync for HostStringRef {}

impl HostStringRef {
    /// View the reference as a UTF-8 string.
    pub unsafe fn as_str<'a>(self) -> HostResult<&'a str> {
        // map the raw bytes into a string
        let bytes = unsafe { std::slice::from_raw_parts(self.data, self.len as usize) };
        std::str::from_utf8(bytes).map_err(|_| Error::TypeMismatch {
            expected: "utf8 string".to_string(),
            actual: "invalid utf8".to_string(),
        })
    }
}

impl From<&str> for HostStringRef {
    /// Build a string reference from a string slice.
    fn from(value: &str) -> Self {
        Self {
            data: value.as_ptr(),
            len: value.len() as u64,
        }
    }
}

impl From<&String> for HostStringRef {
    /// Build a string reference from an owned string reference.
    fn from(value: &String) -> Self {
        Self::from(value.as_str())
    }
}

/// FFI slice of string references.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HostStringSlice {
    /// Pointer to string references.
    pub data: *const HostStringRef,
    /// Number of string references.
    pub len: u64,
}

// safety: points into HostContext-owned string references that stay immutable
unsafe impl Send for HostStringSlice {}
// safety: points into HostContext-owned string references that stay immutable
unsafe impl Sync for HostStringSlice {}

impl HostStringSlice {
    /// Build a slice from string references.
    pub fn from_slice(values: &[HostStringRef]) -> Self {
        let data = if values.is_empty() {
            ptr::null()
        } else {
            values.as_ptr()
        };

        Self {
            data,
            len: values.len() as u64,
        }
    }
}

/// Status code returned by native host bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostStatus {
    /// Status code, where zero indicates success.
    pub code: u32,
}

impl HostStatus {
    /// Successful status.
    pub const OK: Self = Self { code: 0 };

    /// Build an error status from a host error.
    pub fn from_error(error: HostError) -> Self {
        let code = u32::from(error.sub_code()).saturating_add(1);
        Self { code }
    }

    /// Convert a host result into a status.
    pub fn from_result<T>(result: HostResult<T>) -> Self {
        match result {
            Ok(_) => Self::OK,
            Err(error) => Self::from_error(error),
        }
    }
}

/// TLS payload for native host calls.
#[derive(Debug, Clone, Copy)]
pub struct HostCallContext {
    /// Host context for platform state.
    host: *const HostContext,
    /// Binding policy for host calls.
    policy: BindingPolicy,
}

impl HostCallContext {
    /// Create a host call context for TLS.
    pub fn new(host: &HostContext, policy: BindingPolicy) -> Self {
        Self {
            host: host as *const HostContext,
            policy,
        }
    }

    /// Borrow the host context.
    pub fn host(&self) -> &HostContext {
        // safety: the host context outlives the call
        unsafe { &*self.host }
    }

    /// Validate the policy against a binding descriptor.
    pub fn check_policy(&self, spec: BindingDescriptor) -> HostResult<()> {
        self.policy.check(spec)
    }
}

/// Guard that restores the previous TLS host call context.
#[derive(Debug)]
pub struct HostCallGuard {
    /// Previous TLS context pointer.
    previous: *const HostCallContext,
}

impl Drop for HostCallGuard {
    /// Restore the previous host call context.
    fn drop(&mut self) {
        // restore the previous host context
        HOST_CALL_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Enter a host call context for native bindings.
pub fn enter_host_call_context(context: &HostCallContext) -> HostCallGuard {
    // swap the context pointer for this thread
    let previous = HOST_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const HostCallContext);
        previous
    });

    HostCallGuard { previous }
}

/// Access the current host call context for native bindings.
pub fn with_host_call_context<T>(
    f: impl FnOnce(&HostCallContext) -> HostResult<T>,
) -> HostResult<T> {
    // load the current TLS context
    let pointer = HOST_CALL_CONTEXT.with(|slot| slot.get());
    if pointer.is_null() {
        return Err(Error::ExternalCallForbidden {
            name: "host call context missing".to_string(),
        });
    }

    // safety: pointer is set by enter_host_call_context
    let context = unsafe { &*pointer };
    f(context)
}
