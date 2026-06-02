use destack_engine::HostCall;

use crate::{NativeTrapCode, NativeValue};

/// Native call context passed to generated native code.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContext {
    /// Opaque host call state for helper calls.
    pub host: *mut HostCall,
    /// Exit record written before non-completion returns.
    pub exit: *mut NativeExit,
}

impl NativeContext {
    /// Create one native call context.
    pub const fn new(host: *mut HostCall, exit: *mut NativeExit) -> Self {
        Self { host, exit }
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

impl Default for NativeExit {
    fn default() -> Self {
        Self {
            safepoint: 0,
            trap: 0,
            payload: NativeValue::VOID,
        }
    }
}
