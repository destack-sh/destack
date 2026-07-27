use serde::{Deserialize, Serialize};

use super::{NativeContext, NativeRuntimeStatusCode};

/// Native allocation initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeAllocationInitialization {
    /// Initialize bytes to zero.
    Zeroed = 0,
    /// Leave bytes uninitialized.
    Uninit = 1,
}

/// Allocate one heap object through the runtime.
pub type NativeAllocate = unsafe extern "C" fn(
    context: *mut NativeContext,
    allocation_plan: u32,
    initialization: NativeAllocationInitialization,
    result: *mut usize,
) -> NativeRuntimeStatusCode;

/// Allocate one repeated heap backing through the runtime.
pub type NativeAllocateSlice = unsafe extern "C" fn(
    context: *mut NativeContext,
    element_allocation_plan: u32,
    length: usize,
    initialization: NativeAllocationInitialization,
    result: *mut usize,
) -> NativeRuntimeStatusCode;

/// Release one unique heap value through the runtime.
pub type NativeFree =
    unsafe extern "C" fn(context: *mut NativeContext, value: usize) -> NativeRuntimeStatusCode;

/// Pin one heap value against movement through the runtime.
pub type NativePin = unsafe extern "C" fn(
    context: *mut NativeContext,
    value: usize,
    result: *mut usize,
) -> NativeRuntimeStatusCode;

/// Release one pinned heap value through the runtime.
pub type NativeUnpin =
    unsafe extern "C" fn(context: *mut NativeContext, value: usize) -> NativeRuntimeStatusCode;

/// Record one managed reference write through the runtime.
pub type NativeWriteBarrier = unsafe extern "C" fn(
    context: *mut NativeContext,
    object: usize,
    offset: usize,
    byte_len: usize,
) -> NativeRuntimeStatusCode;
