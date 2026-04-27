use std::fmt;

use destack_engine::{self as engine, StaticSpace};
use destack_heap::{AllocationLayout, Heap, HeapError, HeapReference, HeapResult, Payload};
use destack_mir as mir;

use super::{Frame, Interpreter};
use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::options::IsolateOptions;
use crate::program::{ArgumentRange, Function, Layout, Program};
use crate::{FrameInfo, FramePointer, SharedHeap, StackPointer, StaticPointer, Word};

/// Cached state for one interpreter dispatch.
pub(crate) struct DispatchState<'ctx, 'iso> {
    /// Immutable program metadata for this dispatch.
    pub(crate) program: &'iso Program,
    /// Immutable isolate options.
    pub(crate) options: &'iso IsolateOptions,
    /// Mutable static byte arena.
    pub(crate) statics: &'iso mut StaticSpace,
    /// The worker-local heap.
    heap: *mut Heap,
    /// The world-shared heap.
    shared: *const SharedHeap,
    /// Interpreter engine state for this dispatch.
    pub(crate) engine: &'ctx mut Interpreter,

    /// Index of the current frame in the stack.
    pub frame_index: usize,
    /// Whether bounds checks are enabled for this dispatch.
    pub bounds_checks: bool,
    /// Whether null checks are enabled for this dispatch.
    pub null_checks: bool,
    /// Pointer to the current frame for fast access.
    frame: *mut Frame,
    /// Pointer to the current frame layout.
    frame_layout: *const engine::FrameLayout,
    /// Argument pool for the current function.
    argument_pool: *const mir::Value,
    /// Argument pool length.
    argument_pool_len: usize,
}

impl fmt::Debug for DispatchState<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DispatchState")
            .field("frame_index", &self.frame_index)
            .field("argument_pool_len", &self.argument_pool_len)
            .field("bounds_checks", &self.bounds_checks)
            .field("null_checks", &self.null_checks)
            .finish()
    }
}

