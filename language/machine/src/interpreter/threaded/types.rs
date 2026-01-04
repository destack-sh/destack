#![allow(elided_lifetimes_in_paths)]

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::Value;

/// Handler function for threaded dispatch.
///
/// Takes state, current block's instructions, and program counter.
/// Uses `become` to tail-call next handler, or returns `ControlFlow` for special cases.
pub type Handler = fn(&mut ThreadedState, &[ThreadedInst], usize) -> ControlFlow;

/// Control flow actions that exit the tail-call chain.
pub enum ControlFlow {
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: usize,
        /// Arguments for block parameters.
        arguments: SmallVec<[Value; 4]>,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: mir::LocalNodeId<mir::Function>,
        /// Destination for return value.
        destination: Option<mir::Value>,
        /// Arguments to pass.
        arguments: SmallVec<[Value; 4]>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Return from current function.
    Return(Value),
    /// Runtime error.
    Error(Error),
}

/// Pre-decoded instruction with handler pointer.
#[derive(Clone)]
pub struct ThreadedInst {
    /// Handler function.
    pub handler: Handler,
    /// Decoded data.
    pub data: InstData,
}

/// Decoded instruction data.
#[derive(Clone)]
pub enum InstData {
    /// Load constant.
    Const { dest: mir::Value, value: Value },

    /// Binary operation.
    Binary {
        dest: mir::Value,
        op: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    },

    /// Unary operation.
    Unary {
        dest: mir::Value,
        op: mir::UnaryOperator,
        arg: mir::Value,
    },

    /// Type cast.
    Cast {
        dest: mir::Value,
        op: mir::CastOperator,
        arg: mir::Value,
        to_type: mir::LocalNodeId<mir::Type>,
    },

    /// Function call.
    Call {
        dest: Option<mir::Value>,
        function: mir::LocalNodeId<mir::Function>,
        arguments: SmallVec<[mir::Value; 4]>,
    },

    /// Indirect function call.
    CallIndirect {
        dest: Option<mir::Value>,
        callee: mir::Value,
        arguments: SmallVec<[mir::Value; 4]>,
    },

    /// Load local variable.
    LocalGet {
        dest: mir::Value,
        local: mir::LocalNodeId<mir::Local>,
    },

    /// Store local variable.
    LocalSet { local: mir::LocalNodeId<mir::Local>, value: mir::Value },

    /// Get global address.
    GlobalAddr {
        dest: mir::Value,
        global: mir::LocalNodeId<mir::Global>,
    },

    /// Load global constant.
    GlobalConst {
        dest: mir::Value,
        global: mir::LocalNodeId<mir::Global>,
    },

    /// Load from pointer.
    Load { dest: mir::Value, pointer: mir::Value },

    /// Store to pointer.
    Store { pointer: mir::Value, value: mir::Value },

    /// Get struct/tuple field.
    FieldGet { dest: mir::Value, aggregate: mir::Value, index: u32 },

    /// Set struct/tuple field.
    FieldSet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    },

    /// Get array element.
    ElementGet { dest: mir::Value, array: mir::Value, index: mir::Value },

    /// Set array element.
    ElementSet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
    },

    /// Allocate managed memory.
    ManagedAlloc { dest: mir::Value },

    /// Allocate managed array.
    ManagedAllocArray { dest: mir::Value, length: mir::Value },

    /// Allocate raw memory.
    RawAlloc { dest: mir::Value },

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Allocate stack memory.
    StackAlloc { dest: mir::Value },

    /// Intrinsic call.
    Intrinsic {
        dest: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: SmallVec<[mir::Value; 4]>,
        ordering: Option<mir::MemoryOrdering>,
    },

    /// Return from function.
    Return { value: Option<mir::Value> },

    /// Unconditional jump.
    Jump { target: usize, arguments: SmallVec<[mir::Value; 4]> },

    /// Conditional branch.
    Branch {
        condition: mir::Value,
        then_target: usize,
        then_arguments: SmallVec<[mir::Value; 4]>,
        else_target: usize,
        else_arguments: SmallVec<[mir::Value; 4]>,
    },

    /// Switch on integer.
    Switch {
        value: mir::Value,
        cases: Vec<SwitchCase>,
        default_target: usize,
        default_arguments: SmallVec<[mir::Value; 4]>,
    },

    /// Unreachable code.
    Unreachable,
}

/// Switch case.
#[derive(Clone)]
pub struct SwitchCase {
    /// Match value.
    pub value: i64,
    /// Target block.
    pub target: usize,
    /// Block arguments.
    pub arguments: SmallVec<[mir::Value; 4]>,
}

/// Threaded basic block.
#[derive(Clone)]
pub struct ThreadedBlock {
    /// Block parameters.
    pub parameters: SmallVec<[mir::Value; 4]>,
    /// Instructions including terminator.
    pub instructions: Vec<ThreadedInst>,
}

/// Threaded function.
#[derive(Clone)]
pub struct ThreadedFunction {
    /// Function parameters.
    pub parameters: SmallVec<[mir::Value; 4]>,
    /// Entry block index.
    pub entry: usize,
    /// All blocks.
    pub blocks: Vec<ThreadedBlock>,
    /// Original MIR function.
    pub mir_function: mir::LocalNodeId<mir::Function>,
}

/// Execution state for threaded interpreter.
pub struct ThreadedState<'a> {
    /// SSA value storage.
    pub values: &'a mut Vec<Value>,
    /// Local variable storage.
    pub locals: &'a mut Vec<Value>,
    /// Interpreter reference for heap/globals.
    pub interp: &'a mut crate::Interpreter,
}

impl<'a> ThreadedState<'a> {
    /// Get value by SSA id.
    #[inline]
    pub fn get(&self, v: mir::Value) -> Value {
        self.values.get(v.0 as usize).copied().unwrap_or(Value::VOID)
    }

    /// Set value by SSA id.
    #[inline]
    pub fn set(&mut self, v: mir::Value, val: Value) {
        let idx = v.0 as usize;
        if idx >= self.values.len() {
            self.values.resize(idx + 1, Value::VOID);
        }
        self.values[idx] = val;
    }

    /// Get local variable.
    #[inline]
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Value {
        self.locals.get(local.id as usize).copied().unwrap_or(Value::VOID)
    }

    /// Set local variable.
    #[inline]
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, val: Value) {
        let idx = local.id as usize;
        if idx >= self.locals.len() {
            self.locals.resize(idx + 1, Value::VOID);
        }
        self.locals[idx] = val;
    }
}
