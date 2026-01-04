#![allow(elided_lifetimes_in_paths)]

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::Value;
use crate::{Frame, Interpreter};

/// Handler function for threaded dispatch.
///
/// Takes state, current block's instructions, and program counter.
/// Uses `become` to tail-call next handler, or returns `ControlFlow` for special cases.
pub(super) type ThreadedHandler =
    fn(&mut ThreadedState, &[ThreadedInstruction], usize) -> ControlFlow;

/// Control flow actions that exit the tail-call chain.
pub(super) enum ControlFlow {
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
pub(super) struct ThreadedInstruction {
    /// Handler function.
    pub handler: ThreadedHandler,
    /// Decoded data.
    pub data: ThreadedInstructionData,
}

/// Decoded instruction data.
#[derive(Clone)]
pub(super) enum ThreadedInstructionData {
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
        arguments: SmallVec<[mir::Value; 4]>,
        ordering: Option<mir::MemoryOrdering>,
    },

    /// Return from function.
    Return { value: Option<mir::Value> },

    /// Unconditional jump.
    Jump {
        target: usize,
        arguments: SmallVec<[mir::Value; 4]>,
    },

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

    /// Unsupported instruction or terminator.
    Unsupported { name: &'static str },
}

/// Switch case.
#[derive(Clone)]
pub(super) struct SwitchCase {
    /// Match value.
    pub value: i64,
    /// Target block.
    pub target: usize,
    /// Block arguments.
    pub arguments: SmallVec<[mir::Value; 4]>,
}

/// Threaded basic block.
#[derive(Clone)]
pub(super) struct ThreadedBlock {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Block parameters.
    pub parameters: SmallVec<[mir::Value; 4]>,
    /// Instructions including terminator.
    pub instructions: Vec<ThreadedInstruction>,
}

/// Threaded function with optimized dispatch.
#[derive(Clone)]
pub(super) struct ThreadedFunction {
    /// Function parameters.
    pub parameters: SmallVec<[mir::Value; 4]>,
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
pub(super) struct ThreadedState<'a> {
    /// Index of the current frame in the call stack.
    pub frame_index: usize,
    /// Interpreter reference for heap and globals.
    pub interpreter: &'a mut Interpreter,
    /// Pointer to the current frame for fast access.
    frame: *mut Frame,
    /// Pointer to SSA value storage.
    values: *mut Vec<Value>,
    /// Pointer to local variable storage.
    locals: *mut Vec<Value>,
}

impl<'a> ThreadedState<'a> {
    /// Create state for the current frame.
    pub(super) fn new(interpreter: &'a mut Interpreter, frame_index: usize) -> Self {
        // get frame pointer
        // safety: frame_index always points at the current frame
        let frame =
            unsafe { interpreter.call_stack.get_unchecked_mut(frame_index) as *mut super::Frame };

        // assemble state
        Self {
            frame_index,
            interpreter,
            frame,
            values: unsafe { &mut (*frame).values },
            locals: unsafe { &mut (*frame).locals },
        }
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub(super) fn current_frame_mut(&mut self) -> &mut super::Frame {
        // return current frame
        // safety: frame pointer is valid for current block execution
        unsafe { &mut *self.frame }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub(super) fn frame_by_index(&self, frame_index: usize) -> Result<&super::Frame, Error> {
        // look up frame by index
        self.interpreter
            .call_stack
            .get(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get a frame by index mutably.
    #[inline(always)]
    pub(super) fn frame_by_index_mut(
        &mut self,
        frame_index: usize,
    ) -> Result<&mut super::Frame, Error> {
        // look up frame by index
        self.interpreter
            .call_stack
            .get_mut(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub(super) fn get(&self, v: mir::Value) -> Value {
        // compute value index
        let index = v.0 as usize;
        let values = unsafe { &*self.values };

        // validate bounds in debug builds
        debug_assert!(index < values.len(), "ssa value out of bounds: {v:?}");

        // read value
        unsafe { *values.get_unchecked(index) }
    }

    /// Set value by SSA id.
    #[inline(always)]
    pub(super) fn set(&mut self, v: mir::Value, val: Value) {
        // compute value index
        let index = v.0 as usize;
        let values = unsafe { &mut *self.values };

        // validate bounds in debug builds
        debug_assert!(index < values.len(), "ssa value out of bounds: {v:?}");

        // write value
        unsafe {
            *values.get_unchecked_mut(index) = val;
        }
    }

    /// Get local variable.
    #[inline(always)]
    pub(super) fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Value {
        // compute local index
        let index = local.id as usize;
        let locals = unsafe { &*self.locals };

        // validate bounds in debug builds
        debug_assert!(index < locals.len(), "local out of bounds: {local:?}");

        // read local value
        unsafe { *locals.get_unchecked(index) }
    }

    /// Set local variable.
    #[inline(always)]
    pub(super) fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, val: Value) {
        // compute local index
        let index = local.id as usize;
        let locals = unsafe { &mut *self.locals };

        // validate bounds in debug builds
        debug_assert!(index < locals.len(), "local out of bounds: {local:?}");

        // write local value
        unsafe {
            *locals.get_unchecked_mut(index) = val;
        }
    }
}
