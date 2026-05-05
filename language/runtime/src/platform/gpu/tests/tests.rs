#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct GpuHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native gpu harness.
pub(crate) struct NativeGpuHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeGpuHarness {
    /// Create a new native gpu harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM gpu harness.
pub(crate) struct VmGpuHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmGpuHarness {
    /// Create a new VM gpu harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum GpuHarnessHandle {
    /// Native gpu harness.
    Native(NativeGpuHarness),
    /// VM gpu harness.
    Vm(VmGpuHarness),
}

impl GpuHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(GpuHarnessContext<'call>) -> R,
    {
        match self {
            GpuHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(GpuHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            GpuHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(GpuHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run one callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
    where
        F: for<'call> FnOnce(GpuHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("gpu harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut GpuHarnessHandle),
{
    let mut native = GpuHarnessHandle::Native(NativeGpuHarness::new());
    callback(&mut native);
    let mut vm = GpuHarnessHandle::Vm(VmGpuHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(GpuHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
