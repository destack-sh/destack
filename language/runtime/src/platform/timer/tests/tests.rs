#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::timer::{TimerClock, TimerFdSpec, TimerFdSpecVm, TimerFlags, TimerOptions};
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;

/// Return canonical timer options for tests.
pub(crate) fn default_timer_options() -> TimerOptions {
    TimerOptions {
        clock: TimerClock::Monotonic,
        flags: TimerFlags(0),
    }
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

/// Normalize one harness timerfd spec payload into the native value shape.
pub(crate) fn timer_fd_spec_from_value(
    value: harness::HarnessValue<TimerFdSpec, TimerFdSpecVm>,
) -> TimerFdSpec {
    match value {
        harness::HarnessValue::Native(value) => value,
        harness::HarnessValue::Vm(value) => value,
    }
}

/// Test harness context used by tests.
pub(crate) struct TimerHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native timer harness.
pub(crate) struct NativeTimerHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTimerHarness {
    /// Create a new native timer harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM timer harness.
pub(crate) struct VmTimerHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTimerHarness {
    /// Create a new VM timer harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TimerHarnessHandle {
    /// Native timer harness.
    Native(NativeTimerHarness),
    /// VM timer harness.
    Vm(VmTimerHarness),
}

impl TimerHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(TimerHarnessContext<'call>) -> R,
    {
        match self {
            TimerHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(TimerHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            TimerHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(TimerHarnessContext {
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
        F: for<'call> FnOnce(TimerHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("timer harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&TimerHarnessHandle),
{
    let native = TimerHarnessHandle::Native(NativeTimerHarness::new());
    callback(&native);
    let vm = TimerHarnessHandle::Vm(VmTimerHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(TimerHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
