use serde::{Deserialize, Serialize};

use super::{
    ConstantSpace, Exit, ExitCode, RuntimeStatusCode, StaticSpace, TaskOutcomeCode, TrapCode,
};

/// Opaque runtime owner for one active native call.
#[repr(C)]
#[derive(Debug)]
pub struct Call {
    /// Prevent external construction.
    _private: [u8; 0],
}

/// Native activation passed to generated code and runtime operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activation {
    /// The runtime owner for this call.
    pub call: *mut Call,
    /// Program constant bytes.
    pub constants: ConstantSpace,
    /// Runtime-shared static bytes.
    pub shared_statics: StaticSpace,
    /// Worker-local static bytes.
    pub local_statics: StaticSpace,
    /// Exit record written before non-completion returns.
    pub exit: *mut Exit,
}

/// Native allocation initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationInitialization {
    /// Initialize bytes to zero.
    Zeroed = 0,
    /// Leave bytes uninitialized.
    Uninit = 1,
}

impl Activation {
    /// Create one native activation.
    pub const fn new(
        call: *mut Call,
        constants: ConstantSpace,
        shared_statics: StaticSpace,
        local_statics: StaticSpace,
        exit: *mut Exit,
    ) -> Self {
        Self {
            call,
            constants,
            shared_statics,
            local_statics,
            exit,
        }
    }
}

/// Native entry function.
pub type Entry = unsafe extern "C" fn(
    activation: *mut Activation,
    arguments: *const u64,
    result: *mut u64,
) -> ExitCode;

/// Allocate one heap object through the runtime.
pub type Allocate = unsafe extern "C" fn(
    activation: *mut Activation,
    allocation_plan: u32,
    initialization: AllocationInitialization,
    result: *mut usize,
) -> RuntimeStatusCode;

/// Allocate one repeated heap backing through the runtime.
pub type AllocateSlice = unsafe extern "C" fn(
    activation: *mut Activation,
    element_allocation_plan: u32,
    length: usize,
    initialization: AllocationInitialization,
    result: *mut usize,
) -> RuntimeStatusCode;

/// Release one unique heap value through the runtime.
pub type Free =
    unsafe extern "C" fn(activation: *mut Activation, value: usize) -> RuntimeStatusCode;

/// Pin one heap value against movement through the runtime.
pub type Pin = unsafe extern "C" fn(
    activation: *mut Activation,
    value: usize,
    result: *mut usize,
) -> RuntimeStatusCode;

/// Release one pinned heap value through the runtime.
pub type Unpin =
    unsafe extern "C" fn(activation: *mut Activation, value: usize) -> RuntimeStatusCode;

/// Record one managed reference write through the runtime.
pub type WriteBarrier = unsafe extern "C" fn(
    activation: *mut Activation,
    object: usize,
    offset: usize,
    byte_len: usize,
) -> RuntimeStatusCode;

/// Poll runtime work at one native safepoint.
pub type Poll =
    unsafe extern "C" fn(activation: *mut Activation, safepoint: u32) -> RuntimeStatusCode;

/// Stop execution for host inspection.
pub type Stop = unsafe extern "C" fn(activation: *mut Activation, safepoint: u32) -> ExitCode;

/// Deoptimize native execution into interpreter state.
pub type Deopt = unsafe extern "C" fn(activation: *mut Activation, safepoint: u32) -> ExitCode;

/// Report one native trap.
pub type TrapExit = unsafe extern "C" fn(activation: *mut Activation, trap: TrapCode) -> ExitCode;

/// Report one payloadless language panic.
pub type Panic = unsafe extern "C" fn(activation: *mut Activation) -> ExitCode;

/// Copy one typed language panic payload into runtime ownership.
pub type PanicValue =
    unsafe extern "C" fn(activation: *mut Activation, ty: u32, words: *const u64) -> ExitCode;

