#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.rs"]
mod harness;

/// Test harness context used by tests.
pub(crate) struct ResourceHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native resource harness.
pub(crate) struct NativeResourceHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeResourceHarness {
    /// Create a new native resource harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM resource harness.
pub(crate) struct VmResourceHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmResourceHarness {
    /// Create a new VM resource harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ResourceHarnessHandle {
    /// Native resource harness.
    Native(NativeResourceHarness),
    /// VM resource harness.
    Vm(VmResourceHarness),
}

impl ResourceHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(ResourceHarnessContext<'call>) -> R,
    {
        match self {
            ResourceHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ResourceHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ResourceHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(ResourceHarnessContext {
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
        F: for<'call> FnOnce(ResourceHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("resource harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&ResourceHarnessHandle),
{
    let native = ResourceHarnessHandle::Native(NativeResourceHarness::new());
    callback(&native);
    let vm = ResourceHarnessHandle::Vm(VmResourceHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ResourceHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Assert one result failed with one exact platform error code.
pub(crate) fn assert_platform_error_code<T>(
    result: RuntimeResult<T>,
    expected: PlatformErrorCode,
) -> RuntimeResult<()> {
    let error = match result {
        Ok(_) => panic!("operation should fail"),
        Err(error) => error,
    };
    let platform = error
        .platform_error()
        .expect("error should contain one platform error");
    assert_eq!(platform.code, expected);

    Ok(())
}
