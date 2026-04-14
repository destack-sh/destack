use std::ptr::NonNull;

use {destack_engine as engine, destack_mir as mir};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::executable::{Block, Executable, Function, FunctionTable};
use crate::snapshot::InterpreterFrameImage;
use destack_heap::{ManagedReference, Value};

use super::StackAllocation;

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The logical frame layout for this activation.
    pub(crate) frame_layout: engine::FrameLayoutId,
    /// The function being executed.
    pub(crate) function: mir::LocalNodeId<mir::Function>,
    /// Pointer to the lowered function for fast dispatch.
    pub(crate) function_ptr: NonNull<Function>,
    /// Pointer to the current block.
    pub(crate) block_ptr: NonNull<Block>,
    /// The entry block of the function.
    pub(crate) entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub(crate) current_block: mir::LocalNodeId<mir::Block>,
    /// Current block index.
    pub(crate) block_index: usize,
    /// Program counter within the current block.
    pub(crate) resume_pc: usize,
    /// The pending transfer owned by this frame while one callee runs.
    pub(crate) transfer: Option<engine::FrameTransfer>,
    /// Base offset into the interpreter value stack.
    pub(crate) value_base: usize,
    /// Count of SSA values in this frame.
    pub(crate) value_count: usize,
    /// Base offset into the interpreter local stack.
    pub(crate) local_base: usize,
    /// Count of local variables in this frame.
    pub(crate) local_count: usize,
    /// Stack-allocated byte buffers, freed when the frame pops.
    pub(crate) stack_allocations: Vec<Option<StackAllocation>>,
    /// Closure environment pointer for this frame.
    pub(crate) environment: Value,
}

// the raw function and block pointers always point into immutable executable storage
// that stays alive for the duration of the owning isolate
unsafe impl Send for Frame {}

