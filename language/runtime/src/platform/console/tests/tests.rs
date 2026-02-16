#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct ConsoleHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native console harness.
pub(crate) struct NativeConsoleHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeConsoleHarness {
    /// Create a new native console harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM console harness.
pub(crate) struct VmConsoleHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmConsoleHarness {
    /// Create a new VM console harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ConsoleHarnessHandle {
    /// Native console harness.
    Native(NativeConsoleHarness),
    /// VM console harness.
    Vm(VmConsoleHarness),
}

impl ConsoleHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(ConsoleHarnessContext<'call>) -> R,
    {
        match self {
            ConsoleHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ConsoleHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ConsoleHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(ConsoleHarnessContext {
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
        F: for<'call> FnOnce(ConsoleHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("console harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&ConsoleHarnessHandle),
{
    let native = ConsoleHarnessHandle::Native(NativeConsoleHarness::new());
    callback(&native);
    let vm = ConsoleHarnessHandle::Vm(VmConsoleHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ConsoleHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