/// Continue the active language unwind.
pub type UnwindResume = unsafe extern "C" fn(activation: *mut Activation) -> ExitCode;

/// Queue one suspended waiter with a typed value.
pub type QueueWaiter = unsafe extern "C" fn(
    activation: *mut Activation,
    waiter: u64,
    ty: u32,
    words: *const u64,
    result: *mut u32,
) -> RuntimeStatusCode;

/// Cancel one suspended waiter.
pub type CancelWaiter = unsafe extern "C" fn(
    activation: *mut Activation,
    waiter: u64,
    result: *mut u32,
) -> RuntimeStatusCode;

/// Create one already completed task.
pub type ResolveTask = unsafe extern "C" fn(
    activation: *mut Activation,
    ty: u32,
    words: *const u64,
    result: *mut u64,
) -> RuntimeStatusCode;

/// Start one running task.
pub type StartTask =
    unsafe extern "C" fn(activation: *mut Activation, result: *mut u64) -> RuntimeStatusCode;

/// Suspend one running task with canonical continuation storage.
pub type SuspendTask = unsafe extern "C" fn(
    activation: *mut Activation,
    task: u64,
    completion: u32,
    states: *const u32,
    state_count: usize,
    bytes: *const u8,
    byte_len: usize,
    result: *mut u64,
) -> RuntimeStatusCode;

/// Park one waiter until one task settles.
pub type ParkTask =
    unsafe extern "C" fn(activation: *mut Activation, task: u64, waiter: u64) -> RuntimeStatusCode;

/// Request cooperative cancellation of one task.
pub type CancelTask =
    unsafe extern "C" fn(activation: *mut Activation, task: u64) -> RuntimeStatusCode;

/// Query whether cooperative cancellation was requested for one task.
pub type IsTaskCancelled = unsafe extern "C" fn(
    activation: *mut Activation,
    task: u64,
    result: *mut u32,
) -> RuntimeStatusCode;

/// Detach one task result.
pub type DetachTask =
    unsafe extern "C" fn(activation: *mut Activation, task: u64) -> RuntimeStatusCode;

/// Finish one task with its terminal outcome.
pub type FinishTask = unsafe extern "C" fn(
    activation: *mut Activation,
    task: u64,
    outcome: TaskOutcomeCode,
    result_type: u32,
    result: *const u64,
) -> RuntimeStatusCode;

/// Return the current worker-local execution context.
pub type CurrentContext =
    unsafe extern "C" fn(activation: *mut Activation, result: *mut u64) -> RuntimeStatusCode;

/// Push one typed scoped context patch.
pub type PushContext = unsafe extern "C" fn(
    activation: *mut Activation,
    patch_type: u32,
    patch: *const u64,
    result: *mut u64,
) -> RuntimeStatusCode;

/// Pop one scoped context patch.
pub type PopContext =
    unsafe extern "C" fn(activation: *mut Activation, token: u64) -> RuntimeStatusCode;

/// Read one optional typed userland context entry.
pub type GetContext = unsafe extern "C" fn(
    activation: *mut Activation,
    owner: u64,
    key: u64,
    result_type: u32,
    result: *mut u64,
    is_present: *mut u32,
) -> RuntimeStatusCode;

/// Read one required typed userland context entry.
pub type RequireContext = unsafe extern "C" fn(
    activation: *mut Activation,
    owner: u64,
    key: u64,
    result_type: u32,
    result: *mut u64,
) -> RuntimeStatusCode;

/// Read one builtin binding family from the current context.
pub type ContextFamily = unsafe extern "C" fn(
    activation: *mut Activation,
    family: u32,
    result: *mut u64,
) -> RuntimeStatusCode;

/// Call one runtime binding selected by its Program function.
pub type BindingCall = unsafe extern "C" fn(
    activation: *mut Activation,
    function: u32,
    arguments: *const u64,
    argument_count: usize,
    result: *mut u64,
    result_count: usize,
) -> RuntimeStatusCode;
