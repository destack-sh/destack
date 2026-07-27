use crate::{Completion, FrameStateId, Task, TypeId, Waiter, Word};

use super::{NativeContext, NativeRuntimeStatusCode};

/// Queue one suspended waiter with a typed value.
pub type NativeQueueWaiter = unsafe extern "C" fn(
    context: *mut NativeContext,
    waiter: Waiter,
    ty: TypeId,
    words: *const Word,
    result: *mut u32,
) -> NativeRuntimeStatusCode;

/// Cancel one suspended waiter.
pub type NativeCancelWaiter = unsafe extern "C" fn(
    context: *mut NativeContext,
    waiter: Waiter,
    result: *mut u32,
) -> NativeRuntimeStatusCode;

/// Create one already completed task.
pub type NativeResolveTask = unsafe extern "C" fn(
    context: *mut NativeContext,
    ty: TypeId,
    words: *const Word,
    result: *mut Task,
) -> NativeRuntimeStatusCode;

/// Start one running task.
pub type NativeStartTask =
    unsafe extern "C" fn(context: *mut NativeContext, result: *mut Task) -> NativeRuntimeStatusCode;

/// Suspend one running task with canonical continuation storage.
pub type NativeSuspendTask = unsafe extern "C" fn(
    context: *mut NativeContext,
    task: Task,
    completion: Completion,
    states: *const FrameStateId,
    state_count: usize,
    bytes: *const u8,
    byte_len: usize,
    result: *mut Waiter,
) -> NativeRuntimeStatusCode;

/// Park one waiter until one task settles.
pub type NativeParkTask = unsafe extern "C" fn(
    context: *mut NativeContext,
    task: Task,
    waiter: Waiter,
) -> NativeRuntimeStatusCode;

/// Request cooperative cancellation of one task.
pub type NativeCancelTask =
    unsafe extern "C" fn(context: *mut NativeContext, task: Task) -> NativeRuntimeStatusCode;

/// Query whether cooperative cancellation was requested for one task.
pub type NativeIsTaskCancelled = unsafe extern "C" fn(
    context: *mut NativeContext,
    task: Task,
    result: *mut u32,
) -> NativeRuntimeStatusCode;

/// Detach one task result.
pub type NativeDetachTask =
    unsafe extern "C" fn(context: *mut NativeContext, task: Task) -> NativeRuntimeStatusCode;
