use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Native imports required by one native code payload.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ImportTable {
    /// Native imports in linker order.
    import: Vec<Import>,
}

impl ImportTable {
    /// Create one native import table.
    pub fn new(import: Vec<Import>) -> Self {
        Self { import }
    }

    /// Create one empty native import table.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return native imports in linker order.
    pub fn imports(&self) -> &[Import] {
        &self.import
    }
}

/// One native import required by generated native code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Import {
    /// Fixed Destack runtime binding.
    Runtime(RuntimeBinding),
    /// External linker-visible symbol.
    Symbol(SymbolImport),
}

/// External linker-visible symbol import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SymbolImport {
    /// The imported native symbol.
    pub symbol: String,
}

impl SymbolImport {
    /// Create one native symbol import.
    pub fn new(symbol: String) -> Self {
        Self { symbol }
    }
}

/// Fixed Destack runtime ABI binding imported by generated native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum RuntimeBinding {
    /// Typed heap allocation.
    New,
    /// Typed repeated heap allocation.
    NewSlice,
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
            Self::New => "__destack_new",
            Self::NewSlice => "__destack_new_slice",
            Self::Free => "__destack_free",
            Self::Pin => "__destack_pin",
            Self::Unpin => "__destack_unpin",
            Self::WriteBarrier => "__destack_write_barrier",
            Self::Safepoint => "__destack_safepoint",
            Self::Yield => "__destack_yield",
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
