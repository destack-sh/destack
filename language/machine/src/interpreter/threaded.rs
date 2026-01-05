#![allow(elided_lifetimes_in_paths)]

use std::fmt;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::Value;
use crate::{Frame, Interpreter};

/// Handler function for threaded dispatch.
///
/// Takes state, current block's instructions, and program counter.
/// Uses `become` to tail-call next handler, or returns `ControlFlow` for special cases.
pub type ThreadedHandler = fn(&mut ThreadedState, &[ThreadedInstruction], usize) -> ControlFlow;

/// Control flow actions that exit the tail-call chain.
#[derive(Debug)]
pub enum ControlFlow {
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: usize,
        /// Arguments for block parameters (SSA value ids).
        arguments: SmallVec<[mir::Value; 8]>,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: mir::LocalNodeId<mir::Function>,
        /// Destination for return value.
        destination: Option<mir::Value>,
        /// Arguments to pass (SSA value ids).
        arguments: SmallVec<[mir::Value; 8]>,
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
pub struct ThreadedInstruction {
    /// Handler function.
    pub handler: ThreadedHandler,
    /// Decoded data.
    pub data: ThreadedInstructionData,
}

/// Decoded instruction data.
#[derive(Clone, Debug)]
pub enum ThreadedInstructionData {
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
        arguments: SmallVec<[mir::Value; 8]>,
    },

    /// Indirect function call.
    CallIndirect {
        dest: Option<mir::Value>,
        callee: mir::Value,
        arguments: SmallVec<[mir::Value; 8]>,
    },

    /// Load local variable.
    LocalGet {
        dest: mir::Value,
        local: mir::LocalNodeId<mir::Local>,
    },

    /// Store local variable.
    LocalSet {
        local: mir::LocalNodeId<mir::Local>,
        value: mir::Value,
    },

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
    Load {
        dest: mir::Value,
        pointer: mir::Value,
    },

    /// Store to pointer.
    Store {
        pointer: mir::Value,
        value: mir::Value,
    },

    /// Get struct/tuple field.
    FieldGet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
    },

    /// Set struct/tuple field.
    FieldSet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    },

    /// Get array element.
    ElementGet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
    },

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
    ManagedAllocArray {
        dest: mir::Value,
        length: mir::Value,
    },

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
        arguments: SmallVec<[mir::Value; 8]>,
        ordering: Option<mir::MemoryOrdering>,
    },

    /// Return from function.
    Return { value: Option<mir::Value> },

    /// Unconditional jump.
    Jump {
        target: usize,
        arguments: SmallVec<[mir::Value; 8]>,
    },

    /// Conditional branch.
    Branch {
        condition: mir::Value,
        then_target: usize,
        then_arguments: SmallVec<[mir::Value; 8]>,
        else_target: usize,
        else_arguments: SmallVec<[mir::Value; 8]>,
    },

    /// Switch on integer.
    Switch {
        value: mir::Value,
        cases: Vec<SwitchCase>,
        default_target: usize,
        default_arguments: SmallVec<[mir::Value; 8]>,
    },

    /// Unreachable code.
    Unreachable,

    /// Unsupported instruction or terminator.
    Unsupported { name: &'static str },
}

/// Switch case.
#[derive(Clone, Debug)]
pub struct SwitchCase {
    /// Match value.
    pub value: i64,
    /// Target block.
    pub target: usize,
    /// Block arguments.
    pub arguments: SmallVec<[mir::Value; 8]>,
}

/// Threaded basic block.
#[derive(Clone, Debug)]
pub struct ThreadedBlock {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Block parameters.
    pub parameters: SmallVec<[mir::Value; 8]>,
    /// Instructions including terminator.
    pub instructions: Vec<ThreadedInstruction>,
}

