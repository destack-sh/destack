#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct MemoryHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native memory harness.
pub(crate) struct NativeMemoryHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeMemoryHarness {
    /// Create a new native memory harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM memory harness.
pub(crate) struct VmMemoryHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmMemoryHarness {
    /// Create a new VM memory harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum MemoryHarnessHandle {
    /// Native memory harness.
    Native(NativeMemoryHarness),
    /// VM memory harness.
    Vm(VmMemoryHarness),
}

impl MemoryHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(MemoryHarnessContext<'call>) -> R,
    {
        match self {
            MemoryHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(MemoryHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            MemoryHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(MemoryHarnessContext {
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
        F: for<'call> FnOnce(MemoryHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("memory harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&MemoryHarnessHandle),
{
    let native = MemoryHarnessHandle::Native(NativeMemoryHarness::new());
    callback(&native);
    let vm = MemoryHarnessHandle::Vm(VmMemoryHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(MemoryHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
