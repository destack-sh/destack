use destack_engine::RuntimeContext;

use crate::{NativeTrapCode, NativeValue};

/// Native call context passed to generated native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContext {
    /// Opaque runtime context for helper calls.
    pub runtime: *mut RuntimeContext,
    /// Exit record written before non-completion returns.
    pub exit: *mut NativeExit,
}

impl NativeContext {
    /// Create one native call context.
    pub const fn new(runtime: *mut RuntimeContext, exit: *mut NativeExit) -> Self {
        Self { runtime, exit }
    }
}

/// Native non-completion exit payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeExit {
    /// Safepoint associated with yield or deoptimization.
    pub safepoint: u32,
    /// Trap code associated with trap exits.
    pub trap: NativeTrapCode,
    /// Payload associated with language panic exits.
    pub payload: NativeValue,
}

impl NativeExit {
    /// Empty native exit payload.
    pub const EMPTY: Self = Self {
        safepoint: 0,
        trap: 0,
        payload: NativeValue::VOID,
    };
}

impl Default for NativeExit {
    fn default() -> Self {
        Self::EMPTY
    }
}
