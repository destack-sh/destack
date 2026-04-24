use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use destack_core::{Capture, CaptureMode, ImmutableStringPool, SnapshotCodec};
use {destack_engine as engine, destack_mir as mir};

use super::{
    ExternalCallContext, ExternalFn, ExternalHandler, GlobalStorage, RootSet, RootVisitor,
};
use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::interpreter::{
    Continuation, Interpreter, RunOutcome, RunOutput, visit_materialized_value_roots,
};
use crate::module::Module;
use crate::options::IsolateOptions;
use crate::snapshot::{ContinuationImage, IsolateImage};
use crate::{SharedHeap, Value};
use destack_heap::{Heap, HeapReference, SharedRawLimits};

/// VM isolate with globals and execution state.
pub struct Isolate {
    /// Unique id used to validate continuation ownership.
    isolate_id: engine::IsolateId,
    /// Immutable module shared by this isolate.
    module: Arc<Module>,
    /// Configuration options for this isolate.
    options: IsolateOptions,
    /// Global variable storage.
    globals: GlobalStorage,
    /// External function handlers.
    externals: HashMap<String, ExternalFn>,
    /// Interpreter engine backing this isolate.
    interpreter: Interpreter,
}

impl fmt::Debug for Isolate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Isolate")
            .field("module", &self.module)
            .field("globals", &format!("<{} globals>", self.globals.len()))
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[allow(clippy::arc_with_non_send_sync)]
impl Isolate {
    /// Create a new isolate from one shared immutable image.
    pub fn new(image: Arc<IsolateImage>) -> RuntimeResult<Self> {
        let module = Arc::new(Module::new(image.tree.clone(), image.strings.clone())?);
        let mut isolate = Self {
            isolate_id: image.isolate_id,
            module,
            options: image.options.clone(),
            globals: image.globals.clone(),
            externals: HashMap::new(),
            interpreter: Interpreter::new(),
        };
        isolate.interpreter =
            Interpreter::from_image(&isolate.module.functions, &image.interpreter)?;

        Ok(isolate)
    }

    /// Build a new isolate with default options.
    pub fn build(
        isolate_id: engine::IsolateId,
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
    ) -> RuntimeResult<Self> {
        Self::build_with_options(isolate_id, tree, strings, IsolateOptions::default())
    }

    /// Build a new isolate with custom options.
    pub fn build_with_options(
        isolate_id: engine::IsolateId,
        tree: mir::NodeTree,
        strings: ImmutableStringPool,
        options: IsolateOptions,
    ) -> RuntimeResult<Self> {
        let module = Arc::new(Module::new(tree, strings)?);
        let native_pointer_bytes = module.tree.metadata.layout.storage.native_pointer_bytes;
        let host_pointer_bytes = HeapReference::BYTE_LEN as u8;

        // host execution only supports native-width pointers
        if native_pointer_bytes != host_pointer_bytes {
            return Err(RuntimeError::new(Error::IncompatiblePointerWidth {
                bytes: native_pointer_bytes,
                host_bytes: host_pointer_bytes,
            }));
        }

        Ok(Self {
            isolate_id,
            module,
            options,
            globals: GlobalStorage::new(),
            externals: HashMap::new(),
            interpreter: Interpreter::new(),
        })
    }

    /// Initialize isolate globals against one explicit heap.
    pub fn initialize(&mut self, heap: &mut Heap, shared: &SharedHeap) -> RuntimeResult<()> {
        // initialize globals
        self.interpreter.initialize_globals(
            self.module.as_ref(),
            &mut self.globals,
            heap,
            shared,
        )?;

        self.stabilize_globals(heap)
    }

    /// Get the isolate options.
    pub fn options(&self) -> &IsolateOptions {
        &self.options
    }

    /// Get mutable isolate options.
    pub fn options_mut(&mut self) -> &mut IsolateOptions {
        &mut self.options
    }

    /// Set whether to collect execution statistics.
    pub fn set_collect_stats(&mut self, collect: bool) {
        self.options.telemetry.collect_stats = collect;
    }

    /// Enable instruction profiling with the given sampling interval.
    #[cfg(feature = "stats")]
    pub fn enable_instruction_profile(&mut self, sample_interval: std::time::Duration) {
        self.interpreter.enable_instruction_profile(sample_interval);
    }

    /// Reset instruction profiling samples without disabling sampling.
    #[cfg(feature = "stats")]
    pub fn reset_instruction_profile(&mut self) {
        self.interpreter.reset_instruction_profile();
    }

    /// Clear instruction profiling data and disable sampling.
    #[cfg(feature = "stats")]
    pub fn clear_instruction_profile(&mut self) {
        self.interpreter.clear_instruction_profile();
    }

    /// Return a compact instruction profile report if available.
    #[cfg(feature = "stats")]
    pub fn instruction_profile_report(&mut self, target_percent: f64) -> Option<String> {
        self.interpreter.instruction_profile_report(target_percent)
    }

