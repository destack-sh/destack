#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct RandomHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native random harness.
pub(crate) struct NativeRandomHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeRandomHarness {
    /// Create a new native random harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM random harness.
pub(crate) struct VmRandomHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmRandomHarness {
    /// Create a new VM random harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum RandomHarnessHandle {
    /// Native random harness.
    Native(NativeRandomHarness),
    /// VM random harness.
    Vm(VmRandomHarness),
}

impl RandomHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(RandomHarnessContext<'call>) -> R,
    {
        match self {
            RandomHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(RandomHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            RandomHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(RandomHarnessContext {
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
        F: for<'call> FnOnce(RandomHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("random harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&RandomHarnessHandle),
{
    let native = RandomHarnessHandle::Native(NativeRandomHarness::new());
    callback(&native);
    let vm = RandomHarnessHandle::Vm(VmRandomHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(RandomHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
