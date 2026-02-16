#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct DebugHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native debug harness.
pub(crate) struct NativeDebugHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeDebugHarness {
    /// Create a new native debug harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM debug harness.
pub(crate) struct VmDebugHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmDebugHarness {
    /// Create a new VM debug harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum DebugHarnessHandle {
    /// Native debug harness.
    Native(NativeDebugHarness),
    /// VM debug harness.
    Vm(VmDebugHarness),
}

impl DebugHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(DebugHarnessContext<'call>) -> R,
    {
        match self {
            DebugHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(DebugHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            DebugHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(DebugHarnessContext {
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
        F: for<'call> FnOnce(DebugHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("debug harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&DebugHarnessHandle),
{
    let native = DebugHarnessHandle::Native(NativeDebugHarness::new());
    callback(&native);
    let vm = DebugHarnessHandle::Vm(VmDebugHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(DebugHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