/// Threaded function with optimized dispatch.
#[derive(Clone, Debug)]
pub struct ThreadedFunction {
    /// Function parameters.
    pub parameters: SmallVec<[mir::Value; 8]>,
    /// Entry block index.
    pub entry: usize,
    /// All blocks.
    pub blocks: Vec<ThreadedBlock>,
    /// Count of SSA values used by the function.
    pub value_count: usize,
    /// Count of local variables used by the function.
    pub local_count: usize,
}

/// Execution state for threaded interpreter.
pub struct ThreadedState<'a> {
    /// Index of the current frame in the call stack.
    pub frame_index: usize,
    /// Interpreter reference for heap and globals.
    pub interpreter: &'a mut Interpreter,
    /// Pointer to the current frame for fast access.
    frame: *mut Frame,
    /// Pointer to SSA value storage for this frame.
    values: *mut Value,
    /// Count of SSA values in this frame.
    value_count: usize,
    /// Pointer to local variable storage for this frame.
    locals: *mut Value,
    /// Count of local variables in this frame.
    local_count: usize,
}

impl fmt::Debug for ThreadedInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadedInstruction")
            .field("data", &self.data)
            .finish()
    }
}

impl fmt::Debug for ThreadedState<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThreadedState")
            .field("frame_index", &self.frame_index)
            .field("value_count", &self.value_count)
            .field("local_count", &self.local_count)
            .finish()
    }
}

impl<'a> ThreadedState<'a> {
    /// Create state for the current frame.
    pub fn new(interpreter: &'a mut Interpreter, frame_index: usize) -> Self {
        // get frame pointer
        // safety: frame_index always points at the current frame
        let frame =
            unsafe { interpreter.call_stack.get_unchecked_mut(frame_index) as *mut super::Frame };

        // load frame bounds
        let value_base = unsafe { (*frame).value_base };
        let value_count = unsafe { (*frame).value_count };
        let local_base = unsafe { (*frame).local_base };
        let local_count = unsafe { (*frame).local_count };

        // validate stack bounds in debug builds
        debug_assert!(
            value_base + value_count <= interpreter.value_stack.len(),
            "value stack out of bounds for frame"
        );
        debug_assert!(
            local_base + local_count <= interpreter.local_stack.len(),
            "local stack out of bounds for frame"
        );

        // cache stack pointers
        let values_ptr = interpreter.value_stack.as_mut_ptr();
        let locals_ptr = interpreter.local_stack.as_mut_ptr();

        // assemble state
        Self {
            frame_index,
            interpreter,
            frame,
            values: unsafe { values_ptr.add(value_base) },
            value_count,
            locals: unsafe { locals_ptr.add(local_base) },
            local_count,
        }
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub fn current_frame_mut(&mut self) -> &mut super::Frame {
        // return current frame
        // safety: frame pointer is valid for current block execution
        unsafe { &mut *self.frame }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub fn frame_by_index(&self, frame_index: usize) -> Result<&super::Frame, Error> {
        // look up frame by index
        self.interpreter
            .call_stack
            .get(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get a frame by index mutably.
    #[inline(always)]
    pub fn frame_by_index_mut(&mut self, frame_index: usize) -> Result<&mut super::Frame, Error> {
        // look up frame by index
        self.interpreter
            .call_stack
            .get_mut(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub fn get(&self, v: mir::Value) -> Value {
        // compute value index
        let index = v.0 as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");

        // read value
        unsafe { *self.values.add(index) }
    }

    /// Set value by SSA id.
    #[inline(always)]
    pub fn set(&mut self, v: mir::Value, val: Value) {
        // compute value index
        let index = v.0 as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");

        // write value
        unsafe {
            *self.values.add(index) = val;
        }
    }

    /// Get local variable.
    #[inline(always)]
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Value {
        // compute local index
        let index = local.id as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");

        // read local value
        unsafe { *self.locals.add(index) }
    }

    /// Set local variable.
    #[inline(always)]
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, val: Value) {
        // compute local index
        let index = local.id as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");

        // write local value
        unsafe {
            *self.locals.add(index) = val;
        }
    }
}
