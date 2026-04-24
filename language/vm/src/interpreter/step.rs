use std::fmt;

use destack_heap::{Heap, HeapError, HeapReference, HeapResult, SharedHeapReference};
use destack_mir as mir;

use super::{Frame, Interpreter};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::isolate::GlobalStorage;
use crate::module::{
    ArgumentRange, Function, Instruction, Layout, Module, SwitchCase, SwitchRange,
};
use crate::options::IsolateOptions;
use crate::{FrameInfo, SharedHeap, Value};

/// Step state for one interpreter instruction step.
pub(crate) struct StepState<'ctx, 'iso> {
    /// Immutable module metadata for this step.
    pub(crate) module: &'iso Module,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable global variable storage.
    pub(crate) globals: &'iso mut GlobalStorage,
    /// The worker-local heap.
    heap: *mut Heap,
    /// The world-shared heap.
    shared: *const SharedHeap,
    /// Interpreter engine state for this step.
    pub(crate) engine: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Whether bounds checks are enabled for this step.
    pub bounds_checks: bool,
    /// Whether null checks are enabled for this step.
    pub null_checks: bool,
    /// Whether to collect execution statistics for this step.
    pub collect_stats: bool,

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

impl fmt::Debug for StepState<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepState")
            .field("frame_index", &self.frame_index)
            .field("value_count", &self.value_count)
            .field("local_count", &self.local_count)
            .field("argument_pool_len", &self.argument_pool_len)
            .field("switch_case_pool_len", &self.switch_case_pool_len)
            .field("bounds_checks", &self.bounds_checks)
            .field("null_checks", &self.null_checks)
            .field("collect_stats", &self.collect_stats)
            .finish()
    }
}

