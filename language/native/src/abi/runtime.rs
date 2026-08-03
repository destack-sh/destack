use serde::{Deserialize, Serialize};

use super::{Activation, Space, TaskOutcome, Unwind};

macro_rules! runtime_operations {
    ($macro:ident) => {
        $macro! {
            /// Allocate one heap object.
            Allocate = 0x0000 => allocate: Allocate -> Pointer,
            /// Allocate one repeated heap backing.
            AllocateRepeated = 0x0001 => allocate_repeated: AllocateRepeated -> Pointer,
            /// Release one unique heap value.
            Free = 0x0002 => free: Free -> Void,
            /// Pin one heap value.
            Pin = 0x0003 => pin: Pin -> Pointer,
            /// Release one pinned heap value.
            Unpin = 0x0004 => unpin: Unpin -> Void,
            /// Record one managed reference write.
            WriteBarrier = 0x0005 => write_barrier: WriteBarrier -> Void,

            /// Poll pending runtime work.
            Poll = 0x0010 => poll: Poll -> Never,
            /// Stop execution for host inspection.
            Stop = 0x0011 => stop: Stop -> Never,
            /// Deoptimize native execution.
            Deopt = 0x0012 => deopt: Deopt -> Never,

            /// Report one payloadless language panic.
            Panic = 0x0014 => panic: Panic -> Never,
            /// Report one typed language panic.
            PanicValue = 0x0015 => panic_value: PanicValue -> Never,
            /// Classify the active unwind.
            UnwindClassify = 0x0016 => unwind_classify: ClassifyUnwind -> Uint32,
            /// Continue the active unwind.
            UnwindResume = 0x0017 => unwind_resume: ResumeUnwind -> Never,
            /// Queue one suspended waiter.
            WaiterQueue = 0x0020 => waiter_queue: QueueWaiter -> Uint32,
            /// Cancel one suspended waiter.
            WaiterCancel = 0x0021 => waiter_cancel: CancelWaiter -> Uint32,
            /// Create one completed task.
            TaskResolve = 0x0022 => task_resolve: ResolveTask -> Uint64,
            /// Start one running task.
            TaskStart = 0x0023 => task_start: StartTask -> Uint64,
            /// Suspend one running task.
            TaskSuspend = 0x0024 => task_suspend: SuspendTask -> Uint64,
            /// Park one waiter until a task settles.
            TaskPark = 0x0025 => task_park: ParkTask -> Void,
            /// Request cooperative task cancellation.
            TaskCancel = 0x0026 => task_cancel: CancelTask -> Void,
            /// Query cooperative task cancellation.
            TaskIsCancelled = 0x0027 => task_is_cancelled: IsTaskCancelled -> Uint32,
            /// Detach one task result.
            TaskDetach = 0x0028 => task_detach: DetachTask -> Void,
            /// Finish one task.
            TaskFinish = 0x0029 => task_finish: FinishTask -> Void,

            /// Call one runtime binding.
            BindingCall = 0x0040 => binding_call: BindingCall -> Void,
        }
    };
}

pub(super) use runtime_operations;

macro_rules! define_runtime {
    ($( $(#[$meta:meta])* $operation:ident = $code:literal => $field:ident: $ty:ident -> $result:ident, )*) => {
        /// Runtime operations callable by generated native code.
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        pub struct Runtime {
            $(
                $(#[$meta])*
                pub $field: $ty,
            )*
        }
    };
}

runtime_operations!(define_runtime);

/// Native allocation initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationInitialization {
    /// Initialize bytes to zero.
    Zeroed = 0,
    /// Leave bytes uninitialized.
    Uninit = 1,
}

/// Allocate one heap object through the runtime.
pub type Allocate = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    space: Space,
    allocation_plan: u32,
    initialization: AllocationInitialization,
) -> usize;

/// Allocate one repeated heap backing through the runtime.
pub type AllocateRepeated = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    space: Space,
    element_allocation_plan: u32,
    length: usize,
    initialization: AllocationInitialization,
) -> usize;

/// Release one unique heap value through the runtime.
pub type Free =
    unsafe extern "C-unwind" fn(activation: *mut Activation, space: Space, value: usize);

/// Pin one heap value against movement through the runtime.
pub type Pin =
    unsafe extern "C-unwind" fn(activation: *mut Activation, space: Space, value: usize) -> usize;

/// Release one pinned heap value through the runtime.
pub type Unpin =
    unsafe extern "C-unwind" fn(activation: *mut Activation, space: Space, value: usize);

/// Record one managed reference write through the runtime.
pub type WriteBarrier = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    space: Space,
    object: usize,
    offset: usize,
    byte_len: usize,
);

/// Poll runtime work at one reconstructable native frame.
pub type Poll = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    marker: *const u8,
) -> !;

/// Pause execution and return control to the host.
pub type Stop = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    marker: *const u8,
) -> !;

/// Deoptimize native execution into interpreter state.
pub type Deopt = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    frame_map: u32,
    marker: *const u8,
) -> !;

/// Start unwinding one payloadless language panic.
pub type Panic = unsafe extern "C-unwind" fn(activation: *mut Activation) -> !;

/// Start unwinding one typed language panic.
pub type PanicValue =
    unsafe extern "C-unwind" fn(activation: *mut Activation, ty: u32, words: *const u64) -> !;

/// Classify the active native unwind.
pub type ClassifyUnwind = unsafe extern "C-unwind" fn(activation: *mut Activation) -> UnwindAction;

/// Continue the active platform unwind.
pub type ResumeUnwind =
    unsafe extern "C-unwind" fn(activation: *mut Activation, unwind: *mut Unwind) -> !;

/// Action selected for one active native unwind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnwindAction {
    /// Run the local cleanup path.
    Cleanup = 0,
    /// Retain live values without running cleanup.
    Retain = 1,
}

/// Queue one suspended waiter with a typed value.
pub type QueueWaiter = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    waiter: u64,
    ty: u32,
    words: *const u64,
) -> u32;

/// Cancel one suspended waiter.
pub type CancelWaiter =
    unsafe extern "C-unwind" fn(activation: *mut Activation, waiter: u64) -> u32;

/// Create one already completed task.
pub type ResolveTask =
    unsafe extern "C-unwind" fn(activation: *mut Activation, ty: u32, words: *const u64) -> u64;

/// Start one running task.
pub type StartTask = unsafe extern "C-unwind" fn(activation: *mut Activation) -> u64;

/// Suspend one running task at a reconstructable native frame.
pub type SuspendTask = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    task: u64,
    frame_map: u32,
    marker: *const u8,
) -> u64;

/// Park one waiter until a task settles.
pub type ParkTask =
    unsafe extern "C-unwind" fn(activation: *mut Activation, task: u64, waiter: u64);

/// Request cooperative cancellation of one task.
pub type CancelTask = unsafe extern "C-unwind" fn(activation: *mut Activation, task: u64);

/// Query whether cooperative cancellation was requested for one task.
pub type IsTaskCancelled =
    unsafe extern "C-unwind" fn(activation: *mut Activation, task: u64) -> u32;

/// Detach one task result.
pub type DetachTask = unsafe extern "C-unwind" fn(activation: *mut Activation, task: u64);

/// Finish one task with its terminal outcome.
pub type FinishTask = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    task: u64,
    outcome: TaskOutcome,
    result_type: u32,
    result: *const u64,
);

/// Call one runtime binding selected by its Program function.
pub type BindingCall = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    function: u32,
    arguments: *const u64,
    argument_count: usize,
    result: *mut u64,
    result_count: usize,
);
