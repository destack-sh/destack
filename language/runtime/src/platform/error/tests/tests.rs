#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct ErrorHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native error harness.
pub(crate) struct NativeErrorHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeErrorHarness {
    /// Create a new native error harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM error harness.
pub(crate) struct VmErrorHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmErrorHarness {
    /// Create a new VM error harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ErrorHarnessHandle {
    /// Native error harness.
    Native(NativeErrorHarness),
    /// VM error harness.
    Vm(VmErrorHarness),
}

impl ErrorHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(ErrorHarnessContext<'call>) -> R,
    {
        match self {
            ErrorHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ErrorHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ErrorHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(ErrorHarnessContext {
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
        F: for<'call> FnOnce(ErrorHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("error harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&ErrorHarnessHandle),
{
    let native = ErrorHarnessHandle::Native(NativeErrorHarness::new());
    callback(&native);
    let vm = ErrorHarnessHandle::Vm(VmErrorHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ErrorHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
