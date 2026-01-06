#![allow(elided_lifetimes_in_paths)]

use std::fmt;

use destack_mir as mir;

use crate::diagnostic::Error;
use crate::memory::{ReferenceMeta, Value};
use crate::{Frame, Interpreter};

/// Handler function for threaded dispatch.
///
/// Takes state, current block's instructions, and program counter.
/// Uses `become` to tail-call next handler, or returns `ControlFlow` for special cases.
pub type ThreadedHandler = fn(&mut ThreadedState, &[ThreadedInstruction], usize) -> ControlFlow;

/// Argument range within the threaded function argument pool.
#[derive(Clone, Copy, Debug)]
pub struct ArgumentRange {
    /// Start offset into the argument pool.
    pub start: u32,
    /// Number of arguments in the range.
    pub len: u32,
}

impl ArgumentRange {
    /// Create an empty argument range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice arguments from the pool for this range.
    pub fn slice<'a>(&self, pool: &'a [mir::Value]) -> &'a [mir::Value] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "argument pool out of bounds for range"
        );

        // return argument slice
        &pool[start..start + len]
    }
}

/// Switch case range within the threaded function switch pool.
#[derive(Clone, Copy, Debug)]
pub struct SwitchRange {
    /// Start offset into the switch case pool.
    pub start: u32,
    /// Number of cases in the range.
    pub len: u32,
}

impl SwitchRange {
    /// Create an empty switch range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice cases from the pool for this range.
    pub fn slice<'a>(&self, pool: &'a [SwitchCase]) -> &'a [SwitchCase] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "switch case pool out of bounds for range"
        );

        // return case slice
        &pool[start..start + len]
    }
}

/// Copy range within the threaded function copy pool.
#[derive(Clone, Copy, Debug)]
pub struct CopyRange {
    /// Start offset into the copy pool.
    pub start: u32,
    /// Number of pairs in the range.
    pub len: u32,
}

impl CopyRange {
    /// Create an empty copy range.
    pub const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice pairs from the pool for this range.
    pub fn slice<'a>(&self, pool: &'a [CopyPair]) -> &'a [CopyPair] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "copy pool out of bounds for range"
        );

        // return copy slice
        &pool[start..start + len]
    }
}

/// Sentinel value id used for optional destinations.
pub(crate) const INVALID_VALUE_ID: u32 = u32::MAX;

/// Sentinel function index used for missing threaded entries.
pub(crate) const INVALID_FUNCTION_INDEX: u32 = u32::MAX;

/// Sentinel field count for unknown aggregate layouts.
pub(super) const UNKNOWN_FIELD_COUNT: u32 = u32::MAX;

/// Sentinel array length for unknown layouts.
pub(super) const UNKNOWN_ARRAY_LENGTH: u64 = u64::MAX;

/// Pack an optional SSA value into a sentinel encoding.
pub(crate) fn pack_optional_value(value: Option<mir::Value>) -> mir::Value {
    value.unwrap_or(mir::Value(INVALID_VALUE_ID))
}

/// Check if a packed SSA value is the sentinel.
pub(crate) fn is_invalid_value(value: mir::Value) -> bool {
    value.0 == INVALID_VALUE_ID
}

/// Control flow actions that exit the tail-call chain.
#[derive(Debug)]
pub enum ControlFlow {
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: u32,
        /// Copy plan for block parameters.
        copies: CopyRange,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: u32,
        /// Threaded function index when available.
        callee_index: u32,
        /// Destination for return value.
        destination: mir::Value,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
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
        to_type: u32,
    },

    /// Function call.
    Call {
        dest: mir::Value,
        function: u32,
        callee_index: u32,
        arguments: ArgumentRange,
        copies: CopyRange,
    },

    /// Indirect function call.
    CallIndirect {
        dest: mir::Value,
        callee: mir::Value,
        arguments: ArgumentRange,
    },

    /// Load local variable.
    LocalGet { dest: mir::Value, local: u32 },

    /// Store local variable.
    LocalSet { local: u32, value: mir::Value },

    /// Get global address.
    GlobalAddr {
        dest: mir::Value,
        global: u32,
        reference: ReferenceMeta,
    },

    /// Load global constant.
    GlobalConst { dest: mir::Value, global: u32 },

    /// Load from pointer.
    Load {
        dest: mir::Value,
        pointer: mir::Value,
    },

    /// Store to pointer.
    Store {
        pointer: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
    },

    /// Get struct/tuple field.
    FieldGet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
    },

    /// Get struct/tuple field address.
    FieldAddr {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        reference: ReferenceMeta,
        field_count: u32,
    },

    /// Load a field through field.addr + load.
    FieldLoad {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        field_count: u32,
    },

    /// Set struct/tuple field.
    FieldSet {
        dest: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    },

    /// Store a field through field.addr + store.
    FieldStore {
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
        reference: ReferenceMeta,
        field_count: u32,
    },

    /// Get array element.
    ElementGet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
    },

    /// Get array element address.
    ElementAddr {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
    },

    /// Load an element through element.addr + load.
    ElementLoad {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        array_length: u64,
    },

    /// Set array element.
    ElementSet {
        dest: mir::Value,
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
    },

    /// Store an element through element.addr + store.
    ElementStore {
        array: mir::Value,
        index: mir::Value,
        value: mir::Value,
        reference: ReferenceMeta,
        array_length: u64,
    },

    /// Allocate managed memory.
    ManagedAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
    },

    /// Allocate managed array.
    ManagedAllocArray {
        dest: mir::Value,
        length: mir::Value,
        reference: ReferenceMeta,
    },

    /// Allocate raw memory.
    RawAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
    },

    /// Free raw memory.
    RawFree { pointer: mir::Value },

    /// Allocate stack memory.
    StackAlloc {
        dest: mir::Value,
        reference: ReferenceMeta,
    },

    /// Intrinsic call.
    Intrinsic {
        dest: mir::Value,
        intrinsic: mir::Intrinsic,
        arguments: ArgumentRange,
        ordering: Option<mir::MemoryOrdering>,
    },

    /// Return from function.
    Return { value: mir::Value },

    /// Unconditional jump.
    Jump { target: u32, copies: CopyRange },

    /// Conditional branch.
    Branch {
        condition: mir::Value,
        then_target: u32,
        then_copies: CopyRange,
        else_target: u32,
        else_copies: CopyRange,
    },

    /// Switch on integer.
    Switch {
        value: mir::Value,
        cases: SwitchRange,
        default_target: u32,
        default_copies: CopyRange,
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
    pub target: u32,
    /// Block parameter copies.
    pub copies: CopyRange,
}