    /// Register a VM binding handler.
    pub fn register_vm_binding(&mut self, name: &str, handler: impl ExternalHandler + 'static) {
        self.externals.insert(name.to_string(), Arc::new(handler));
    }

    // FUGU #Architecture: remove once generated ABI stops registering runtime aggregate schemas
    /// Accept generated runtime aggregate registrations.
    pub fn register_named_aggregate_type(
        &mut self,
        _name: &str,
        _field_count: usize,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Run a callback with a runtime context for this isolate.
    pub fn with_runtime_context<F, R>(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        shared_raw_limits: SharedRawLimits,
        run: F,
    ) -> R
    where
        F: for<'ctx> FnOnce(&mut ExternalCallContext<'ctx>) -> R,
    {
        // borrow the isolate state needed by the external context
        let module = self.module.as_ref();
        let mut context = ExternalCallContext::new(module, heap, shared, shared_raw_limits);
        run(&mut context)
    }

    /// Resolve a function id by name.
    pub fn function_id_by_name(
        &self,
        name: &str,
    ) -> Result<mir::LocalNodeId<mir::Function>, RuntimeError> {
        let func_id = self
            .module
            .function_id_by_name
            .get(name)
            .copied()
            .ok_or_else(|| {
                self.make_error(Error::ExternalFunctionNotFound {
                    name: name.to_string(),
                })
            })?;

        Ok(func_id)
    }

    /// Run a function by name and return its output.
    pub fn run_function_by_name(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutput> {
        self.interpreter.run_function_by_name(
            self.isolate_id,
            self.module.as_ref(),
            &self.options,
            &mut self.globals,
            &self.externals,
            heap,
            shared,
            name,
            arguments,
        )
    }

    /// Run a function by name and allow yielding.
    pub fn run_function_by_name_yielding(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        name: &str,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutcome> {
        self.interpreter.run_function_by_name_yielding(
            self.isolate_id,
            self.module.as_ref(),
            &self.options,
            &mut self.globals,
            &self.externals,
            heap,
            shared,
            name,
            arguments,
        )
    }

    /// Run a function by id and return its output.
    pub fn run_function(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutput> {
        self.interpreter.run_function(
            self.isolate_id,
            self.module.as_ref(),
            &self.options,
            &mut self.globals,
            &self.externals,
            heap,
            shared,
            func_id,
            arguments,
        )
    }

    /// Run a function by id and allow yielding.
    pub fn run_function_yielding(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        func_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
    ) -> RuntimeResult<RunOutcome> {
        self.interpreter.run_function_yielding(
            self.isolate_id,
            self.module.as_ref(),
            &self.options,
            &mut self.globals,
            &self.externals,
            heap,
            shared,
            func_id,
            arguments,
        )
    }

    /// Resume a previously yielded coroutine.
    pub fn resume(
        &mut self,
        heap: &mut Heap,
        shared: &SharedHeap,
        continuation: Continuation,
        resume_value: engine::MaterializedValue,
    ) -> RuntimeResult<RunOutcome> {
        self.interpreter.resume(
            self.isolate_id,
            self.module.as_ref(),
            &self.options,
            &mut self.globals,
            &self.externals,
            heap,
            shared,
            continuation,
            resume_value,
        )
    }

    /// Visit roots retained by one materialized boundary value.
    pub fn visit_materialized_value_roots(
        &self,
        value: &engine::MaterializedValue,
        roots: &mut impl RootVisitor,
    ) -> RuntimeResult<()> {
        visit_materialized_value_roots(value, self.module.as_ref(), roots)
            .map_err(RuntimeError::new)
    }

    /// Capture one continuation as one immutable image.
    pub fn continuation_image(
        &self,
        continuation: &Continuation,
    ) -> RuntimeResult<ContinuationImage> {
        continuation.image(&self.module)
    }

    /// Restore one continuation from one immutable image.
    pub fn restore_continuation_image(
        &self,
        image: &ContinuationImage,
    ) -> RuntimeResult<Continuation> {
        let mut continuation =
            Continuation::from_image(image, &self.module, &self.module.functions)?;
        continuation.isolate_id = self.isolate_id;

        Ok(continuation)
    }

    /// Collect one complete root set from live state and optional continuations.
    pub fn root_set(&mut self, continuations: &[Continuation]) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();

        self.interpreter
            .visit_roots(&self.module, &self.globals, continuations, &mut roots)?;

        Ok(roots)
    }

    /// Visit one complete root set from live state and optional continuations.
    pub fn visit_state_roots(
        &mut self,
        continuations: &[Continuation],
        roots: &mut impl RootVisitor,
    ) -> RuntimeResult<()> {
        self.interpreter
            .visit_roots(&self.module, &self.globals, continuations, roots)
    }

    /// Collect roots from one live continuation.
    pub fn continuation_root_set(&mut self, continuation: &Continuation) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();
        self.visit_continuation_roots(continuation, &mut roots)?;

        Ok(roots)
    }

    /// Visit roots from one live continuation.
    pub fn visit_continuation_roots(
        &mut self,
        continuation: &Continuation,
        roots: &mut impl RootVisitor,
    ) -> RuntimeResult<()> {
        continuation
            .visit_roots(&self.module, roots)
            .map_err(|error| self.make_error(error))
    }

    /// Collect roots from one captured continuation image.
    pub fn continuation_image_root_set(
        &mut self,
        image: &ContinuationImage,
    ) -> RuntimeResult<RootSet> {
        let mut roots = RootSet::default();

        self.visit_image_roots(image, &mut roots)?;

        Ok(roots)
    }

    /// Visit roots from one captured continuation image.
    pub fn visit_image_roots(
        &mut self,
        image: &ContinuationImage,
        roots: &mut impl RootVisitor,
    ) -> RuntimeResult<()> {
        Continuation::visit_image_roots(image, &self.module, roots)
            .map_err(|error| self.make_error(error))
    }

    /// Capture one immutable VM image.
    pub fn image(&mut self) -> RuntimeResult<IsolateImage> {
        // capture the mutable isolate state
        let globals = self.globals.clone();
        let interpreter = self.interpreter.image();

        Ok(IsolateImage {
            tree: self.module.tree.clone(),
            strings: self.module.strings.clone(),
            options: self.options.clone(),
            isolate_id: self.isolate_id,
            globals,
            interpreter,
        })
    }

    /// Fork this isolate for one child branch.
    pub fn fork(&self) -> RuntimeResult<Self> {
        Ok(Self {
            isolate_id: self.isolate_id,
            module: self.module.clone(),
            options: self.options.clone(),
            globals: self.globals.clone(),
            externals: self.externals.clone(),
            interpreter: self.interpreter.fork(),
        })
    }

    /// Restore this isolate from one immutable VM image.
    pub fn restore_image(&mut self, heap: &mut Heap, image: &IsolateImage) -> RuntimeResult<()> {
        self.module = Arc::new(Module::new(image.tree.clone(), image.strings.clone())?);
        self.options = image.options.clone();

        // restore isolate-owned mutable state first
        self.isolate_id = image.isolate_id;
        self.globals = image.globals.clone();

        // rebuild interpreter state over the restored isolate
        let _ = heap;
        self.interpreter = Interpreter::from_image(&self.module.functions, &image.interpreter)?;

        Ok(())
    }

    /// Stabilize one live continuation before it escapes the running interpreter.
    pub fn stabilize_boundary_continuation(
        &mut self,
        heap: &mut Heap,
        continuation: &mut Continuation,
    ) -> RuntimeResult<()> {
        continuation.stabilize(&self.module, heap)
    }

    /// Stabilize one materialized boundary value before it escapes the running interpreter.
    pub fn stabilize_boundary_value(
        &mut self,
        heap: &mut Heap,
        value: &mut destack_engine::MaterializedValue,
    ) -> RuntimeResult<()> {
        crate::interpreter::stabilize_materialized_value(&self.module, heap, value)
    }

    /// Stabilize every global heap reference after initialization.
    fn stabilize_globals(&mut self, heap: &mut Heap) -> RuntimeResult<()> {
        for value in self.globals.values_mut() {
            crate::interpreter::stabilize_value(heap, value).map_err(RuntimeError::new)?;
        }

        Ok(())
    }

    // build a runtime error with the current call stack
    fn make_error(&self, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.get_call_stack_info())
    }

