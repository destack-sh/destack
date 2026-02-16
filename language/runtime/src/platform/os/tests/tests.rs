#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct OsHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native os harness.
pub(crate) struct NativeOsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeOsHarness {
    /// Create a new native os harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM os harness.
pub(crate) struct VmOsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmOsHarness {
    /// Create a new VM os harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum OsHarnessHandle {
    /// Native os harness.
    Native(NativeOsHarness),
    /// VM os harness.
    Vm(VmOsHarness),
}

impl OsHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(OsHarnessContext<'call>) -> R,
    {
        match self {
            OsHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(OsHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            OsHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(OsHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&self, callback: F)
    where
        F: for<'call> FnOnce(OsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("os harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&OsHarnessHandle),
{
    let native = OsHarnessHandle::Native(NativeOsHarness::new());
    callback(&native);
    let vm = OsHarnessHandle::Vm(VmOsHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(OsHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
