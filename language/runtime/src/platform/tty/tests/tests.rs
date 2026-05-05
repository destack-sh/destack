#![cfg_attr(windows, allow(dead_code, unused_imports))]

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::tty::PtyPair;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;
pub(crate) use crate::tests::platform::{assert_ok_or_expected_error, assert_platform_error_codes};
use crate::tests::runtime::TestRuntime;

#[path = "harness.rs"]
mod harness;

#[allow(unused_imports)]
pub(crate) use harness::*;

/// Test harness context used by tests.
pub(crate) struct TtyHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(crate) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(crate) vm_context: Option<*mut ()>,
}

/// Native tty harness.
pub(crate) struct NativeTtyHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeTtyHarness {
    /// Create a new native tty harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM tty harness.
pub(crate) struct VmTtyHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmTtyHarness {
    /// Create a new VM tty harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum TtyHarnessHandle {
    /// Native tty harness.
    Native(NativeTtyHarness),
    /// VM tty harness.
    Vm(VmTtyHarness),
}

impl TtyHarnessHandle {
    /// Run a native or VM call context around one callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(TtyHarnessContext<'call>) -> R,
    {
        match self {
            TtyHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(TtyHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            TtyHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(TtyHarnessContext {
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
        F: for<'call> FnOnce(TtyHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("tty harness call should succeed");
    }
}

/// Run one callback against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut TtyHarnessHandle),
{
    let mut native = TtyHarnessHandle::Native(NativeTtyHarness::new());
    callback(&mut native);
    let mut vm = TtyHarnessHandle::Vm(VmTtyHarness::new());
    callback(&mut vm);
}

/// Run one callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(TtyHarnessContext<'call>) -> RuntimeResult<()>,
{
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Open one pty pair or return none when the host does not support ptys.
pub(crate) fn open_pty_or_skip_not_supported(
    context: &mut TtyHarnessContext<'_>,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<Option<PtyPair>> {
    let pair = assert_ok_or_expected_error(
        context.destack_tty_pty_open(rows, columns, flags),
        &[PlatformErrorCode::NotSupported],
    )?;
    let Some(pair) = pair else {
        return Ok(None);
    };
    let pair = decode_harness_value(pair);

    Ok(Some(pair))
}

/// Decode one harness value where native and vm payloads are the same ABI type.
pub(crate) fn decode_harness_value<T>(value: HarnessValue<T, T>) -> T {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => value,
    }
}

/// Remove one tty worker entry through the resource table.
pub(crate) fn close_tty_worker_resource(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown tty handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Resolve one pty handle into one raw unix descriptor.
#[cfg(unix)]
pub(crate) fn pty_descriptor(
    binding: &BindingCallContext,
    handle: resource::PtyHandle,
) -> RuntimeResult<libc::c_int> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Pty {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown pty handle",
            ))
            .boxed()
        })?;

    Ok(descriptor)
}

/// Resolve one tty handle into one raw unix descriptor.
#[cfg(unix)]
pub(crate) fn tty_descriptor(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<libc::c_int> {
    let descriptor = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Tty {
                return None;
            }

            entry.fd()
        })
        .flatten()
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown tty handle",
            ))
            .boxed()
        })?;

    Ok(descriptor)
}
