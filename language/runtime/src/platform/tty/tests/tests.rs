#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct TtyHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native tty harness.
pub(crate) struct NativeTtyHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTtyHarness {
    /// Create a new native tty harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM tty harness.
pub(crate) struct VmTtyHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTtyHarness {
    /// Create a new VM tty harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TtyHarnessHandle {
    /// Native tty harness.
    Native(NativeTtyHarness),
    /// VM tty harness.
    Vm(VmTtyHarness),
}

impl TtyHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(TtyHarnessContext<'call>) -> R,
    {
        match self {
            TtyHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(TtyHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            TtyHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(TtyHarnessContext {
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
        F: for<'call> FnOnce(TtyHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("tty harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&TtyHarnessHandle),
{
    let native = TtyHarnessHandle::Native(NativeTtyHarness::new());
    callback(&native);
    let vm = TtyHarnessHandle::Vm(VmTtyHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(TtyHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
