use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::{HeapCell, HeapHandle, Value};

/// Inline capacity for SSA values.
const VALUES_INLINE_CAP: usize = 8;

/// Inline capacity for local variables.
const LOCALS_INLINE_CAP: usize = 4;

/// Call frame in the interpreter.
#[derive(Debug)]
pub struct Frame {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The entry block of the function.
    pub entry_block: mir::LocalNodeId<mir::Block>,
    /// The current block being executed.
    pub current_block: mir::LocalNodeId<mir::Block>,
    /// Current instruction index within the block (for resuming after calls).
    pub instruction_idx: usize,
    /// SSA values in this frame, indexed by value id.
    pub values: SmallVec<[Option<Value>; VALUES_INLINE_CAP]>,
    /// Local variables (stack slots).
    pub locals: SmallVec<[(mir::LocalNodeId<mir::Local>, Value); LOCALS_INLINE_CAP]>,
    /// Stack-allocated cells (freed when frame pops).
    pub stack_cells: Vec<HeapCell>,
    /// Where to store the return value when callee returns (set by caller before pushing a new frame).
    pub return_destination: Option<mir::Value>,
}

impl Frame {
    /// Create a new frame for a function.
    pub fn new(
        function: mir::LocalNodeId<mir::Function>,
        entry_block: mir::LocalNodeId<mir::Block>,
    ) -> Self {
        Self {
            function,
            entry_block,
            current_block: entry_block,
            instruction_idx: 0,
            values: SmallVec::new(),
            locals: SmallVec::new(),
            stack_cells: Vec::new(),
            return_destination: None,
        }
    }

    /// Get a value from this frame.
    #[inline]
    pub fn get_value(&self, value: mir::Value) -> RuntimeResult<Value> {
        let index = value.0 as usize;
        self.values
            .get(index)
            .and_then(|opt| opt.clone())
            .ok_or_else(|| RuntimeError::new(Error::UndefinedValue { value }))
    }

    /// Set a value in this frame.
    #[inline]
    pub fn set_value(&mut self, value: mir::Value, val: Value) {
        let index = value.0 as usize;
        // grow vec if needed
        if index >= self.values.len() {
            self.values.resize(index + 1, None);
        }
        self.values[index] = Some(val);
    }

    /// Get a local variable.
    pub fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> RuntimeResult<Value> {
        self.locals
            .iter()
            .find(|(id, _)| *id == local)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| RuntimeError::new(Error::UndefinedLocal { local }))
    }

    /// Set a local variable.
    pub fn set_local(&mut self, local: mir::LocalNodeId<mir::Local>, value: Value) {
        // check if local already exists
        if let Some((_, existing)) = self.locals.iter_mut().find(|(id, _)| *id == local) {
            *existing = value;
        } else {
            self.locals.push((local, value));
        }
    }

    /// Check if a value is defined in this frame.
    #[inline]
    pub fn has_value(&self, value: mir::Value) -> bool {
        let index = value.0 as usize;
        self.values.get(index).is_some_and(|opt| opt.is_some())
    }

    /// Clear all values (but keep locals).
    pub fn clear_values(&mut self) {
        self.values.clear();
    }

    /// Allocate a new stack cell, returning its slot index.
    pub fn allocate_stack_cell(&mut self) -> usize {
        let slot = self.stack_cells.len();
        self.stack_cells.push(HeapCell::new());
        slot
    }

    /// Get a stack cell by slot index.
    #[inline]
    pub fn get_stack_cell(&self, slot: usize) -> Option<&HeapCell> {
        self.stack_cells.get(slot)
    }

    /// Get a mutable reference to a stack cell by slot index.
    #[inline]
    pub fn get_stack_cell_mut(&mut self, slot: usize) -> Option<&mut HeapCell> {
        self.stack_cells.get_mut(slot)
    }

    /// Collect all heap handles from this frame for GC roots.
    pub fn collect_roots(&self, roots: &mut Vec<HeapHandle>) {
        for value in self.values.iter().flatten() {
            Self::collect_handles_from_value(value, roots);
        }
        for (_, value) in &self.locals {
            Self::collect_handles_from_value(value, roots);
        }
        for cell in &self.stack_cells {
            for value in &cell.slots {
                Self::collect_handles_from_value(value, roots);
            }
        }
    }

    /// Recursively collect heap handles from a value.
    fn collect_handles_from_value(value: &Value, roots: &mut Vec<HeapHandle>) {
        match value {
            Value::ManagedReference(handle) => {
                roots.push(*handle);
            }
            Value::Aggregate(fields) => {
                for field in fields {
                    Self::collect_handles_from_value(field, roots);
                }
            }
            _ => {}
        }
    }
}
