#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.generated.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct SecurityHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native security harness.
pub(crate) struct NativeSecurityHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeSecurityHarness {
    /// Create a new native security harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM security harness.
pub(crate) struct VmSecurityHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmSecurityHarness {
    /// Create a new VM security harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum SecurityHarnessHandle {
    /// Native security harness.
    Native(NativeSecurityHarness),
    /// VM security harness.
    Vm(VmSecurityHarness),
}

impl SecurityHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(SecurityHarnessContext<'call>) -> R,
    {
        match self {
            SecurityHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(SecurityHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            SecurityHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(SecurityHarnessContext {
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
        F: for<'call> FnOnce(SecurityHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("security harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&SecurityHarnessHandle),
{
    let native = SecurityHarnessHandle::Native(NativeSecurityHarness::new());
    callback(&native);
    let vm = SecurityHarnessHandle::Vm(VmSecurityHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(SecurityHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
