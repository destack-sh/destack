#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.rs"]
mod harness;

#[allow(unused_imports)]
pub(crate) use harness::*;

/// Test harness context used by tests.
pub(crate) struct IpcHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native ipc harness.
pub(crate) struct NativeIpcHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeIpcHarness {
    /// Create a new native ipc harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM ipc harness.
pub(crate) struct VmIpcHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmIpcHarness {
    /// Create a new VM ipc harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum IpcHarnessHandle {
    /// Native ipc harness.
    Native(NativeIpcHarness),
    /// VM ipc harness.
    Vm(VmIpcHarness),
}

impl IpcHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(IpcHarnessContext<'call>) -> R,
    {
        match self {
            IpcHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(IpcHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            IpcHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(IpcHarnessContext {
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
        F: for<'call> FnOnce(IpcHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("ipc harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut IpcHarnessHandle),
{
    let mut native = IpcHarnessHandle::Native(NativeIpcHarness::new());
    callback(&mut native);
    let mut vm = IpcHarnessHandle::Vm(VmIpcHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(IpcHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
