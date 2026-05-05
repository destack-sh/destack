#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::assert_ok_or_expected_error;
use crate::tests::runtime::TestRuntime;

/// Test harness context used by tests.
pub(crate) struct DeviceHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native device harness.
pub(crate) struct NativeDeviceHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeDeviceHarness {
    /// Create a new native device harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM device harness.
pub(crate) struct VmDeviceHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmDeviceHarness {
    /// Create a new VM device harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum DeviceHarnessHandle {
    /// Native device harness.
    Native(NativeDeviceHarness),
    /// VM device harness.
    Vm(VmDeviceHarness),
}

impl DeviceHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(DeviceHarnessContext<'call>) -> R,
    {
        match self {
            DeviceHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(DeviceHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            DeviceHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(DeviceHarnessContext {
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
        F: for<'call> FnOnce(DeviceHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("device harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut DeviceHarnessHandle),
{
    let mut native = DeviceHarnessHandle::Native(NativeDeviceHarness::new());
    callback(&mut native);
    let mut vm = DeviceHarnessHandle::Vm(VmDeviceHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(DeviceHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Return the mutable VM context when the harness is running in VM mode.
pub(crate) fn vm_context_mut<'a>(
    context: &'a DeviceHarnessContext<'a>,
) -> Option<&'a mut vm::BindingContext<'a>> {
    context
        .vm_context
        .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
}

/// Run one callback against one native call context.
pub(crate) fn with_native_context<F>(callback: F)
where
    F: for<'call> FnOnce(&'call BindingCallContext) -> RuntimeResult<()>,
{
    let mut runtime = TestRuntime::deterministic_random();

    runtime
        .with_native_call_context(callback)
        .expect("device native test should succeed");
}

/// Assert one result is success or one expected not-supported outcome.
pub(crate) fn assert_supported_or_not_supported<T>(
    result: Result<T, Box<RuntimeError>>,
) -> RuntimeResult<Option<T>> {
    assert_ok_or_expected_error(result, &[PlatformErrorCode::NotSupported])
}
