#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct TlsHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native tls harness.
pub(crate) struct NativeTlsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTlsHarness {
    /// Create a new native tls harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM tls harness.
pub(crate) struct VmTlsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTlsHarness {
    /// Create a new VM tls harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TlsHarnessHandle {
    /// Native tls harness.
    Native(NativeTlsHarness),
    /// VM tls harness.
    Vm(VmTlsHarness),
}

impl TlsHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(TlsHarnessContext<'call>) -> R,
    {
        match self {
            TlsHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(TlsHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            TlsHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(TlsHarnessContext {
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
        F: for<'call> FnOnce(TlsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("tls harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&TlsHarnessHandle),
{
    let native = TlsHarnessHandle::Native(NativeTlsHarness::new());
    callback(&native);
    let vm = TlsHarnessHandle::Vm(VmTlsHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(TlsHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
