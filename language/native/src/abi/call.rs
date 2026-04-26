use std::ffi::c_void;

use crate::{Status, Trap};

/// Opaque runtime context pointer passed to generated native code.
pub type Context = c_void;

/// Loaded native entry function.
pub type EntryFn =
    unsafe extern "C" fn(context: *mut Context, args: *const u8, out: *mut u8) -> Status;

/// Allocate one local managed value.
pub type AllocFn = unsafe extern "C" fn(context: *mut Context, layout: u32) -> usize;

/// Allocate one local managed slice.
pub type AllocSliceFn =
    unsafe extern "C" fn(context: *mut Context, layout: u32, len: usize) -> usize;

/// Allocate one shared managed value.
pub type AllocSharedFn = unsafe extern "C" fn(context: *mut Context, layout: u32) -> usize;

/// Allocate one local raw byte range.
pub type RawAllocFn =
    unsafe extern "C" fn(context: *mut Context, byte_len: usize, alignment: usize) -> usize;

/// Free one local raw pointer.
pub type RawFreeFn = unsafe extern "C" fn(context: *mut Context, pointer: usize);

/// Allocate one shared raw byte range.
pub type SharedRawAllocFn =
    unsafe extern "C" fn(context: *mut Context, byte_len: usize, alignment: usize) -> usize;

/// Free one shared raw pointer.
pub type SharedRawFreeFn = unsafe extern "C" fn(context: *mut Context, pointer: usize);

/// Record one local heap edge store.
pub type WriteBarrierFn = unsafe extern "C" fn(context: *mut Context, dst: *mut u8, value: usize);

/// Cooperate with the runtime at one safepoint.
pub type SafepointFn = unsafe extern "C" fn(context: *mut Context, safepoint: u32);

/// Yield execution from generated code.
pub type YieldFn = unsafe extern "C" fn(context: *mut Context, resume: u32, value: *const u8) -> !;

/// Report one fatal native trap.
pub type TrapFn = unsafe extern "C" fn(context: *mut Context, code: Trap) -> !;

/// Call one runtime platform binding.
pub type BindingFn = unsafe extern "C" fn(
    context: *mut Context,
    binding: u32,
    args: *const u8,
    out: *mut u8,
) -> Status;

/// One runtime helper referenced by native code metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Import {
    /// Local managed allocation.
    Alloc,
    /// Local managed slice allocation.
    AllocSlice,
    /// Shared managed allocation.
    AllocShared,
    /// Local raw allocation.
    RawAlloc,
    /// Local raw free.
    RawFree,
    /// Shared raw allocation.
    SharedRawAlloc,
    /// Shared raw free.
    SharedRawFree,
    /// Local heap edge write barrier.
    WriteBarrier,
    /// Safepoint cooperation.
    Safepoint,
    /// Yield from generated code.
    Yield,
    /// Fatal trap.
    Trap,
    /// Platform binding call.
    CallBinding,
}

impl Import {
    /// Return the runtime symbol imported by generated native code.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Alloc => "__destack_alloc",
            Self::AllocSlice => "__destack_alloc_slice",
            Self::AllocShared => "__destack_alloc_shared",
            Self::RawAlloc => "__destack_raw_alloc",
            Self::RawFree => "__destack_raw_free",
            Self::SharedRawAlloc => "__destack_shared_raw_alloc",
            Self::SharedRawFree => "__destack_shared_raw_free",
            Self::WriteBarrier => "__destack_write_barrier",
            Self::Safepoint => "__destack_safepoint",
            Self::Yield => "__destack_yield",
            Self::Trap => "__destack_trap",
            Self::CallBinding => "__destack_call_binding",
        }
    }
}
