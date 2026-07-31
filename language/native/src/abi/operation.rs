use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

macro_rules! operations {
    (
        $(
            $(#[$meta:meta])*
            $operation:ident = $code:literal => $symbol:literal,
        )*
    ) => {
        /// Fixed runtime operation imported by generated native code.
        #[repr(u16)]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            Serialize,
            Deserialize,
            Reflect,
            SectionEntry,
        )]
        pub enum Operation {
            $(
                $(#[$meta])*
                $operation = $code,
            )*
        }

        impl Operation {
            /// Return the fixed runtime symbol.
            pub const fn symbol(self) -> &'static str {
                match self {
                    $(Self::$operation => $symbol,)*
                }
            }
        }
    };
}

operations! {
    /// Heap object allocation through the runtime.
    Allocate = 0x0000 => "__destack_runtime_allocate",
    /// Repeated heap backing allocation through the runtime.
    AllocateSlice = 0x0001 => "__destack_runtime_allocate_slice",
    /// Unique heap release.
    Free = 0x0002 => "__destack_runtime_free",
    /// Heap pin.
    Pin = 0x0003 => "__destack_runtime_pin",
    /// Heap unpin.
    Unpin = 0x0004 => "__destack_runtime_unpin",
    /// Managed reference write barrier.
    WriteBarrier = 0x0005 => "__destack_runtime_write_barrier",

    /// Poll runtime work at one reconstructable native frame.
    Poll = 0x0010 => "__destack_runtime_poll",
    /// Stop execution for host inspection.
    Stop = 0x0011 => "__destack_runtime_stop",
    /// Native to bytecode deoptimization.
    Deopt = 0x0012 => "__destack_runtime_deopt",
    /// Native trap exit.
    Trap = 0x0013 => "__destack_runtime_trap",
    /// Language panic exit.
    Panic = 0x0014 => "__destack_runtime_panic",
    /// Language panic exit with one typed value.
    PanicValue = 0x0015 => "__destack_runtime_panic_value",
    /// Continue the active language unwind.
    UnwindResume = 0x0016 => "__destack_runtime_unwind_resume",

    /// Queue one suspended waiter.
    WaiterQueue = 0x0020 => "__destack_runtime_waiter_queue",
    /// Cancel one suspended waiter.
    WaiterCancel = 0x0021 => "__destack_runtime_waiter_cancel",
    /// Create one already completed task.
    TaskResolve = 0x0022 => "__destack_runtime_task_resolve",
    /// Start one running task.
    TaskStart = 0x0023 => "__destack_runtime_task_start",
    /// Suspend one running task.
    TaskSuspend = 0x0024 => "__destack_runtime_task_suspend",
    /// Park one waiter until a task settles.
    TaskPark = 0x0025 => "__destack_runtime_task_park",
    /// Request cooperative task cancellation.
    TaskCancel = 0x0026 => "__destack_runtime_task_cancel",
    /// Query cooperative task cancellation.
    TaskIsCancelled = 0x0027 => "__destack_runtime_task_is_cancelled",
    /// Detach one task result.
    TaskDetach = 0x0028 => "__destack_runtime_task_detach",
    /// Finish one task with its terminal outcome.
    TaskFinish = 0x0029 => "__destack_runtime_task_finish",

    /// Current worker-local execution context.
    ContextCurrent = 0x0030 => "__destack_runtime_context_current",
    /// Scoped context push.
    ContextPush = 0x0031 => "__destack_runtime_context_push",
    /// Scoped context pop.
    ContextPop = 0x0032 => "__destack_runtime_context_pop",
    /// Userland context entry lookup.
    ContextGet = 0x0033 => "__destack_runtime_context_get",
    /// Required userland context entry lookup.
    ContextRequire = 0x0034 => "__destack_runtime_context_require",
    /// Builtin context binding family lookup.
    ContextFamily = 0x0035 => "__destack_runtime_context_family",

    /// Runtime binding call through the active execution context.
    BindingCall = 0x0040 => "__destack_runtime_binding_call",
}
