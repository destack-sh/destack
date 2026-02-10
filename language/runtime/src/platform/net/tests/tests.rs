use crate::platform::{NativeSlice, NativeStringRef};
use crate::tests::runtime::TestRuntime;

/// Harness interface for network bindings.
pub(crate) trait NetHarness {
    /// Return the runtime backing this harness.
    fn runtime(&self) -> &TestRuntime;

    /// Run a native or VM call context around the callback.
    fn with_context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R;
}

/// Native network harness backed by native bindings.
pub(crate) struct NativeNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeNetHarness {
    /// Create a new native network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

impl NetHarness for NativeNetHarness {
    fn runtime(&self) -> &TestRuntime {
        &self.runtime
    }

    fn with_context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.runtime.with_native_call_context(|_| callback())
    }
}

/// VM network harness backed by VM bindings.
#[allow(dead_code)]
pub(crate) struct VmNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

#[allow(dead_code)]
impl VmNetHarness {
    /// Create a new VM network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

impl NetHarness for VmNetHarness {
    fn runtime(&self) -> &TestRuntime {
        &self.runtime
    }

    fn with_context<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R,
    {
        // NOTE #Incomplete: wire VM call contexts into the harness
        let _ = callback;
        panic!("vm net harness not wired yet")
    }
}

/// Run a test with the native network harness.
pub(crate) fn with_native_harness<F, R>(callback: F) -> R
where
    F: FnOnce(&NativeNetHarness) -> R,
{
    let harness = NativeNetHarness::new();
    callback(&harness)
}

/// Build a NativeSlice from a mutable byte buffer.
pub(crate) fn native_slice_mut(buffer: &mut [u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
    }
}

/// Build a NativeSlice from an immutable byte buffer.
pub(crate) fn native_slice(buffer: &[u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_ptr() as *mut u8,
        len: buffer.len() as u32,
    }
}

/// Build a NativeStringRef from a host string.
pub(crate) fn native_string(value: &str) -> NativeStringRef {
    NativeStringRef::from(value)
}