impl<'ctx, 'iso> DispatchState<'ctx, 'iso> {
    /// Create dispatch state for the current frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        program: &'iso Program,
        options: &'iso IsolateOptions,
        statics: &'iso mut StaticSpace,
        heap: &'iso mut Heap,
        shared: &'iso SharedHeap,
        engine: &'ctx mut Interpreter,
        frame_index: usize,
        argument_pool: &[mir::Value],
    ) -> Result<Self, Error> {
        // resolve check policies
        let mode = options.execution.mode;
        let bounds_checks = options.checks.bounds.is_enabled_for(mode);
        let null_checks = options.checks.null.is_enabled_for(mode);

        // get frame pointer
        // #Safety: frame_index always points at the current frame
        let frame = unsafe { engine.frames.get_unchecked_mut(frame_index) as *mut Frame };

        let frame_layout = unsafe { (*frame).frame_layout };
        let frame_layout = program
            .frame_layout_by_id(frame_layout)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameLayout;

        Ok(Self {
            program,
            options,
            statics,
            heap: heap as *mut Heap,
            shared: shared as *const SharedHeap,
            engine,
            frame_index,
            bounds_checks,
            null_checks,
            frame,
            frame_layout,
            argument_pool: argument_pool.as_ptr(),
            argument_pool_len: argument_pool.len(),
        })
    }

    /// Borrow the program MIR tree.
    #[inline]
    pub(crate) fn tree(&self) -> &mir::Tree {
        &self.program.tree
    }

    /// Return the compiled layout for one MIR type.
    #[inline]
    pub(crate) fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.program.layout(ty).ok_or_else(|| Error::TypeMismatch {
            expected: "compiled layout".to_string(),
            actual: format!("{ty:?}"),
        })
    }

    /// Return the MIR type stored in one SSA value.
    #[inline]
    pub(crate) fn value_type(
        &self,
        value: mir::Value,
    ) -> Result<mir::LocalNodeId<mir::Type>, Error> {
        let region = self
            .frame_layout()
            .value_index(value.0)
            .ok_or(Error::InvalidInstruction)?;

        Ok(self.program.type_for_id(region.ty))
    }

    /// Return the frame region for one SSA value.
    #[inline]
    pub(crate) fn value_region(&self, value: mir::Value) -> Result<&engine::FrameRegion, Error> {
        self.frame_layout()
            .value_index(value.0)
            .ok_or(Error::InvalidInstruction)
    }

    /// Return whether one SSA value is stored as one word.
    #[inline]
    pub(crate) fn value_is_word(&self, value: mir::Value) -> Result<bool, Error> {
        Ok(self.value_region(value)?.is_word)
    }

    /// Return the frame address for one SSA value.
    #[inline]
    pub(crate) fn value_address(&self, value: mir::Value) -> Result<FramePointer, Error> {
        let region = self.value_region(value)?;
        let address = unsafe { (*self.frame).region_address(region) };

        Ok(FramePointer::from_address(address))
    }

    /// Return one value as the operand expected by pointer-style access helpers.
    #[inline]
    pub(crate) fn value_operand(&self, value: mir::Value) -> Result<Word, Error> {
        let region = self.value_region(value)?;
        if region.is_word {
            return Ok(self.get(value));
        }

        Ok(Word::frame_pointer(self.value_address(value)?))
    }

    /// Borrow one SSA value's bytes mutably.
    #[inline]
    pub(crate) fn value_bytes_mut(&mut self, value: mir::Value) -> Result<&mut [u8], Error> {
        let region = self.value_region(value)? as *const engine::FrameRegion;
        let frame = self.current_frame_mut();

        Ok(frame.region_bytes_mut(unsafe { &*region }))
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
                .frames
                .iter()
                .map(|frame| {
                    let func = self.program.tree.get(frame.function);
                    let name = self.program.strings.get(func.name).to_string();
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
        let frame_layout = unsafe { (*self.frame).frame_layout };
        if let Some(frame_layout) = self.program.frame_layout_by_id(frame_layout) {
            self.frame_layout = frame_layout as *const engine::FrameLayout;
        }

        // refresh argument pool
        self.argument_pool = function.argument_pool.as_ptr();
        self.argument_pool_len = function.argument_pool.len();
    }

    /// Borrow the heap for the current block.
    #[inline]
    pub(crate) fn heap_mut(&mut self) -> &mut Heap {
        unsafe { &mut *self.heap }
    }

    /// Allocate one local heap payload from a resolved layout.
    #[inline]
    pub(crate) fn allocate_heap(
        &mut self,
        layout: AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> Result<HeapReference, Error> {
        unsafe { &mut *self.heap }
            .allocate(layout, payload)
            .map_err(Error::from)
    }

    /// Allocate one local heap payload from one program layout id.
    #[inline]
    pub(crate) fn allocate_heap_layout(
        &mut self,
        layout_id: mir::LayoutId,
        payload: Payload<'_>,
    ) -> Result<HeapReference, Error> {
        let layout = self.program.allocation_layout(layout_id)?;

        unsafe { &mut *self.heap }
            .allocate(layout, payload)
            .map_err(Error::from)
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

    /// Read one exact local heap byte range into owned bytes.
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
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        self.execute_intrinsic_with_words(destination, intrinsic, arguments, args)
    }

    /// Move the state to a new frame and function.
    pub(crate) fn enter_frame(&mut self, frame_index: usize, function: &Function) {
        // validate frame index in debug builds
        debug_assert!(
            frame_index < self.engine.frames.len(),
            "frame index out of bounds"
        );

        // update cached frame pointer
        let frame = unsafe { self.engine.frames.get_unchecked_mut(frame_index) };
        self.frame_index = frame_index;
        self.frame = frame as *mut Frame;

        // refresh cached pointers
        self.refresh_for_function(function);
    }

    /// Get the current frame mutably.
    #[inline(always)]
    pub(crate) fn current_frame_mut(&mut self) -> &mut Frame {
        unsafe { &mut *self.frame }
    }

    /// Borrow the current frame layout.
    #[inline(always)]
    pub(crate) fn frame_layout(&self) -> &engine::FrameLayout {
        unsafe { &*self.frame_layout }
    }

    /// Get a frame by index.
    #[inline(always)]
    pub(crate) fn frame_by_index(&self, frame_index: usize) -> Result<&Frame, Error> {
        self.engine
            .frames
            .get(frame_index)
            .ok_or(Error::InvalidHeapReference)
    }

    /// Return whether the interpreter owns one stack byte range.
    #[inline]
    pub(crate) fn owns_stack_range(&self, pointer: StackPointer, byte_len: usize) -> bool {
        self.owns_stack_address_range(pointer.address(), byte_len)
    }

    /// Return whether the interpreter owns one frame byte range.
    #[inline]
    pub(crate) fn owns_frame_range(&self, pointer: FramePointer, byte_len: usize) -> bool {
        self.owns_stack_address_range(pointer.address(), byte_len)
    }

    /// Return whether the interpreter owns one stack address range.
    #[inline]
    fn owns_stack_address_range(&self, address: usize, byte_len: usize) -> bool {
        let start = self.engine.stack.as_ptr() as usize;
        let end = match address.checked_add(byte_len) {
            Some(end) => end,
            None => return false,
        };
        let stack_end = start.saturating_add(self.engine.stack.len());

        start <= address && end <= stack_end
    }

    /// Allocate bytes owned by the current frame.
    pub(crate) fn allocate_stack(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> Result<usize, Error> {
        let base = super::interpreter::align_stack_bytes(self.engine.stack.len(), alignment)
            .ok_or(Error::StackOverflow)?;
        let end = base.checked_add(byte_len).ok_or(Error::StackOverflow)?;
        if end > self.options.limits.max_stack_bytes {
            return Err(Error::StackOverflow);
        }

        self.engine.stack.resize(end, 0);
        self.current_frame_mut().extend_bytes_to(end);

        Ok(unsafe { self.engine.stack.as_mut_ptr().add(base) as usize })
    }

    /// Return whether one static pointer targets VM-owned static memory.
    #[inline]
    pub(crate) fn owns_static_range(&self, pointer: StaticPointer, byte_len: usize) -> bool {
        self.statics.owns_pointer_range(pointer, byte_len)
            || self.program.owns_static_range(pointer, byte_len)
    }

    /// Borrow static bytes for one global.
    #[inline]
    pub(crate) fn static_bytes(&self, global: mir::LocalNodeId<mir::Global>) -> Option<&[u8]> {
        self.statics
            .bytes(self.program.static_id(global))
            .or_else(|| self.program.static_bytes(global))
    }

    /// Return the static pointer for one global.
    #[inline]
    pub(crate) fn static_pointer(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticPointer> {
        self.statics
            .ptr(self.program.static_id(global))
            .or_else(|| self.program.static_pointer(global))
    }

    /// Get value by SSA id.
    #[inline(always)]
    pub(crate) fn get(&self, v: mir::Value) -> Word {
        let index = v.0 as usize;
        let layout = self.frame_layout();
        debug_assert!(
            index < layout.values.len(),
            "ssa value out of bounds: {v:?}"
        );
        let region = unsafe { layout.values.get_unchecked(index) };

        unsafe { (*self.frame).read_operand(region) }
    }

    /// Write one word by SSA id.
    #[inline(always)]
    pub(crate) fn set_word(&mut self, v: mir::Value, val: Word) {
        let index = v.0 as usize;
        let layout = self.frame_layout();
        debug_assert!(
            index < layout.values.len(),
            "ssa value out of bounds: {v:?}"
        );
        let region = unsafe { layout.values.get_unchecked(index) };

        debug_assert!(region.is_word, "attempted word write into frame bytes");
        unsafe { (*self.frame).write_word(region, val) }
    }

    /// Copy one local variable into one SSA value.
    #[inline(always)]
    pub(crate) fn move_local_to_value_by_index(
        &mut self,
        local_index: u32,
        value: mir::Value,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let local_region = layout
            .locals
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;
        let value_region = layout
            .value_index(value.0)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*local_region }, unsafe { &*value_region })
    }

    /// Copy one SSA value into one local variable.
    #[inline(always)]
    pub(crate) fn move_value_to_local_by_index(
        &mut self,
        value: mir::Value,
        local_index: u32,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let value_region = layout
            .value_index(value.0)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;
        let local_region = layout
            .locals
            .get(local_index as usize)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*value_region }, unsafe { &*local_region })
    }

    /// Copy one SSA value into another SSA value.
    #[inline(always)]
    pub(crate) fn move_value_to_value(
        &mut self,
        source: mir::Value,
        destination: mir::Value,
    ) -> Result<(), Error> {
        let layout = self.frame_layout();
        let source_region = layout
            .value_index(source.0)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;
        let destination_region = layout
            .value_index(destination.0)
            .ok_or(Error::InvalidInstruction)?
            as *const engine::FrameRegion;

        self.move_frame_region(unsafe { &*source_region }, unsafe { &*destination_region })
    }

    /// Copy bytes between two frame regions in the current frame.
    #[inline(always)]
    fn move_frame_region(
        &mut self,
        source: &engine::FrameRegion,
        destination: &engine::FrameRegion,
    ) -> Result<(), Error> {
        if source.byte_len != destination.byte_len || source.is_word != destination.is_word {
            return Err(Error::TypeMismatch {
                expected: format!(
                    "{} bytes, word={}",
                    destination.byte_len, destination.is_word
                ),
                actual: format!("{} bytes, word={}", source.byte_len, source.is_word),
            });
        }

        let frame = self.current_frame_mut();
        if source.is_word {
            let value = frame.read_word(source);
            frame.write_word(destination, value);

            return Ok(());
        }

        unsafe {
            std::ptr::copy(
                frame.region_address(source) as *const u8,
                frame.region_address(destination) as *mut u8,
                source.byte_len as usize,
            );
        }

        Ok(())
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
}
