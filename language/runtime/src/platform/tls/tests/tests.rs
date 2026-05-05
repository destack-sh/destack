#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{ResourceId, resource};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

/// TLS harness context used by tests.
pub(crate) struct TlsHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Native TLS harness.
pub(crate) struct NativeTlsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTlsHarness {
    /// Create a new native TLS harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM TLS harness.
pub(crate) struct VmTlsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTlsHarness {
    /// Create a new VM TLS harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TlsHarnessHandle {
    /// Native TLS harness.
    Native(NativeTlsHarness),
    /// VM TLS harness.
    Vm(VmTlsHarness),
}

impl TlsHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
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
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(TlsHarnessContext {
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
        F: for<'call> FnOnce(TlsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("tls harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut TlsHarnessHandle),
{
    let mut native = TlsHarnessHandle::Native(NativeTlsHarness::new());
    callback(&mut native);
    let mut vm = TlsHarnessHandle::Vm(VmTlsHarness::new());
    callback(&mut vm);
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

/// Assert one runtime result failed with `invalidArgumentValue`.
pub(crate) fn assert_invalid_argument_value<T>(result: RuntimeResult<T>) -> RuntimeResult<()> {
    let error = match result {
        Ok(_) => panic!("operation should fail with invalidArgumentValue"),
        Err(error) => error,
    };

    let platform = error
        .platform_error()
        .expect("error should contain one platform error");
    assert_eq!(platform.code, PlatformErrorCode::InvalidArgumentValue);

    Ok(())
}

/// Return one placeholder TLS context handle for invalid-argument tests.
pub(crate) fn placeholder_context_handle() -> resource::TlsContextHandle {
    resource::TlsContextHandle(ResourceId(1))
}

/// Return one placeholder TLS session handle for invalid-argument tests.
pub(crate) fn placeholder_session_handle() -> resource::TlsSessionHandle {
    resource::TlsSessionHandle(ResourceId(1))
}

/// Return one placeholder socket handle for invalid-argument tests.
pub(crate) fn placeholder_socket_handle() -> resource::SocketHandle {
    resource::SocketHandle(ResourceId(1))
}
