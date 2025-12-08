//! Core interpreter structure and management.

use std::collections::HashMap;

use destack_mir as mir;
use destack_source::ImmutableStringPool;

use crate::diagnostic::{DiagnosticAnchor, Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::memory::{Heap, Value};

use super::{Frame, MachineOptions, Statistics};

/// External function type.
pub type ExternalFn = Box<dyn Fn(&[Value]) -> Result<Value, Error> + Send + Sync>;

/// Output from executing MIR code.
#[derive(Debug, Clone)]
pub struct ExecutionOutput {
    /// The return value of the executed function.
    pub value: Value,
    /// Statistics from this execution.
    pub statistics: Statistics,
    /// Number of heap cells at end of execution.
    pub heap_cells: usize,
}

/// MIR interpreter.
///
/// Executes MIR functions by walking the instruction tree.
/// Uses an explicit call stack instead of Rust recursion.
pub struct Interpreter {
    /// The MIR tree being executed.
    pub(super) tree: mir::NodeTree,
    /// String pool for names.
    pub(super) strings: ImmutableStringPool,
    /// The managed heap.
    pub(super) heap: Heap,
    /// External function handlers.
    pub(super) externals: HashMap<String, ExternalFn>,
    /// Configuration options.
    pub(super) options: MachineOptions,
    /// Explicit call stack.
    pub(super) call_stack: Vec<Frame>,
    /// Execution statistics.
    pub(super) statistics: Statistics,
}

impl std::fmt::Debug for Interpreter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interpreter")
            .field("heap", &self.heap)
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .field("call_stack_depth", &self.call_stack.len())
            .field("statistics", &self.statistics)
            .finish_non_exhaustive()
    }
}

impl Interpreter {
    /// Create a new interpreter with default options.
    pub fn new(tree: mir::NodeTree, strings: ImmutableStringPool) -> Self {
        Self::with_options(tree, strings, MachineOptions::default())
    }

    /// Create a new interpreter with custom options.
    pub fn with_options(
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: MachineOptions,
    ) -> Self {
        Self {
            tree,
            strings,
            heap: Heap::new(),
            externals: HashMap::new(),
            options,
            call_stack: Vec::new(),
            statistics: Statistics::new(),
        }
    }

    /// Register an external function handler.
    pub fn register_external<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value, Error> + Send + Sync + 'static,
    {
        self.externals.insert(name.to_string(), Box::new(handler));
    }

    /// Get reference to current frame.
    pub(super) fn current_frame(&self) -> RuntimeResult<&Frame> {
        self.call_stack
            .last()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))
    }

    /// Get mutable reference to current frame.
    pub(super) fn current_frame_mut(&mut self) -> RuntimeResult<&mut Frame> {
        self.call_stack
            .last_mut()
            .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))
    }

    /// Create an error with current call stack.
    pub(super) fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    /// Create an error with instruction anchor.
    pub(super) fn make_error_at(
        &self,
        error: Error,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> RuntimeError {
        let frame = self.call_stack.last();
        let anchor = if let Some(f) = frame {
            DiagnosticAnchor::Instruction {
                function: f.function,
                block: f.current_block,
                instruction: instruction_id,
            }
        } else {
            DiagnosticAnchor::None
        };

        RuntimeError::new(error)
            .with_call_stack(self.get_call_stack_info())
            .with_anchor(anchor)
    }

    /// Get call stack info for error reporting.
    fn get_call_stack_info(&self) -> Vec<FrameInfo> {
        self.call_stack
            .iter()
            .map(|f| {
                let func = self.tree.get(f.function);
                let name = self.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Get a reference to the heap.
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    /// Get a mutable reference to the heap.
    pub fn heap_mut(&mut self) -> &mut Heap {
        &mut self.heap
    }

    /// Run garbage collection.
    pub fn collect_garbage(&mut self) {
        // collect roots from all frames
        let mut roots = Vec::new();
        for frame in &self.call_stack {
            frame.collect_roots(&mut roots);
        }

        self.heap.collect(&roots);
    }
}
