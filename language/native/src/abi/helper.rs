use serde::{Deserialize, Serialize};

use crate::{NativeContext, NativeStatusCode, NativeSymbol, NativeTrapCode, NativeValue};

/// Allocate one local managed value.
pub type NativeAlloc = unsafe extern "C" fn(context: *mut NativeContext, layout: u32) -> usize;

/// Allocate one local managed slice.
pub type NativeAllocSlice =
    unsafe extern "C" fn(context: *mut NativeContext, layout: u32, length: usize) -> usize;

/// Allocate one shared managed value.
pub type NativeAllocShared =
    unsafe extern "C" fn(context: *mut NativeContext, layout: u32) -> usize;

/// Record one local heap edge store.
pub type NativeWriteBarrier =
    unsafe extern "C" fn(context: *mut NativeContext, destination: *mut u8, value: usize);

/// Cooperate with the runtime at one safepoint.
pub type NativeSafepoint =
    unsafe extern "C" fn(context: *mut NativeContext, safepoint: u32) -> NativeStatusCode;

/// Deoptimize native execution into VM materialization.
pub type NativeDeopt =
    unsafe extern "C" fn(context: *mut NativeContext, safepoint: u32) -> NativeStatusCode;

/// Report one native trap.
pub type NativeTrapFunction =
    unsafe extern "C" fn(context: *mut NativeContext, trap: NativeTrapCode) -> NativeStatusCode;

/// Report one language panic.
pub type NativePanic =
    unsafe extern "C" fn(context: *mut NativeContext, payload: NativeValue) -> NativeStatusCode;

/// Abort native execution.
pub type NativeAbort = unsafe extern "C" fn(context: *mut NativeContext, payload: NativeValue) -> !;

/// Runtime helper imported by generated native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NativeHelper {
    /// Local managed allocation.
    Alloc,
    /// Local managed slice allocation.
    AllocSlice,
    /// Shared managed allocation.
    AllocShared,
    /// Local heap edge write barrier.
    WriteBarrier,
    /// Runtime safepoint cooperation.
    Safepoint,
    /// Native to VM deoptimization.
    Deopt,
    /// Native trap exit.
    Trap,
    /// Language panic exit.
    Panic,
    /// Immediate abort.
    Abort,
}

impl NativeHelper {
    /// Return the runtime symbol imported by generated native code.
    pub fn symbol(self) -> NativeSymbol {
        NativeSymbol::new(self.symbol_name())
    }

    /// Return the runtime symbol name imported by generated native code.
    pub const fn symbol_name(self) -> &'static str {
        match self {
            Self::Alloc => "__destack_alloc",
            Self::AllocSlice => "__destack_alloc_slice",
            Self::AllocShared => "__destack_alloc_shared",
            Self::WriteBarrier => "__destack_write_barrier",
            Self::Safepoint => "__destack_safepoint",
            Self::Deopt => "__destack_deopt",
            Self::Trap => "__destack_trap",
            Self::Panic => "__destack_panic",
            Self::Abort => "__destack_abort",
        }
    }
}

/// Native import referenced by generated code metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NativeImport {
    /// Runtime helper import.
    Helper(NativeHelper),
    /// Generated host or runtime binding thunk import.
    Binding(NativeSymbol),
}

impl NativeImport {
    /// Return the imported symbol.
    pub fn symbol(&self) -> NativeSymbol {
        match self {
            Self::Helper(helper) => helper.symbol(),
            Self::Binding(symbol) => symbol.clone(),
        }
    }
}
