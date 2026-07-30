use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Fixed runtime operation imported by generated native code.
#[repr(u16)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum Operation {
    /// Heap object allocation through the runtime.
    Allocate = 0x0000,
    /// Repeated heap backing allocation through the runtime.
    AllocateSlice = 0x0001,
    /// Unique heap release.
    Free = 0x0002,
    /// Heap pin.
    Pin = 0x0003,
    /// Heap unpin.
    Unpin = 0x0004,
    /// Managed reference write barrier.
    WriteBarrier = 0x0005,

    /// Poll runtime work at one safepoint.
    Poll = 0x0010,
    /// Stop execution for host inspection.
    Stop = 0x0011,
    /// Native to bytecode deoptimization.
    Deopt = 0x0012,
    /// Native trap exit.
    Trap = 0x0013,
    /// Language panic exit.
    Panic = 0x0014,
    /// Language panic exit with one typed value.
    PanicValue = 0x0015,
    /// Continue the active language unwind.
    UnwindResume = 0x0016,

    /// Queue one suspended waiter.
    WaiterQueue = 0x0020,
    /// Cancel one suspended waiter.
    WaiterCancel = 0x0021,
    /// Create one already completed task.
    TaskResolve = 0x0022,
    /// Start one running task.
    TaskStart = 0x0023,
    /// Suspend one running task.
    TaskSuspend = 0x0024,
    /// Park one waiter until a task settles.
    TaskPark = 0x0025,
    /// Request cooperative task cancellation.
    TaskCancel = 0x0026,
    /// Query cooperative task cancellation.
    TaskIsCancelled = 0x0027,
    /// Detach one task result.
    TaskDetach = 0x0028,
    /// Finish one task with its terminal outcome.
    TaskFinish = 0x0029,

    /// Current worker-local execution context.
    ContextCurrent = 0x0030,
    /// Scoped context push.
    ContextPush = 0x0031,
    /// Scoped context pop.
    ContextPop = 0x0032,
    /// Userland context entry lookup.
    ContextGet = 0x0033,
    /// Required userland context entry lookup.
    ContextRequire = 0x0034,
    /// Builtin context binding family lookup.
    ContextFamily = 0x0035,

    /// Runtime binding call through the active execution context.
    BindingCall = 0x0040,
}

impl Operation {
    /// Return the fixed runtime symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Allocate => "__destack_runtime_allocate",
            Self::AllocateSlice => "__destack_runtime_allocate_slice",
            Self::Free => "__destack_runtime_free",
            Self::Pin => "__destack_runtime_pin",
            Self::Unpin => "__destack_runtime_unpin",
            Self::WriteBarrier => "__destack_runtime_write_barrier",
            Self::Poll => "__destack_runtime_poll",
            Self::Stop => "__destack_runtime_stop",
            Self::Deopt => "__destack_runtime_deopt",
            Self::Trap => "__destack_runtime_trap",
            Self::Panic => "__destack_runtime_panic",
            Self::PanicValue => "__destack_runtime_panic_value",
            Self::UnwindResume => "__destack_runtime_unwind_resume",
            Self::WaiterQueue => "__destack_runtime_waiter_queue",
            Self::WaiterCancel => "__destack_runtime_waiter_cancel",
            Self::TaskResolve => "__destack_runtime_task_resolve",
            Self::TaskStart => "__destack_runtime_task_start",
            Self::TaskSuspend => "__destack_runtime_task_suspend",
            Self::TaskPark => "__destack_runtime_task_park",
            Self::TaskCancel => "__destack_runtime_task_cancel",
            Self::TaskIsCancelled => "__destack_runtime_task_is_cancelled",
            Self::TaskDetach => "__destack_runtime_task_detach",
            Self::TaskFinish => "__destack_runtime_task_finish",
            Self::ContextCurrent => "__destack_runtime_context_current",
            Self::ContextPush => "__destack_runtime_context_push",
            Self::ContextPop => "__destack_runtime_context_pop",
            Self::ContextGet => "__destack_runtime_context_get",
            Self::ContextRequire => "__destack_runtime_context_require",
            Self::ContextFamily => "__destack_runtime_context_family",
            Self::BindingCall => "__destack_runtime_binding_call",
        }
    }
}
