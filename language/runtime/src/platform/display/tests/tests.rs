#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct DisplayHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native display harness.
pub(crate) struct NativeDisplayHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeDisplayHarness {
    /// Create a new native display harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM display harness.
pub(crate) struct VmDisplayHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmDisplayHarness {
    /// Create a new VM display harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum DisplayHarnessHandle {
    /// Native display harness.
    Native(NativeDisplayHarness),
    /// VM display harness.
    Vm(VmDisplayHarness),
}

impl DisplayHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(DisplayHarnessContext<'call>) -> R,
    {
        match self {
            DisplayHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(DisplayHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            DisplayHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(DisplayHarnessContext {
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
        F: for<'call> FnOnce(DisplayHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("display harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&DisplayHarnessHandle),
{
    let native = DisplayHarnessHandle::Native(NativeDisplayHarness::new());
    callback(&native);
    let vm = DisplayHarnessHandle::Vm(VmDisplayHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(DisplayHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
