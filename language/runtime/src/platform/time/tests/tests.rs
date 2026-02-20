#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::time::{ClockMetadata, ClockMetadataVm};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

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

/// Assert one result failed with one code from the allowed set.
pub(crate) fn assert_platform_error_codes<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<()> {
    let error = match result {
        Ok(_) => panic!("operation should fail"),
        Err(error) => error,
    };

    let platform = error
        .platform_error()
        .expect("error should contain one platform error");
    assert!(
        expected.contains(&platform.code),
        "unexpected platform error code {:?}, allowed: {:?}",
        platform.code,
        expected
    );

    Ok(())
}

/// Normalize one harness clock info payload into the native value shape.
pub(crate) fn clock_metadata_from_value(
    value: harness::HarnessValue<ClockMetadata, ClockMetadataVm>,
) -> ClockMetadata {
    match value {
        harness::HarnessValue::Native(value) => value,
        harness::HarnessValue::Vm(value) => value,
    }
}

/// Test harness context used by tests.
pub(crate) struct TimeHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native time harness.
pub(crate) struct NativeTimeHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTimeHarness {
    /// Create a new native time harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM time harness.
pub(crate) struct VmTimeHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTimeHarness {
    /// Create a new VM time harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TimeHarnessHandle {
    /// Native time harness.
    Native(NativeTimeHarness),
    /// VM time harness.
    Vm(VmTimeHarness),
}

impl TimeHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(TimeHarnessContext<'call>) -> R,
    {
        match self {
            TimeHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(TimeHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            TimeHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(TimeHarnessContext {
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
        F: for<'call> FnOnce(TimeHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("time harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&TimeHarnessHandle),
{
    let native = TimeHarnessHandle::Native(NativeTimeHarness::new());
    callback(&native);
    let vm = TimeHarnessHandle::Vm(VmTimeHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(TimeHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
