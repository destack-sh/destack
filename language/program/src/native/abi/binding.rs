use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{NativeContext, NativeContinuation, NativeExitCode, NativeTrapCode, NativeValue};

/// Native runtime service status code.
pub type NativeRuntimeStatusCode = u32;

/// Native allocation initialization mode.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeAllocationInitialization {
    /// Initialize bytes to zero.
    Zeroed = 0,
    /// Leave bytes uninitialized.
    Uninit = 1,
}

/// Native runtime service status.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeRuntimeStatus {
    /// Native execution may continue.
    Continue = 0,
    /// The service failed normally and native code should take its failure edge.
    Failed = 1,
    /// Native execution must return the exit kind stored in the context.
    Exit = 2,
}

impl NativeRuntimeStatus {
    /// Return the native runtime status code.
    pub const fn code(self) -> NativeRuntimeStatusCode {
        self as NativeRuntimeStatusCode
    }
}

/// Native runtime status code conversion error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRuntimeStatusError {
    /// The invalid status code.
    pub code: NativeRuntimeStatusCode,
}

impl fmt::Display for NativeRuntimeStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid native runtime status code {}",
            self.code
        )
    }
}

impl Error for NativeRuntimeStatusError {}

impl TryFrom<NativeRuntimeStatusCode> for NativeRuntimeStatus {
    type Error = NativeRuntimeStatusError;

    fn try_from(code: NativeRuntimeStatusCode) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(Self::Continue),
            1 => Ok(Self::Failed),
            2 => Ok(Self::Exit),
            code => Err(NativeRuntimeStatusError { code }),
        }
    }
}

/// Allocate one heap object through the runtime.
pub type NativeAllocate = unsafe extern "C" fn(
    context: *mut NativeContext,
    allocation_plan: u32,
    initialization: NativeAllocationInitialization,
    out: *mut usize,
) -> NativeRuntimeStatusCode;

/// Allocate one repeated heap backing through the runtime.
pub type NativeAllocateSlice = unsafe extern "C" fn(
    context: *mut NativeContext,
    element_allocation_plan: u32,
    length: usize,
    initialization: NativeAllocationInitialization,
    out: *mut usize,
) -> NativeRuntimeStatusCode;

/// Release one unique heap value through the runtime.
pub type NativeFree =
    unsafe extern "C" fn(context: *mut NativeContext, value: usize) -> NativeRuntimeStatusCode;

/// Pin one heap value against movement through the runtime.
pub type NativePin = unsafe extern "C" fn(
    context: *mut NativeContext,
    value: usize,
    out: *mut usize,
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

/// Cooperate with the runtime at one native safepoint.
pub type NativeSafepoint = unsafe extern "C" fn(
    context: *mut NativeContext,
    safepoint: u32,
    continuation: NativeContinuation,
) -> NativeRuntimeStatusCode;

/// Suspend execution into the runtime scheduler.
pub type NativeYield = unsafe extern "C" fn(
    context: *mut NativeContext,
    value: NativeValue,
    continuation: NativeContinuation,
) -> NativeExitCode;

/// Stop execution for host inspection.
pub type NativeStop = unsafe extern "C" fn(
    context: *mut NativeContext,
    safepoint: u32,
    continuation: NativeContinuation,
) -> NativeExitCode;

/// Deoptimize native execution into continuation state.
pub type NativeDeopt = unsafe extern "C" fn(
    context: *mut NativeContext,
    safepoint: u32,
    continuation: NativeContinuation,
) -> NativeExitCode;

/// Report one native trap.
pub type NativeTrapExit =
    unsafe extern "C" fn(context: *mut NativeContext, trap: NativeTrapCode) -> NativeExitCode;

/// Report one language panic.
pub type NativePanic =
    unsafe extern "C" fn(context: *mut NativeContext, payload: NativeValue) -> NativeExitCode;

/// Continue the active language unwind.
pub type NativeUnwindResume = unsafe extern "C" fn(context: *mut NativeContext) -> NativeExitCode;
