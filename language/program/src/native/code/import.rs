use destack_core::{Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice, StringId};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native imports required by one native code payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ImportTable {
    /// Native imports in linker order.
    import: SectionSlice<Import>,
}

impl ImportTable {
    /// Pack one native import table.
    pub fn pack(sections: &mut SectionPacker, import: Vec<Import>) -> Self {
        Self {
            import: sections.insert(import),
        }
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ImportKind {
    /// Fixed runtime binding import.
    Runtime = 0,
    /// External symbol import.
    Symbol = 1,
}

/// External linker-visible symbol import.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum RuntimeBinding {
    /// Heap object allocation through the runtime.
    Allocate,
    /// Repeated heap backing allocation through the runtime.
    AllocateSlice,
    /// Unique heap release.
    Free,
    /// Heap pin.
    Pin,
    /// Heap unpin.
    Unpin,
    /// Managed reference write barrier.
    WriteBarrier,
    /// Runtime safepoint cooperation.
    Safepoint,
    /// Coroutine suspension into the runtime scheduler.
    Yield,
    /// Stop execution for host inspection.
    Stop,
    /// Native to VM deoptimization.
    Deopt,
    /// Native trap exit.
    Trap,
    /// Language panic exit.
    Panic,
    /// Active language unwind continuation.
    UnwindResume,
    /// Current worker-local execution context.
    ContextCurrent,
    /// Scoped context push.
    ContextPush,
    /// Scoped context pop.
    ContextPop,
    /// Userland context entry lookup.
    ContextGet,
    /// Required userland context entry lookup.
    ContextRequire,
    /// Builtin context binding family lookup.
    ContextFamily,
    /// Official host binding call through the active context implementation.
    BindingCall,
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
            Self::Yield => "__destack_yield",
            Self::Stop => "__destack_stop",
            Self::Deopt => "__destack_deopt",
            Self::Trap => "__destack_trap",
            Self::Panic => "__destack_panic",
            Self::UnwindResume => "__destack_unwind_resume",
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

// SAFETY: native import entries are fixed-width program entries.
unsafe impl SectionEntry for Import {}
unsafe impl SectionEntry for ImportKind {}
unsafe impl SectionEntry for SymbolImport {}
unsafe impl SectionEntry for RuntimeBinding {}
