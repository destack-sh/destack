#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct FfiHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native ffi harness.
pub(crate) struct NativeFfiHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeFfiHarness {
    /// Create a new native ffi harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM ffi harness.
pub(crate) struct VmFfiHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmFfiHarness {
    /// Create a new VM ffi harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum FfiHarnessHandle {
    /// Native ffi harness.
    Native(NativeFfiHarness),
    /// VM ffi harness.
    Vm(VmFfiHarness),
}

impl FfiHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(FfiHarnessContext<'call>) -> R,
    {
        match self {
            FfiHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(FfiHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            FfiHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(FfiHarnessContext {
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
        F: for<'call> FnOnce(FfiHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("ffi harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut FfiHarnessHandle),
{
    let mut native = FfiHarnessHandle::Native(NativeFfiHarness::new());
    callback(&mut native);
    let mut vm = FfiHarnessHandle::Vm(VmFfiHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(FfiHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
