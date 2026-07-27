use crate::{NativeConstantSpace, NativeStaticSpace, Word};

use super::{NativeExit, NativeExitCode};

/// Opaque runtime owner for one active native call.
#[repr(C)]
#[derive(Debug)]
pub struct NativeCall {
    /// Prevent external construction.
    _private: [u8; 0],
}

/// Native call context passed to generated code and runtime operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeContext {
    /// The runtime owner for this call.
    pub call: *mut NativeCall,
    /// Program constant bytes.
    pub constants: NativeConstantSpace,
    /// Runtime-shared static bytes.
    pub shared_statics: NativeStaticSpace,
    /// Worker-local static bytes.
    pub local_statics: NativeStaticSpace,
    /// Exit record written before non-completion returns.
    pub exit: *mut NativeExit,
}

/// Native entry function.
pub type NativeEntry = unsafe extern "C" fn(
    context: *mut NativeContext,
    arguments: *const Word,
    result: *mut Word,
) -> NativeExitCode;

impl NativeContext {
    /// Create one native call context.
    pub const fn new(
        call: *mut NativeCall,
        constants: NativeConstantSpace,
        shared_statics: NativeStaticSpace,
        local_statics: NativeStaticSpace,
        exit: *mut NativeExit,
    ) -> Self {
        Self {
            call,
            constants,
            shared_statics,
            local_statics,
            exit,
        }
    }
}
