#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

pub(crate) use harness::HarnessValue;

/// Test harness context used by tests.
pub(crate) struct RuntimeHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Native runtime harness.
pub(crate) struct NativeRuntimeHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeRuntimeHarness {
    /// Create a new native runtime harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM runtime harness.
pub(crate) struct VmRuntimeHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmRuntimeHarness {
    /// Create a new VM runtime harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum RuntimeHarnessHandle {
    /// Native runtime harness.
    Native(NativeRuntimeHarness),
    /// VM runtime harness.
    Vm(VmRuntimeHarness),
}

impl RuntimeHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(RuntimeHarnessContext<'call>) -> R,
    {
        match self {
            RuntimeHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(RuntimeHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            RuntimeHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(RuntimeHarnessContext {
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
        F: for<'call> FnOnce(RuntimeHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("runtime harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&RuntimeHarnessHandle),
{
    let native = RuntimeHarnessHandle::Native(NativeRuntimeHarness::new());
    callback(&native);
    let vm = RuntimeHarnessHandle::Vm(VmRuntimeHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(RuntimeHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
