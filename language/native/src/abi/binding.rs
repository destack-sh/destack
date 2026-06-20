pub use destack_program::native::RuntimeBinding;

use crate::{
    NativeContext, NativeContinuation, NativeExitCode, NativeMaterialization, NativeTrapCode,
    NativeValue,
};

/// Allocate one typed heap value.
pub type NativeNew = unsafe extern "C" fn(context: *mut NativeContext, allocation: u32) -> usize;

/// Allocate one typed repeated heap value.
pub type NativeNewSlice =
    unsafe extern "C" fn(context: *mut NativeContext, allocation: u32, length: usize) -> usize;

/// Release one unique heap value.
pub type NativeFree = unsafe extern "C" fn(context: *mut NativeContext, value: usize);

/// Pin one heap value against movement.
pub type NativePin = unsafe extern "C" fn(context: *mut NativeContext, value: usize) -> usize;

/// Release one pinned heap value.
pub type NativeUnpin = unsafe extern "C" fn(context: *mut NativeContext, value: usize);

/// Record one managed reference write.
pub type NativeWriteBarrier =
    unsafe extern "C" fn(context: *mut NativeContext, destination: *mut u8, value: usize);

/// Cooperate with the runtime at one safepoint.
pub type NativeSafepoint = unsafe extern "C" fn(
    context: *mut NativeContext,
    safepoint: u32,
    continuation: NativeContinuation,
) -> NativeExitCode;

/// Suspend execution into the runtime scheduler.
pub type NativeYield = unsafe extern "C" fn(
    context: *mut NativeContext,
    value: NativeValue,
    continuation: NativeContinuation,
) -> NativeExitCode;

/// Deoptimize native execution into VM materialization.
pub type NativeDeopt = unsafe extern "C" fn(
    context: *mut NativeContext,
    safepoint: u32,
    materialization: NativeMaterialization,
) -> NativeExitCode;

/// Report one native trap.
pub type NativeTrapExit =
    unsafe extern "C" fn(context: *mut NativeContext, trap: NativeTrapCode) -> NativeExitCode;

/// Report one language panic.
pub type NativePanic =
    unsafe extern "C" fn(context: *mut NativeContext, payload: NativeValue) -> NativeExitCode;

/// Continue the active language unwind.
pub type NativeUnwindResume = unsafe extern "C" fn(context: *mut NativeContext) -> NativeExitCode;
