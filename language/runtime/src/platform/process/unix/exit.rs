#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::process::{bindings_generated as bindings, core as core_process};
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::{NativeSlice, NativeStringRef, NativeStringSlice};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::process::{
    ExecAtFlags, GroupId, ProcessCpuSet, ProcessFdAction, ProcessFdFlags, ProcessFdSignalFlags,
    ProcessGroupIds, ProcessId, ProcessLimit, ProcessLimitResource, ProcessNamespaceKind,
    ProcessSchedulerConfig, ProcessSchedulerPolicy, ProcessSpawnOptions, ProcessStdio,
    ProcessUnshareFlags, ProcessUserIds, ProcessWaitFlags, ProcessWaitStatus, Signal, SignalEvent,
    SignalFdFlags, SignalMaskHow, SyscallFilterFlags, UserId,
};
use crate::platform::{fs, resource};

/// Exit the current process with the given code.
///
/// Terminate the current process without returning to the caller.
/// Exit code interpretation is host-defined and propagated to the parent process.
///
/// # Platform
/// Unix and Windows.
/// Uses _exit(2) or exit(3) on Unix and ExitProcess on Windows.
///
/// # Errors
/// Returns invalidArgument, processNotFound, processPermissionDenied, notSupported.
///
/// # Security
/// Requires `process.run`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_process_exit(
    _context: &BindingCallContext,
    code: u32,
) -> RuntimeResult<()> {
    unsafe { libc::_exit(code as i32) }
}
