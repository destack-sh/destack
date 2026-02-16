#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct CryptoHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native crypto harness.
pub(crate) struct NativeCryptoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeCryptoHarness {
    /// Create a new native crypto harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM crypto harness.
pub(crate) struct VmCryptoHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmCryptoHarness {
    /// Create a new VM crypto harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum CryptoHarnessHandle {
    /// Native crypto harness.
    Native(NativeCryptoHarness),
    /// VM crypto harness.
    Vm(VmCryptoHarness),
}

impl CryptoHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(CryptoHarnessContext<'call>) -> R,
    {
        match self {
            CryptoHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(CryptoHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            CryptoHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(CryptoHarnessContext {
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
        F: for<'call> FnOnce(CryptoHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("crypto harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&CryptoHarnessHandle),
{
    let native = CryptoHarnessHandle::Native(NativeCryptoHarness::new());
    callback(&native);
    let vm = CryptoHarnessHandle::Vm(VmCryptoHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(CryptoHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
