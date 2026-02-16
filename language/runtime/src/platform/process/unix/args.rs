#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, NativeStringSlice, PlatformError,
};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdActionKind, ProcessFdFlags,
    ProcessFdSignalFlags, ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource,
    ProcessNamespaceKind, ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions,
    ProcessStdio, ProcessStdioKind, ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags,
    ProcessWaitKind, ProcessWaitStatus, Signal, SignalEvent, SignalFdFlags, SignalMaskHow,
    SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Return the process argument vector.
///
/// Read the immutable argument list captured by the runtime at process startup.
/// Argument decoding and quoting semantics follow the host process loader.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses startup argument capture, not a dedicated syscall.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_args(
    context: &RuntimeCallContext,
    out: *mut NativeStringSlice,
) -> RuntimeResult<()> {
    context.check_policy(PROCESS_ARGS_ARGS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    // collect argument strings into call-local storage
    let args = core_process::process_args(context.platform());
    let mut values = Vec::with_capacity(args.len());
    for argument in args {
        values.push(context.store_string(argument));
    }

    // write the encoded string slice
    unsafe {
        *out = context.store_string_slice(values);
    }

    Ok(())
}