    // collect call stack info for error reporting
    fn get_call_stack_info(&self) -> Vec<FrameInfo> {
        self.interpreter
            .stack()
            .iter()
            .map(|f| {
                let func = self.module.tree.get(f.function);
                let name = self.module.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Borrow the canonical runtime layout table.
    pub fn layout_table(&self) -> &mir::LayoutTable {
        self.module.layouts()
    }

    /// Return the canonical layout id for one MIR type.
    #[cfg(test)]
    pub(crate) fn layout_id_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Option<mir::LayoutId> {
        self.module.layout_id_for_type(ty)
    }

    /// Return the heap allocation layout for one layout id.
    #[cfg(test)]
    pub(crate) fn allocation_layout(
        &self,
        layout_id: mir::LayoutId,
    ) -> crate::Result<destack_heap::AllocationLayout<'_>> {
        self.module.allocation_layout(layout_id)
    }

    /// Borrow the module MIR tree.
    #[cfg(test)]
    pub(crate) fn tree(&self) -> &mir::NodeTree {
        &self.module.tree
    }
}

impl Capture for Isolate {
    type Image = IsolateImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = &'a mut Heap;

    /// Capture one isolate image for the given mode.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Isolate::image(self).map_err(Box::<RuntimeError>::from)
    }

    /// Restore one isolate image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        heap: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Isolate::restore_image(self, heap, image).map_err(Box::<RuntimeError>::from)
    }
}

impl SnapshotCodec for Isolate {
    type Snapshot = IsolateImage;

    /// Encode one isolate image as one snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.clone())
    }

    /// Decode one isolate snapshot back into one image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.clone())
    }
}