impl Frame {
    /// Create a new frame for a function.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        frame_layout: engine::FrameLayoutId,
        function: mir::LocalNodeId<mir::Function>,
        function_ptr: NonNull<Function>,
        block_ptr: NonNull<Block>,
        entry_block: mir::LocalNodeId<mir::Block>,
        block_index: usize,
        value_base: usize,
        value_count: usize,
        local_base: usize,
        local_count: usize,
        environment: Value,
    ) -> Self {
        // assemble frame state
        Self {
            frame_layout,
            function,
            function_ptr,
            block_ptr,
            entry_block,
            current_block: entry_block,
            block_index,
            resume_pc: 0,
            transfer: None,
            value_base,
            value_count,
            local_base,
            local_count,
            stack_allocations: Vec::new(),
            environment,
        }
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

    /// Allocate a new stack allocation, returning its slot index.
    pub(crate) fn allocate_stack_allocation(&mut self, allocation: StackAllocation) -> usize {
        let slot = self.stack_allocations.len();
        self.stack_allocations.push(Some(allocation));
        slot
    }

    /// Retire one stack allocation by slot index.
    pub(crate) fn retire_stack_allocation(&mut self, slot: usize) -> bool {
        let Some(buffer) = self.stack_allocations.get_mut(slot) else {
            return false;
        };

        buffer.take().is_some()
    }

    /// Return whether this frame still owns any live stack allocation.
    pub fn has_live_stack_allocations(&self) -> bool {
        self.stack_allocations.iter().any(Option::is_some)
    }

    /// Return one logical frame slot value.
    pub fn slot_value(&self, values: &[Value], locals: &[Value], slot: u32) -> Option<Value> {
        if (slot as usize) < values.len() {
            return values.get(slot as usize).copied();
        }

        let local_start = self.value_count as u32;
        if let Some(index) = slot.checked_sub(local_start)
            && (index as usize) < locals.len()
        {
            return locals.get(index as usize).copied();
        }

        let closure_slot = local_start + self.local_count as u32;
        if slot == closure_slot {
            return Some(self.environment);
        }

        None
    }

    /// Get a stack allocation by slot index.
    #[inline]
    pub(crate) fn stack_allocation(&self, slot: usize) -> Option<&StackAllocation> {
        self.stack_allocations.get(slot).and_then(Option::as_ref)
    }

    /// Get a mutable reference to a stack allocation by slot index.
    #[inline]
    pub(crate) fn stack_allocation_mut(&mut self, slot: usize) -> Option<&mut StackAllocation> {
        self.stack_allocations
            .get_mut(slot)
            .and_then(Option::as_mut)
    }

    /// Collect all managed references from this frame for GC roots.
    pub fn collect_roots(
        &self,
        executable: &Executable,
        values: &[Value],
        locals: &[Value],
        roots: &mut Vec<ManagedReference>,
    ) -> Result<(), Error> {
        let layout =
            executable
                .frame_layout(self.function)
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!("missing frame layout for frame: {:?}", self.function),
                })?;

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

        // value slots
        for (index, value) in value_slice.iter().enumerate() {
            let slot = layout.value_slots.start + index as u32;
            if layout.contains_managed_references(slot) {
                Self::collect_pointers_from_value(value, roots);
            }
        }

        // local slots
        for (index, value) in local_slice.iter().enumerate() {
            let slot = layout.local_slots.start + index as u32;
            if layout.contains_managed_references(slot) {
                Self::collect_pointers_from_value(value, roots);
            }
        }

        // function environment
        if let Some(slot) = layout.environment_slot
            && layout.contains_managed_references(slot)
        {
            Self::collect_pointers_from_value(&self.environment, roots);
        }

        // dynamic stack allocations
        for allocation in self.stack_allocations.iter().flatten() {
            let layout = executable
                .layout(allocation.storage_type())
                .ok_or_else(|| Error::InvariantViolation {
                    context: format!(
                        "missing layout for stack allocation: frame={:?}, storage_type={:?}",
                        self.function,
                        allocation.storage_type(),
                    ),
                })?;

            let trace_result = layout.reference_map.trace_references(
                allocation.bytes(),
                executable.tree.metadata.layout.storage.native_pointer_bytes,
                |reference| roots.push(reference),
            );

            if let Err(error) = trace_result {
                panic!("stack allocation root scan failed: {error}");
            }
        }

        Ok(())
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
        let stack_allocations = self.stack_allocations.to_vec();

        // assemble cloned frame
        Self {
            frame_layout: self.frame_layout,
            function: self.function,
            function_ptr: self.function_ptr,
            block_ptr: self.block_ptr,
            entry_block: self.entry_block,
            current_block: self.current_block,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            value_base: self.value_base,
            value_count: self.value_count,
            local_base: self.local_base,
            local_count: self.local_count,
            stack_allocations,
            environment: self.environment,
        }
    }

    /// Capture one immutable frame image.
    pub(crate) fn image(&self) -> InterpreterFrameImage {
        InterpreterFrameImage {
            frame_layout: self.frame_layout,
            function: self.function,
            entry_block: self.entry_block,
            current_block: self.current_block,
            block_index: self.block_index,
            resume_pc: self.resume_pc,
            transfer: self.transfer.clone(),
            value_base: self.value_base,
            value_count: self.value_count,
            local_base: self.local_base,
            local_count: self.local_count,
            stack_allocations: self.stack_allocations.clone(),
            environment: self.environment,
        }
    }

    /// Create one frame from an immutable image.
    pub(crate) fn from_image(
        image: &InterpreterFrameImage,
        functions: &FunctionTable,
    ) -> RuntimeResult<Self> {
        // resolve the lowered function for this frame
        let function_index = functions.index_for(image.function).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        let function_ptr = functions.get_ptr_by_index(function_index).ok_or_else(|| {
            RuntimeError::new(Error::UndefinedFunction {
                function: image.function,
            })
        })?;

        // resolve the current block pointer from the lowered function
        let function_ref = unsafe { function_ptr.as_ref() };
        let block = function_ref.blocks.get(image.block_index).ok_or_else(|| {
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
            frame_layout: image.frame_layout,
            function: image.function,
            function_ptr,
            block_ptr: NonNull::from(block),
            entry_block: image.entry_block,
            current_block: image.current_block,
            block_index: image.block_index,
            resume_pc: image.resume_pc,
            transfer: image.transfer.clone(),
            value_base: image.value_base,
            value_count: image.value_count,
            local_base: image.local_base,
            local_count: image.local_count,
            stack_allocations: image.stack_allocations.clone(),
            environment: image.environment,
        })
    }
}
