use std::ffi::c_void;

use super::{NativeContinuation, NativeMaterialization, NativeTrapCode, NativeValue};
use crate::{NativeConstantSpace, NativeStaticSpace};

/// Native call context passed to generated native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContext {
    /// Opaque runtime-owned native call state.
    pub state: *mut c_void,
    /// Program constant bytes.
    pub constants: NativeConstantSpace,
    /// Runtime-shared static bytes.
    pub shared_statics: NativeStaticSpace,
    /// Worker-local static bytes.
    pub local_statics: NativeStaticSpace,
    /// Exit record written before non-completion returns.
    pub exit: *mut NativeExit,
}

impl NativeContext {
    /// Create one native call context.
    pub const fn new(
        state: *mut c_void,
        constants: NativeConstantSpace,
        shared_statics: NativeStaticSpace,
        local_statics: NativeStaticSpace,
        exit: *mut NativeExit,
    ) -> Self {
        Self {
            state,
            constants,
            shared_statics,
            local_statics,
            exit,
        }
    }
}

/// Native non-completion exit payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeExit {
    /// Safepoint associated with yield or deoptimization.
    pub safepoint: u32,
    /// Native continuation associated with yield exits.
    pub continuation: NativeContinuation,
    /// Materialized continuation associated with deoptimization.
    pub materialization: NativeMaterialization,
    /// Trap code associated with trap exits.
    pub trap: NativeTrapCode,
    /// Payload associated with language panic exits.
    pub payload: NativeValue,
}

impl Default for NativeExit {
    fn default() -> Self {
        Self {
            safepoint: 0,
            continuation: NativeContinuation::empty(),
            materialization: NativeMaterialization::empty(),
            trap: 0,
            payload: NativeValue::VOID,
        }
    }
}
