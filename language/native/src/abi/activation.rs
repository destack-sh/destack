use super::{ConstantSpace, Exit, ExitCode, Runtime, StaticSpace};

/// Native activation passed to generated code and runtime operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activation {
    /// The runtime owner for this call.
    pub call: *mut Call,
    /// Runtime operations callable by generated code.
    pub runtime: *const Runtime,
    /// Program constant bytes.
    pub constants: ConstantSpace,
    /// Runtime-shared static bytes.
    pub shared_statics: StaticSpace,
    /// Worker-local static bytes.
    pub local_statics: StaticSpace,
    /// Worker-local runtime poll request word.
    pub poll_request: *const u32,
    /// Exit record written before non-completion returns.
    pub exit: *mut Exit,
}

/// Opaque runtime owner for one active native call.
#[repr(C)]
#[derive(Debug)]
pub struct Call {
    /// Prevent external construction.
    _private: [u8; 0],
}

/// Native entry function.
pub type Entry = unsafe extern "C" fn(
    activation: *mut Activation,
    arguments: *const u64,
    result: *mut u64,
) -> ExitCode;

impl Activation {
    /// Create one native activation.
    pub const fn new(
        call: *mut Call,
        runtime: *const Runtime,
        constants: ConstantSpace,
        shared_statics: StaticSpace,
        local_statics: StaticSpace,
        poll_request: *const u32,
        exit: *mut Exit,
    ) -> Self {
        Self {
            call,
            runtime,
            constants,
            shared_statics,
            local_statics,
            poll_request,
            exit,
        }
    }
}
