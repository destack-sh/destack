use destack_core::{Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native imports required by one native code payload.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ImportTable {
    /// Native imports in linker order.
    import: SectionSlice<Import>,
}

/// Build-time native import table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportTableBuilder {
    /// Native imports in linker order.
    imports: Vec<Import>,
}

impl ImportTableBuilder {
    /// Create an empty native import table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set native imports in linker order.
    pub fn imports(mut self, imports: impl IntoIterator<Item = Import>) -> Self {
        self.imports = imports.into_iter().collect();

        self
    }

    /// Build this import table into program sections.
    pub(super) fn build(self, sections: &mut SectionBuilder) -> ImportTable {
        ImportTable {
            import: sections.insert(self.imports),
        }
    }
}

impl ImportTable {
    /// Create one empty native import table.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return native imports in linker order.
    pub fn imports<'a>(&self, sections: SectionImage<'a>) -> &'a [Import] {
        sections.entries(self.import)
    }
}

/// One native import required by generated native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Import {
    /// Import kind.
    kind: ImportKind,
    /// Fixed runtime binding payload.
    runtime: Optional<RuntimeBinding>,
    /// External symbol payload.
    symbol: Optional<SymbolImport>,
}

impl Import {
    /// Create one fixed runtime binding import.
    pub fn runtime(runtime: RuntimeBinding) -> Self {
        Self {
            kind: ImportKind::Runtime,
            runtime: Optional::some(runtime),
            symbol: Optional::none(),
        }
    }

    /// Create one external symbol import.
    pub fn symbol(symbol: SymbolImport) -> Self {
        Self {
            kind: ImportKind::Symbol,
            runtime: Optional::none(),
            symbol: Optional::some(symbol),
        }
    }

    /// Return this import as a fixed runtime binding.
    pub fn runtime_value(self) -> Option<RuntimeBinding> {
        if self.kind == ImportKind::Runtime {
            self.runtime.get()
        } else {
            None
        }
    }

    /// Return this import as an external symbol.
    pub fn symbol_value(self) -> Option<SymbolImport> {
        if self.kind == ImportKind::Symbol {
            self.symbol.get()
        } else {
            None
        }
    }
}

/// Native import kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum ImportKind {
    /// Fixed runtime binding import.
    Runtime = 0,
    /// External symbol import.
    Symbol = 1,
}

/// External linker-visible symbol import.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SymbolImport {
    /// The imported native symbol.
    pub symbol: StringId,
}

impl SymbolImport {
    /// Create one native symbol import.
    pub const fn new(symbol: StringId) -> Self {
        Self { symbol }
    }
}

/// Fixed Destack runtime ABI binding imported by generated native code.
#[repr(u16)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum RuntimeBinding {
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

    /// Runtime safepoint cooperation.
    Safepoint = 0x0010,
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

    /// Official host binding call through the active context implementation.
    BindingCall = 0x0040,
}

impl RuntimeBinding {
    /// Return the fixed runtime symbol name.
    pub const fn symbol_name(self) -> &'static str {
        match self {
            Self::Allocate => "__destack_allocate",
            Self::AllocateSlice => "__destack_allocate_slice",
            Self::Free => "__destack_free",
            Self::Pin => "__destack_pin",
            Self::Unpin => "__destack_unpin",
            Self::WriteBarrier => "__destack_write_barrier",
            Self::Safepoint => "__destack_safepoint",
            Self::Stop => "__destack_stop",
            Self::Deopt => "__destack_deopt",
            Self::Trap => "__destack_trap",
            Self::Panic => "__destack_panic",
            Self::PanicValue => "__destack_panic_value",
            Self::UnwindResume => "__destack_unwind_resume",
            Self::WaiterQueue => "__destack_waiter_queue",
            Self::WaiterCancel => "__destack_waiter_cancel",
            Self::TaskResolve => "__destack_task_resolve",
            Self::TaskStart => "__destack_task_start",
            Self::TaskSuspend => "__destack_task_suspend",
            Self::TaskPark => "__destack_task_park",
            Self::TaskCancel => "__destack_task_cancel",
            Self::TaskIsCancelled => "__destack_task_is_cancelled",
            Self::TaskDetach => "__destack_task_detach",
            Self::ContextCurrent => "__destack_context_current",
            Self::ContextPush => "__destack_context_push",
            Self::ContextPop => "__destack_context_pop",
            Self::ContextGet => "__destack_context_get",
            Self::ContextRequire => "__destack_context_require",
            Self::ContextFamily => "__destack_context_family",
            Self::BindingCall => "__destack_binding_call",
        }
    }
}