/// Threaded basic block.
#[derive(Clone, Debug)]
pub struct ThreadedBlock {
    /// Original MIR block id.
    pub mir_block: mir::LocalNodeId<mir::Block>,
    /// Block parameters.
    pub parameters: ArgumentRange,
    /// Instructions including terminator.
    pub instructions: Vec<ThreadedInstruction>,
}

/// Threaded function with optimized dispatch.
#[derive(Clone, Debug)]
pub struct ThreadedFunction {
    /// Function parameters.
    pub parameters: ArgumentRange,
    /// Entry block index.
    pub entry: u32,
    /// All blocks.
    pub blocks: Vec<ThreadedBlock>,
    /// Pool of argument values referenced by ranges.
    pub argument_pool: Vec<mir::Value>,
    /// Pool of switch cases referenced by ranges.
    pub switch_case_pool: Vec<SwitchCase>,
    /// Pool of value copy pairs referenced by ranges.
    pub copy_pool: Vec<CopyPair>,
    /// Count of SSA values used by the function.
    pub value_count: usize,
    /// Count of local variables used by the function.
    pub local_count: usize,
}

/// Copy pair for parameter binding.
#[derive(Clone, Copy, Debug)]
pub struct CopyPair {
    /// Destination SSA value id.
    pub dest: u32,
    /// Source SSA value id (sentinel for missing).
    pub src: u32,
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
    /// Argument pool for the current function.
    argument_pool: *const mir::Value,
    /// Argument pool length.
    argument_pool_len: usize,
    /// Switch case pool for the current function.
    switch_case_pool: *const SwitchCase,
    /// Switch case pool length.
    switch_case_pool_len: usize,
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
            .field("argument_pool_len", &self.argument_pool_len)
            .field("switch_case_pool_len", &self.switch_case_pool_len)
            .finish()
    }
}

impl<'a> ThreadedState<'a> {
    /// Create state for the current frame.
    pub fn new(
        interpreter: &'a mut Interpreter,
        frame_index: usize,
        argument_pool: &[mir::Value],
        switch_case_pool: &[SwitchCase],
    ) -> Self {
        // get frame pointer
        // #Safety: frame_index always points at the current frame
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
            argument_pool: argument_pool.as_ptr(),
            argument_pool_len: argument_pool.len(),
            switch_case_pool: switch_case_pool.as_ptr(),
            switch_case_pool_len: switch_case_pool.len(),
        }
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub fn current_frame_mut(&mut self) -> &mut super::Frame {
        // #Safety: frame pointer is valid for current block execution
        unsafe { &mut *self.frame }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub fn frame_by_index(&self, frame_index: usize) -> Result<&super::Frame, Error> {
        self.interpreter
            .call_stack
            .get(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get a frame by index mutably.
    #[inline(always)]
    pub fn frame_by_index_mut(&mut self, frame_index: usize) -> Result<&mut super::Frame, Error> {
        self.interpreter
            .call_stack
            .get_mut(frame_index)
            .ok_or(Error::InvalidHeapHandle)
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub fn get(&self, v: mir::Value) -> Value {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe { *self.values.add(index) }
    }

    /// Set value by SSA id.
    #[inline(always)]
    pub fn set(&mut self, v: mir::Value, val: Value) {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe {
            *self.values.add(index) = val;
        }
    }

    /// Get local variable.
    #[inline(always)]
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Value {
        let index = local.id as usize;
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");
        unsafe { *self.locals.add(index) }
    }

    /// Set local variable.
    #[inline(always)]
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, val: Value) {
        let index = local.id as usize;
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");
        unsafe {
            *self.locals.add(index) = val;
        }
    }

    /// Get the argument slice for the given range.
    #[inline(always)]
    pub fn argument_slice(&self, range: ArgumentRange) -> &[mir::Value] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.argument_pool_len,
            "argument pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.argument_pool.add(start), len) }
    }

    /// Get the switch case slice for the given range.
    #[inline(always)]
    pub fn switch_cases(&self, range: SwitchRange) -> &[SwitchCase] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.switch_case_pool_len,
            "switch case pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.switch_case_pool.add(start), len) }
    }
}
