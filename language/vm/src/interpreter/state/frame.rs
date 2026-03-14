use std::ptr::NonNull;

use destack_mir as mir;

use super::super::decode::{INVALID_VALUE_ID, ThreadedBlock, ThreadedFunction};
use super::interpreter::ThreadedFunctionTable;
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::snapshot::FrameImage;
use destack_heap::{ManagedReference, Value, ValueBuffer};

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// Pointer to the threaded function for fast dispatch.
    pub threaded: NonNull<ThreadedFunction>,
    /// Pointer to the current threaded block.
    pub block_ptr: NonNull<ThreadedBlock>,
    /// The entry block of the function.
    pub entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// Current threaded block index.
    pub block_index: usize,
    /// Program counter within the current threaded block.
    pub resume_pc: usize,
    /// Base offset into the interpreter value stack.
    pub value_base: usize,
    /// Count of SSA values in this frame.
    pub value_count: usize,
    /// Base offset into the interpreter local stack.
    pub local_base: usize,
    /// Count of local variables in this frame.
    pub local_count: usize,
    /// Stack-allocated value buffers, freed when the frame pops.
    pub stack_values: Vec<ValueBuffer>,
    /// Closure environment pointer for this frame.
    pub closure_env: Value,
    /// Return destination for the caller or INVALID_VALUE_ID for none.
    pub return_destination: mir::Value,
}

