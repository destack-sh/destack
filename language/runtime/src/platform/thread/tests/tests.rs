#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::NativeStringRef;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::thread::{ThreadOptions, ThreadOptionsVm};
use crate::runtime::BindingCallContext;
use crate::tests::runtime::TestRuntime;

/// Return canonical thread spawn options used by tests.
pub(crate) fn default_thread_options() -> ThreadOptions {
    ThreadOptions {
        stack_bytes: 0,
        flags: 0,
    }
}

/// Return canonical thread entry symbol used by tests.
pub(crate) fn test_thread_entry() -> &'static str {
    "destack.thread.test.entry"
}

/// Return canonical timeout that means wait indefinitely.
pub(crate) fn wait_forever_timeout() -> u64 {
    u64::MAX
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

/// Test harness context used by tests.
pub(crate) struct ThreadHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Native thread harness.
pub(crate) struct NativeThreadHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeThreadHarness {
    /// Create a new native thread harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM thread harness.
pub(crate) struct VmThreadHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmThreadHarness {
    /// Create a new VM thread harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum ThreadHarnessHandle {
    /// Native thread harness.
    Native(NativeThreadHarness),
    /// VM thread harness.
    Vm(VmThreadHarness),
}

impl ThreadHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(ThreadHarnessContext<'call>) -> R,
    {
        match self {
            ThreadHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(ThreadHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            ThreadHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(ThreadHarnessContext {
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
        F: for<'call> FnOnce(ThreadHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("thread harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&ThreadHarnessHandle),
{
    let native = ThreadHarnessHandle::Native(NativeThreadHarness::new());
    callback(&native);
    let vm = ThreadHarnessHandle::Vm(VmThreadHarness::new());
    callback(&vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(ThreadHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}
