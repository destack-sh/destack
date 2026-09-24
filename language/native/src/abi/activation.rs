use super::{DynamicTable, Exit, Runtime, StaticSpace, VirtualTable};

/// Native activation passed to generated code and runtime operations.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activation {
    /// The runtime owner for this call.
    pub call: *mut Call,
    /// Runtime operations callable by generated code.
    pub runtime: *const Runtime,
    /// Typed native body addresses keyed by encoded callable word.
    pub functions: *const usize,
    /// Virtual method rows keyed by Program virtual table id.
    pub virtuals: *const *const VirtualTable,
    /// Dynamic entry rows keyed by Program dynamic table id.
    pub dynamics: *const *const DynamicTable,
    /// The first byte in world memory.
    pub memory_base: *mut u8,
    /// The lowest stack address a generated prologue may reach.
    pub stack_limit: usize,
    /// Program constant bytes.
    pub constants: StaticSpace,
    /// Runtime-shared static bytes.
    pub shared_statics: StaticSpace,
    /// Worker-local static bytes.
    pub local_statics: StaticSpace,
    /// Current execution context reference.
    pub context: usize,
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
pub type Entry = unsafe extern "C-unwind" fn(
    activation: *mut Activation,
    arguments: *const u64,
    result: *mut u64,
);

impl Activation {
    /// Create one native activation.
    pub const fn new(
        call: *mut Call,
        runtime: *const Runtime,
        functions: *const usize,
        virtuals: *const *const VirtualTable,
        dynamics: *const *const DynamicTable,
        memory_base: *mut u8,
        stack_limit: usize,
        constants: StaticSpace,
        shared_statics: StaticSpace,
        local_statics: StaticSpace,
        context: usize,
        poll_request: *const u32,
        exit: *mut Exit,
    ) -> Self {
        Self {
            call,
            runtime,
            functions,
            virtuals,
            dynamics,
            memory_base,
            stack_limit,
            constants,
            shared_statics,
            local_statics,
            context,
            poll_request,
            exit,
        }
    }
}