impl Frame {
    /// Create a new frame for a function.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        function: mir::LocalNodeId<mir::Function>,
        threaded: NonNull<ThreadedFunction>,
        block_ptr: NonNull<ThreadedBlock>,
        entry_block: mir::LocalNodeId<mir::Block>,
        block_index: usize,
        value_base: usize,
        value_count: usize,
        local_base: usize,
        local_count: usize,
        closure_env: Value,
    ) -> Self {
        // assemble frame state
        Self {
            function,
            threaded,
            block_ptr,
            entry_block,
            current_block: entry_block,
            block_index,
            resume_pc: 0,
            value_base,
            value_count,
            local_base,
            local_count,
            stack_values: Vec::new(),
            closure_env,
            return_destination: mir::Value(INVALID_VALUE_ID),
        }
    }

    /// Get the threaded function for this frame.
    #[inline]
    pub fn threaded(&self) -> &ThreadedFunction {
        // return threaded function
        // #Safety: threaded pointer is valid for interpreter lifetime
        unsafe { self.threaded.as_ref() }
    }

    /// Get a value from this frame.
    #[inline]
    pub fn get_value(&self, values: &[Value], value: mir::Value) -> RuntimeResult<Value> {
        // forward to value lookup
        self.get_value_or_error(values, value)
            .map_err(RuntimeError::new)
    }

    /// Get a value from this frame without call stack context.
    #[inline]
    pub fn get_value_or_error(&self, values: &[Value], value: mir::Value) -> Result<Value, Error> {
        // compute value index
        let index = value.0 as usize;

        // reject out of bounds values
        if index >= self.value_count {
            return Err(Error::UndefinedValue { value });
        }

        // read value slot
        let slot = self.value_base + index;
        values
            .get(slot)
            .copied()
            .ok_or(Error::UndefinedValue { value })
    }

    /// Set a value in this frame.
    #[inline]
    pub fn set_value(&self, values: &mut [Value], value: mir::Value, val: Value) {
        // compute value index
        let index = value.0 as usize;

        // validate bounds in debug builds
        debug_assert!(
            index < self.value_count,
            "ssa value out of bounds: {value:?}"
        );

        // write value slot
        let slot = self.value_base + index;
        values[slot] = val;
    }

    /// Get a local variable.
    pub fn get_local(
        &self,
        locals: &[Value],
        local: mir::LocalNodeId<mir::Local>,
    ) -> RuntimeResult<Value> {
        // forward to local lookup
        self.get_local_or_error(locals, local)
            .map_err(RuntimeError::new)
    }

    /// Get a local variable without call stack context.
    #[inline]
    pub fn get_local_or_error(
        &self,
        locals: &[Value],
        local: mir::LocalNodeId<mir::Local>,
    ) -> Result<Value, Error> {
        // compute local index
        let index = local.id as usize;

        // reject out of bounds locals
        if index >= self.local_count {
            return Err(Error::UndefinedLocal { local });
        }

        // read local slot
        let slot = self.local_base + index;
        locals
            .get(slot)
            .copied()
            .ok_or(Error::UndefinedLocal { local })
    }

    /// Set a local variable.
    pub fn set_local(
        &self,
        locals: &mut [Value],
        local: mir::LocalNodeId<mir::Local>,
        value: Value,
    ) {
        // compute local index
        let index = local.id as usize;

        // validate bounds in debug builds
        debug_assert!(index < self.local_count, "local out of bounds: {local:?}");

        // write local slot
        let slot = self.local_base + index;
        locals[slot] = value;
    }

    /// Check if a value is defined in this frame.
    #[inline]
    pub fn has_value(&self, value: mir::Value) -> bool {
        // check value bounds
        let index = value.0 as usize;
        index < self.value_count
    }

    /// Clear all values (but keep locals).
    pub fn clear_values(&self, values: &mut [Value]) {
        // compute value range
        let start = self.value_base;
        let end = self.value_base + self.value_count;

        // clear value slots
        values[start..end].fill(Value::VOID);
    }

    /// Allocate a new stack buffer, returning its slot index.
    pub fn allocate_stack_buffer(&mut self) -> usize {
        let slot = self.stack_values.len();
        self.stack_values.push(ValueBuffer::new());
        slot
    }

    /// Allocate a new stack buffer with the given value count.
    pub fn allocate_stack_buffer_with_values(&mut self, slot_count: usize) -> usize {
        // allocate stack buffer
        let slot = self.stack_values.len();
        self.stack_values
            .push(ValueBuffer::with_values_len(slot_count));
        slot
    }

    /// Get a stack buffer by slot index.
    #[inline]
    pub fn stack_buffer(&self, slot: usize) -> Option<&ValueBuffer> {
        self.stack_values.get(slot)
    }

    /// Get a mutable reference to a stack buffer by slot index.
    #[inline]
    pub fn stack_buffer_mut(&mut self, slot: usize) -> Option<&mut ValueBuffer> {
        self.stack_values.get_mut(slot)
    }

    /// Collect all managed references from this frame for GC roots.
    pub fn collect_roots(
        &self,
        values: &[Value],
        locals: &[Value],
        roots: &mut Vec<ManagedReference>,
    ) {
        // validate stack bounds in debug builds
        debug_assert!(
            self.value_base + self.value_count <= values.len(),
            "value stack out of bounds for frame"
        );
        debug_assert!(
            self.local_base + self.local_count <= locals.len(),
            "local stack out of bounds for frame"
        );

        // slice value and local ranges
        let value_slice = &values[self.value_base..self.value_base + self.value_count];
        let local_slice = &locals[self.local_base..self.local_base + self.local_count];

        // collect pointers from values
        for value in value_slice {
            Self::collect_pointers_from_value(value, roots);
        }

        // collect pointers from locals
        for value in local_slice {
            Self::collect_pointers_from_value(value, roots);
        }

        // collect pointers from stack value buffers
        for values in &self.stack_values {
            for value in values {
                Self::collect_pointers_from_value(value, roots);
            }
        }
    }

    /// Collect one managed reference from a value if applicable.
    fn collect_pointers_from_value(value: &Value, roots: &mut Vec<ManagedReference>) {
        if let Some(pointer) = value.as_managed_reference() {
            roots.push(pointer);
        }
    }

    /// Clone this frame for a forked continuation.
    pub(crate) fn clone_for_fork(&self) -> Self {
        // clone stack value buffers for the forked frame
        let stack_values = self
            .stack_values
            .iter()
            .map(ValueBuffer::clone_for_fork)
            .collect();

        // assemble cloned frame
        Self {
            function: self.function,
            threaded: self.threaded,
            block_ptr: self.block_ptr,
            entry_block: self.entry_block,
            current_block: self.current_block,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            value_base: self.value_base,
            value_count: self.value_count,
            local_base: self.local_base,
            local_count: self.local_count,
            stack_values,
            closure_env: self.closure_env,
            return_destination: self.return_destination,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self) -> FrameImage {
        FrameImage {
            function: self.function,
            entry_block: self.entry_block,
            current_block: self.current_block,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            value_base: self.value_base,
            value_count: self.value_count,
            local_base: self.local_base,
            local_count: self.local_count,
            stack_values: self.stack_values.clone(),
            closure_env: self.closure_env,
            return_destination: self.return_destination,
        }
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(
        image: &FrameImage,
        threaded_functions: &ThreadedFunctionTable,
    ) -> RuntimeResult<Self> {
        // resolve the threaded function for this frame
        let threaded_index = threaded_functions
            .index_for(image.function)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: image.function,
                })
            })?;

        let threaded = threaded_functions
            .get_ptr_by_index(threaded_index)
            .ok_or_else(|| {
                RuntimeError::new(Error::UndefinedFunction {
                    function: image.function,
                })
            })?;

        // resolve the current block pointer from the threaded function
        let threaded_ref = unsafe { threaded.as_ref() };
        let block = threaded_ref.blocks.get(image.block_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedBlock {
                block: image.current_block,
            })
        })?;

        // validate the restored block identity
        if block.mir_block != image.current_block {
            return Err(RuntimeError::new(Error::UndefinedBlock {
                block: image.current_block,
            }));
        }

        Ok(Self {
            function: image.function,
            threaded,
            block_ptr: NonNull::from(block),
            entry_block: image.entry_block,
            current_block: image.current_block,
            block_index: image.block_index,
            resume_pc: image.resume_pc,
            value_base: image.value_base,
            value_count: image.value_count,
            local_base: image.local_base,
            local_count: image.local_count,
            stack_values: image.stack_values.clone(),
            closure_env: image.closure_env,
            return_destination: image.return_destination,
        })
    }
}
