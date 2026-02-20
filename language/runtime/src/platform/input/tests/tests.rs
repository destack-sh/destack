#![cfg_attr(any(windows, target_os = "macos"), allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

#[path = "harness.rs"]
mod harness;
#[cfg(any(windows, target_os = "macos"))]
pub(crate) use harness::InputKeyboardStateRecord;
pub(crate) use harness::{InputDeviceRecord, InputEventRecord, InputMonitorEventRecord};

/// Test harness context used by tests.
pub(crate) struct InputHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native input harness.
pub(crate) struct NativeInputHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeInputHarness {
    /// Create a new native input harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM input harness.
pub(crate) struct VmInputHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmInputHarness {
    /// Create a new VM input harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum InputHarnessHandle {
    /// Native input harness.
    Native(NativeInputHarness),
    /// VM input harness.
    Vm(VmInputHarness),
}

impl InputHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(InputHarnessContext<'call>) -> R,
    {
        match self {
            InputHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(InputHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            InputHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(InputHarnessContext {
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
        F: for<'call> FnOnce(InputHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("input harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&InputHarnessHandle),
{
    let native = InputHarnessHandle::Native(NativeInputHarness::new());
    callback(&native);
    let vm = InputHarnessHandle::Vm(VmInputHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(InputHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Extract one platform error code from one failed result.
pub(crate) fn error_code<T>(result: RuntimeResult<T>) -> RuntimeResult<PlatformErrorCode> {
    match result {
        Ok(_) => Err(
            RuntimeError::from(PlatformError::invalid_argument("operation should fail")).boxed(),
        ),
        Err(error) => {
            let platform = error.platform_error().ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument(
                    "missing platform error payload",
                ))
                .boxed()
            })?;
            Ok(platform.code)
        }
    }
}

/// Assert one failed result with one exact platform error code.
pub(crate) fn assert_platform_error_code<T>(
    result: RuntimeResult<T>,
    expected: PlatformErrorCode,
) -> RuntimeResult<()> {
    let actual = error_code(result)?;
    assert_eq!(actual, expected);
    Ok(())
}

/// Assert one result is ok or fails with one expected platform error code.
pub(crate) fn assert_ok_or_expected_error<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            let Some(code) = error.platform_error().map(|platform| platform.code) else {
                return Err(error);
            };

            if expected.contains(&code) {
                return Ok(None);
            }

            Err(error)
        }
    }
}