impl<'ctx, 'iso> StepState<'ctx, 'iso> {
    /// Create step state for the current frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        module: &'iso Module,
        options: &'iso IsolateOptions,
        globals: &'iso mut GlobalStorage,
        heap: &'iso mut Heap,
        shared: &'iso SharedHeap,
        engine: &'ctx mut Interpreter,
        frame_index: usize,
        argument_pool: &[mir::Value],
        switch_case_pool: &[SwitchCase],
    ) -> Self {
        // resolve check policies
        let mode = options.execution.mode;
        let bounds_checks = options.checks.bounds.is_enabled_for(mode);
        let null_checks = options.checks.null.is_enabled_for(mode);
        let collect_stats = options.telemetry.collect_stats;

        // get frame pointer
        // #Safety: frame_index always points at the current frame
        let frame = unsafe { engine.stack.get_unchecked_mut(frame_index) as *mut Frame };

        // load frame bounds
        let value_count = unsafe { (*frame).value_count };
        let local_count = unsafe { (*frame).local_count };

        // cache stack pointers
        let values_ptr = unsafe { (*frame).slots_mut_ptr() };
        let locals_ptr = unsafe { values_ptr.add(value_count) };

        Self {
            module,
            options,
            globals,
            heap: heap as *mut Heap,
            shared: shared as *const SharedHeap,
            engine,
            frame_index,
            bounds_checks,
            null_checks,
            collect_stats,
            frame,
            values: values_ptr,
            value_count,
            locals: locals_ptr,
            local_count,
            argument_pool: argument_pool.as_ptr(),
            argument_pool_len: argument_pool.len(),
            switch_case_pool: switch_case_pool.as_ptr(),
            switch_case_pool_len: switch_case_pool.len(),
        }
    }

    /// Borrow the module MIR tree.
    #[inline]
    pub(crate) fn tree(&self) -> &mir::NodeTree {
        &self.module.tree
    }

    /// Return the compiled layout for one MIR type.
    #[inline]
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.module.layout(ty).ok_or_else(|| Error::TypeMismatch {
            expected: "compiled layout".to_string(),
            actual: format!("{ty:?}"),
        })
    }

    /// Return the storage byte width for one MIR type.
    #[inline]
    pub(crate) fn storage_byte_len(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<usize, Error> {
        Ok(self.layout(ty)?.byte_len)
    }

    /// Return the MIR type stored in one SSA value slot.
    #[inline]
    pub(crate) fn value_type(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        let frame_layout = unsafe { (*self.frame).frame_layout };
        let frame_layout = self
            .module
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?;
        let slot_index = frame_layout
            .value_slots
            .start
            .checked_add(value.0)
            .ok_or(Error::InvalidInstruction)?;
        let slot = frame_layout
            .slot(slot_index)
            .ok_or(Error::InvalidInstruction)?;

        Ok(slot.ty)
    }

    /// Borrow the isolate options.
    #[inline]
    pub(crate) fn options(&self) -> &IsolateOptions {
        self.options
    }

    /// Create an error with current call stack.
    #[cold]
    pub(crate) fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(
            self.engine
                .stack
                .iter()
                .map(|frame| {
                    let func = self.module.tree.get(frame.function);
                    let name = self.module.strings.get(func.name).to_string();
                    FrameInfo {
                        function: frame.function,
                        block: frame.current_block,
                        function_name: Some(name),
                    }
                })
                .collect(),
        )
    }

    /// Refresh cached pointers for the current frame and function.
    pub(crate) fn refresh_for_function(&mut self, function: &Function) {
        // load frame bounds
        let (values_ptr, value_count, local_count) = {
            let frame = self.current_frame_mut();
            (frame.slots_mut_ptr(), frame.value_count, frame.local_count)
        };

        // cache stack pointers
        self.values = values_ptr;
        self.locals = unsafe { values_ptr.add(value_count) };
        self.value_count = value_count;
        self.local_count = local_count;

        // refresh argument and switch pools
        self.argument_pool = function.argument_pool.as_ptr();
        self.argument_pool_len = function.argument_pool.len();
        self.switch_case_pool = function.switch_case_pool.as_ptr();
        self.switch_case_pool_len = function.switch_case_pool.len();
    }

    /// Borrow the heap for the current block.
    #[inline]
    pub(crate) fn heap_mut(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Borrow the heap immutably for the current block.
    #[inline]
    pub(crate) fn heap(&self) -> &Heap {
        unsafe { &*self.heap }
    }

    /// Borrow the shared heap immutably for the current block.
    #[inline]
    pub(crate) fn shared(&self) -> &SharedHeap {
        unsafe { &*self.shared }
    }

    /// Borrow the shared heap immutably for the current block.
    #[inline]
    pub(crate) fn shared_ref(&self) -> &SharedHeap {
        self.shared()
    }

    /// Read one exact local heap byte range into owned storage.
    pub(crate) fn read_heap_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let available_len = self.heap().heap_byte_len(reference)?;
        let end = start
            .checked_add(byte_len)
            .ok_or(HeapError::InvariantOverflow {
                context: "vm heap read bytes",
            })?;
        if end > available_len {
            return Err(HeapError::InvalidHeapReference { reference });
        }

        let mut bytes = vec![0u8; byte_len];
        self.heap()
            .read_heap_bytes_into(reference, start, &mut bytes)?;

        Ok(bytes)
    }

    /// Read one exact shared heap byte range into owned storage.
    pub(crate) fn read_shared_heap_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let available_len = self.shared().heap_byte_len(reference)?;
        let end = start
            .checked_add(byte_len)
            .ok_or(HeapError::InvariantOverflow {
                context: "vm shared heap read bytes",
            })?;
        if end > available_len {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        }

        let mut bytes = vec![0u8; byte_len];
        self.shared()
            .read_heap_bytes_into(reference, start, &mut bytes)?;

        Ok(bytes)
    }

    /// Write one managed byte range through the interpreter mutator path.
    pub(crate) fn write_heap_bytes(
        &mut self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        self.heap_mut().write_heap_bytes(reference, start, bytes)?;
        self.heap_mut().write_barrier(reference, start, bytes.len())
    }

    /// Execute one intrinsic against the current interpreter and heap state.
    pub(crate) fn execute_intrinsic(
        &mut self,
        destination: mir::Value,
        intrinsic: mir::Intrinsic,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_intrinsic_resolved(destination, intrinsic, args)
    }

    /// Move the state to a new frame and function.
    pub(crate) fn enter_frame(&mut self, frame_index: usize, function: &Function) {
        // validate frame index in debug builds
        debug_assert!(
            frame_index < self.engine.stack.len(),
            "frame index out of bounds"
        );

        // update cached frame pointer
        let frame = unsafe { self.engine.stack.get_unchecked_mut(frame_index) };
        self.frame_index = frame_index;
        self.frame = frame as *mut Frame;

        // refresh cached pointers
        self.refresh_for_function(function);
    }

    /// Record a profile sample for the current instruction when enabled.
    #[inline(always)]
    pub(crate) fn maybe_profile_instruction(&mut self, instruction: &Instruction) {
        #[cfg(feature = "stats")]
        if let Some(profile) = self.engine.instruction_profile.as_mut() {
            profile.maybe_sample(instruction.opcode.name());
        }
        #[cfg(not(feature = "stats"))]
        {
            let _ = instruction;
        }
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub(crate) fn current_frame_mut(&mut self) -> &mut Frame {
        unsafe { &mut *self.frame }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub(crate) fn frame_by_index(&self, frame_index: usize) -> Result<&Frame, Error> {
        self.engine
            .stack
            .get(frame_index)
            .ok_or(Error::InvalidHeapReference)
    }

    /// Get a frame by index mutably.
    #[inline(always)]
    pub(crate) fn frame_by_index_mut(&mut self, frame_index: usize) -> Result<&mut Frame, Error> {
        self.engine
            .stack
            .get_mut(frame_index)
            .ok_or(Error::InvalidHeapReference)
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub(crate) fn get(&self, v: mir::Value) -> Value {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe { *self.values.add(index) }
    }

    /// Set value by SSA id.
    #[inline(always)]
    pub(crate) fn set(&mut self, v: mir::Value, val: Value) {
        let index = v.0 as usize;
        debug_assert!(index < self.value_count, "ssa value out of bounds: {v:?}");
        unsafe {
            *self.values.add(index) = val;
        }
    }

    /// Get local variable by local index.
    #[inline(always)]
    pub(crate) fn get_local_by_index(&self, local_index: u32) -> Value {
        let index = local_index as usize;
        debug_assert!(
            index < self.local_count,
            "local out of bounds: {local_index}"
        );
        unsafe { *self.locals.add(index) }
    }

    /// Set local variable by local index.
    #[inline(always)]
    pub(crate) fn set_local_by_index(&mut self, local_index: u32, val: Value) {
        let index = local_index as usize;
        debug_assert!(
            index < self.local_count,
            "local out of bounds: {local_index}"
        );
        unsafe {
            *self.locals.add(index) = val;
        }
    }

    /// Get the argument slice for the given range.
    #[inline(always)]
    pub(crate) fn argument_slice(&self, range: ArgumentRange) -> &[mir::Value] {
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
    pub(crate) fn switch_cases(&self, range: SwitchRange) -> &[SwitchCase] {
        let start = range.start as usize;
        let len = range.len as usize;
        debug_assert!(
            start + len <= self.switch_case_pool_len,
            "switch case pool out of bounds for range"
        );
        unsafe { std::slice::from_raw_parts(self.switch_case_pool.add(start), len) }
    }
}
