#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct InputHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native input harness.
pub(crate) struct NativeInputHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeInputHarness {
    /// Create a new native input harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM input harness.
pub(crate) struct VmInputHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmInputHarness {
    /// Create a new VM input harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum InputHarnessHandle {
    /// Native input harness.
    Native(NativeInputHarness),
    /// VM input harness.
    Vm(VmInputHarness),
}

impl InputHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(InputHarnessContext<'call>) -> R,
    {
        match self {
            InputHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(InputHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            InputHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(InputHarnessContext {
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
        F: for<'call> FnOnce(InputHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("input harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&InputHarnessHandle),
{
    let native = InputHarnessHandle::Native(NativeInputHarness::new());
    callback(&native);
    let vm = InputHarnessHandle::Vm(VmInputHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(InputHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
