use std::fmt;
use std::ops::Range;

use destack_heap::{HeapEdge, HeapError, HeapReference, HeapReferenceKind, Payload, Release};
use destack_memory::MemoryMap;
use destack_mir::Space;
use destack_native as native;
use destack_native::abi;
use destack_program as program;
use destack_program::Runtime;

use crate::diagnostic::RuntimeError;
use crate::worker::Activation;

use super::{Error, Function, Platform};

/// Runtime state for one active native call.
pub struct Call<'call, 'runtime, 'memory, 'state> {
    /// The executing Program.
    program: &'call program::Program,
    /// Native frame maps for this call.
    frames: native::CodeMap,
    /// The addresses of the running code image.
    code: Range<usize>,
    /// Process-local native functions used to resolve caller return addresses.
    functions: &'call [Option<Function>],
    /// World memory receiving canonical native activation bytes.
    memory: &'call MemoryMap,
    /// The world offsets of the native stack the call's frames live on.
    native_stack: Range<usize>,
    /// Runtime and memory operations available to generated code.
    activation: &'call mut program::Activation<'runtime, 'memory, Activation<'state>>,
    /// Optional profile receiving explicit native observations.
    profile: Option<&'call mut program::Profile>,
    /// A failure raised by one native runtime operation.
    transfer: Option<Transfer>,
    /// The stop produced by the active native call when present.
    stop: Option<Stop>,
    /// Native frames captured from active to caller order.
    captures: Vec<FrameCapture>,
}

/// Source of one retained native stop.
#[derive(Clone, Copy)]
pub(super) enum Stop {
    /// A runtime poll requested host work.
    Poll,
    /// Program execution reached an explicit stop operation.
    Instruction(u32),
}

/// One canonical frame captured while native execution unwinds.
struct FrameCapture {
    /// Canonical frame state identity.
    state: program::FrameStateId,
    /// Retained program operation cursor.
    point: program::ProgramPoint,
    /// Canonical bytes for this frame.
    bytes: Vec<u8>,
    /// The frame's stack segments, moving from world offsets to canonical frame offsets.
    segments: Vec<program::FrameSegment>,
}

/// Nonlocal result retained by one active native call.
pub(super) enum Transfer {
    /// One runtime failure.
    Error(Box<RuntimeError>),
    /// One language panic payload.
    Panic(Option<program::Value>),
    /// One language trap at a linked trap site.
    Trap(abi::Trap),
    /// One captured activation retained for the host.
    Retain,
}

/// Private platform-unwind payload.
pub(super) struct Unwind;

impl<'call, 'runtime, 'memory, 'state> Call<'call, 'runtime, 'memory, 'state> {
    /// Runtime operation table shared by generated native code.
    const RUNTIME: abi::Runtime = abi::Runtime {
        allocate: Self::allocate,
        allocate_repeated: Self::allocate_repeated,
        release: Self::release,
        free: Self::free,
        write_barrier: Self::write_barrier,
        poll: Self::poll,
        stop: Self::stop,
        deopt: Self::deopt,
        panic: Self::panic,
        panic_value: Self::panic_value,
        unwind_classify: Self::unwind_classify,
        unwind_resume: Self::unwind_resume,
        is_subtype: Self::is_subtype,
        profile_increment: Self::increment_profile,
        profile_sample: Self::sample_profile,
        binding_call: Self::binding,
        volatile_read: Self::volatile_read,
        volatile_write: Self::volatile_write,
    };

    /// Create one active native call.
    pub fn new(
        program: &'call program::Program,
        frames: native::CodeMap,
        code: Range<usize>,
        functions: &'call [Option<Function>],
        memory: &'call MemoryMap,
        native_stack: Range<usize>,
        activation: &'call mut program::Activation<'runtime, 'memory, Activation<'state>>,
        profile: Option<&'call mut program::Profile>,
    ) -> Self {
        Self {
            program,
            frames,
            code,
            functions,
            memory,
            native_stack,
            activation,
            profile,
            transfer: None,
            stop: None,
            captures: Vec::new(),
        }
    }

    /// Build the ABI activation borrowing this call.
    pub fn activation(
        &mut self,
        functions: *const usize,
        virtuals: *const *const abi::VirtualTable,
        dynamics: *const *const abi::DynamicTable,
        stack_limit: usize,
        exit: &mut abi::Exit,
    ) -> abi::Activation {
        let call = (self as *mut Self).cast::<abi::Call>();
        let memory = &mut self.activation.memory;

        abi::Activation::new(
            call,
            Self::runtime(),
            functions,
            virtuals,
            dynamics,
            self.memory.base_address() as *mut u8,
            stack_limit,
            memory.constants.native(),
            memory.shared_statics.native(),
            memory.local_statics.native(),
            self.activation.context.reference().bits(),
            self.activation.runtime.poll_address(),
            exit,
        )
    }

    /// Retain the execution context selected by generated code.
    pub fn set_context(&mut self, context: usize) {
        *self.activation.context = program::Context::new(HeapReference::from_bits(context));
    }

    /// Return the native runtime operation table.
    fn runtime() -> *const abi::Runtime {
        std::ptr::from_ref(&Self::RUNTIME)
    }

    /// Take the canonical activation captured by one retained native exit.
    pub fn take_activation(&mut self) -> Result<program::ActivationImage, Box<RuntimeError>> {
        if self.captures.is_empty() {
            return Err(self.internal("retained native exit has no captured frames"));
        }

        // order frames from caller to active and assign canonical byte offsets
        let mut captures = std::mem::take(&mut self.captures);
        captures.reverse();
        let mut byte_len = 0usize;
        let mut offsets = Vec::with_capacity(captures.len());
        let mut layouts = Vec::with_capacity(captures.len());
        for capture in &captures {
            let state = self
                .program
                .frame_state(capture.state)
                .ok_or_else(|| self.internal("captured native frame state is missing"))?;
            let layout = self
                .program
                .frame_layout(state.layout)
                .ok_or_else(|| self.internal("captured native frame layout is missing"))?;
            byte_len = byte_len.next_multiple_of(layout.alignment() as usize);
            offsets.push(byte_len);
            layouts.push(*layout);
            byte_len += layout.byte_len() as usize;
        }
        let mut bytes = vec![0; byte_len];
        let mut segments = Vec::new();

        // concatenate canonical frames and shift their segments to activation offsets
        for (capture, frame_offset) in captures.iter().zip(offsets.iter().copied()) {
            let end = frame_offset + capture.bytes.len();
            bytes[frame_offset..end].copy_from_slice(&capture.bytes);
            segments.extend(
                capture
                    .segments
                    .iter()
                    .map(|segment| program::FrameSegment {
                        source: segment.source.clone(),
                        target: frame_offset + segment.target,
                    }),
            );
        }

        // move frame borrows from world offsets onto encoded activation offsets
        let frames = offsets.iter().copied().zip(&layouts);
        self.program
            .relocate_frame_addresses(frames, &mut bytes, |address| {
                let offset = segments
                    .iter()
                    .find_map(|segment| segment.relocate(address as usize));

                match offset {
                    // encode a captured frame address
                    Some(offset) => {
                        Ok(Some(program::ActivationImage::encode_frame_address(offset)))
                    }
                    // fail a native stack address outside every captured segment
                    None if self.native_stack.contains(&(address as usize)) => {
                        Err(program::Error::StrayFrameAddress { address })
                    }
                    // keep an address outside the native stack
                    None => Ok(None),
                }
            })
            .map_err(|error| Box::<RuntimeError>::from(Error::from(error)))?;
        let frames = captures
            .iter()
            .enumerate()
            .map(|(index, capture)| {
                let return_to = if index == 0 {
                    program::FrameReturn::Root
                } else {
                    program::FrameReturn::Call
                };

                program::FrameImage::new(capture.state, capture.point, return_to)
            })
            .collect::<Vec<_>>();

        let range = self
            .memory
            .allocate_bytes(&bytes, align_of::<program::Word>())
            .map_err(Box::<RuntimeError>::from)?;

        Ok(program::ActivationImage::new(
            program::Completion::Return,
            frames,
            range,
            *self.activation.context,
        ))
    }

    /// Take the nonlocal result raised by generated native code.
    pub(super) fn take_transfer(&mut self) -> Option<Transfer> {
        self.transfer.take()
    }

    /// Take the exact source of one retained native stop.
    pub(super) fn take_stop(&mut self) -> Option<Stop> {
        self.stop.take()
    }

    /// Call one linked runtime binding.
    unsafe extern "C-unwind" fn binding(
        activation: *mut abi::Activation,
        function: u32,
        arguments: *const u64,
        argument_count: usize,
        result: *mut u64,
        result_count: usize,
    ) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let function = program::FunctionId(function);
        let Some(binding) = call.program.function_binding(function) else {
            call.fail(RuntimeError::Internal {
                message: format!("function {function:?} has no runtime binding"),
            });
        };
        if (argument_count != 0 && arguments.is_null()) || (result_count != 0 && result.is_null()) {
            call.fail(RuntimeError::Internal {
                message: format!("native binding call for {function:?} has null value storage"),
            });
        }

        // SAFETY: generated code supplies the exact linked signature ranges
        let arguments = if argument_count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(arguments.cast(), argument_count) }
        };
        let result = if result_count == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(result.cast(), result_count) }
        };
        let memory = call.activation.memory.reborrow();
        let context = *call.activation.context;

        // the native tier carries no detach boundaries; the mounted fiber is current
        let fiber_id = call.activation.runtime.fiber_id();
        if let Err(error) = call
            .activation
            .runtime
            .call_binding(memory, context, fiber_id, binding, arguments, result)
        {
            call.fail_boxed(error);
        }
    }

    /// Allocate one fixed heap object.
    unsafe extern "C-unwind" fn allocate(
        activation: *mut abi::Activation,
        space: abi::Space,
        allocation: u32,
        initialization: abi::AllocationInitialization,
    ) -> usize {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let site = program::AllocationSiteId(allocation);
        let space = Self::space(space);
        let plan = call.activation.memory.allocation_plan(site);
        let payload = Self::payload(initialization);
        let allocation =
            call.activation
                .memory
                .allocate(space, plan, payload, call.program.trace_view());

        match allocation {
            Ok(reference) => reference.bits(),
            Err(error) => call.fail(error),
        }
    }

    /// Allocate one repeated heap backing.
    unsafe extern "C-unwind" fn allocate_repeated(
        activation: *mut abi::Activation,
        space: abi::Space,
        allocation: u32,
        length: usize,
        initialization: abi::AllocationInitialization,
    ) -> usize {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let site = program::AllocationSiteId(allocation);
        let space = Self::space(space);
        let element = call.activation.memory.allocation_plan(site);

        // derive the exact repeated allocation plan
        let trace = match element.trace_map(call.program.trace_view()) {
            Ok(trace) => trace,
            Err(error) => call.fail(error),
        };
        let shape = match element.repeat(&trace, length) {
            Ok(shape) => shape,
            Err(error) => call.fail(error),
        };
        let plan = call.activation.memory.plan_allocation(space, &shape);
        let payload = Self::payload(initialization);
        let allocation =
            call.activation
                .memory
                .allocate(space, plan, payload, call.program.trace_view());

        match allocation {
            Ok(reference) => reference.bits(),
            Err(error) => call.fail(error),
        }
    }

    /// Release one unique heap object.
    unsafe extern "C-unwind" fn release(
        activation: *mut abi::Activation,
        owner: usize,
        frame_map: u32,
        marker: *const u8,
    ) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let edge = match call.activation.memory.edge(owner) {
            Ok(Some(edge)) => edge,
            Ok(None) => return,
            Err(error) => call.fail(error),
        };
        let (plan, byte_len) = match call.activation.memory.release(edge) {
            Ok(Release::Destroy { plan, byte_len }) => (plan, byte_len),
            Ok(Release::Freed | Release::Retained) => return,
            Err(error) => call.fail(error),
        };

        // select the placement-specific generated destructor
        let Some(entry) = call.program.drop_entry(plan.drop) else {
            call.fail(RuntimeError::Internal {
                message: format!("drop {} is undefined", plan.drop.index()),
            });
        };
        let function = match edge {
            HeapEdge::Local(_) => entry.local.get(),
            HeapEdge::Shared(_) => entry.shared.get(),
        };
        let Some(function) = function else {
            call.fail(RuntimeError::Internal {
                message: format!("drop {} has no heap destructor", plan.drop.index()),
            });
        };
        let function = call
            .functions
            .get(function.index())
            .and_then(Option::as_ref);

        // continue this operation in bytecode when its destructor is not installed
        let Some(function) = function else {
            // SAFETY: generated code supplies this operation's exact reconstruction point
            unsafe { Self::deopt(activation, frame_map, marker) }
        };

        // destroy each value from the last to the first, then free the storage
        let count = match plan.value_count(byte_len) {
            Ok(count) => count,
            Err(error) => call.fail(error),
        };
        for index in (0..count).rev() {
            let arguments = [program::Word::from_bits(
                (owner + index * plan.stride()) as u64,
            )];
            let mut result = [];
            // SAFETY: the selected canonical entry uses this activation and Word ABI
            unsafe { function.call(activation, &arguments, &mut result) };
        }

        // SAFETY: the destructors returned to this activation
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = call.activation.memory.free(edge) {
            call.fail(error);
        }
    }

    /// Free one unique heap object holding no live values.
    unsafe extern "C-unwind" fn free(activation: *mut abi::Activation, owner: usize) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let edge = match call.activation.memory.edge(owner) {
            Ok(Some(edge)) => edge,
            Ok(None) => return,
            Err(error) => call.fail(error),
        };

        if let Err(error) = call.activation.memory.free(edge) {
            call.fail(error);
        }
    }

    /// Record one managed-reference write into the allocation holding an address.
    unsafe extern "C-unwind" fn write_barrier(
        activation: *mut abi::Activation,
        object: *const u8,
        offset: usize,
        byte_len: usize,
    ) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        // SAFETY: generated code passes an address inside world memory
        let distance = unsafe { object.offset_from((*activation).memory_base) };
        let owner = match usize::try_from(distance) {
            Ok(owner) => owner,
            Err(_) => call.fail(HeapError::InvalidReference {
                kind: HeapReferenceKind::Heap,
                value: distance as u64,
            }),
        };
        let edge = match call.activation.memory.edge(owner) {
            Ok(Some(edge)) => edge,
            Ok(None) => return,
            Err(error) => call.fail(error),
        };
        let barrier =
            call.activation
                .memory
                .barrier(edge, offset, byte_len, call.program.trace_view());

        if let Err(error) = barrier {
            call.fail(error);
        }
    }

    /// Read one volatile byte range into ordinary native storage.
    unsafe extern "C-unwind" fn volatile_read(
        _activation: *mut abi::Activation,
        source: *const u8,
        destination: *mut u8,
        byte_len: usize,
    ) {
        for offset in 0..byte_len {
            // SAFETY: generated code supplies accessible source and destination ranges
            let value = unsafe { source.add(offset).read_volatile() };
            unsafe { destination.add(offset).write(value) };
        }
    }

    /// Write one ordinary native byte range into volatile storage.
    unsafe extern "C-unwind" fn volatile_write(
        _activation: *mut abi::Activation,
        destination: *mut u8,
        source: *const u8,
        byte_len: usize,
    ) {
        for offset in 0..byte_len {
            // SAFETY: generated code supplies accessible source and destination ranges
            let value = unsafe { source.add(offset).read() };
            unsafe { destination.add(offset).write_volatile(value) };
        }
    }

    /// Return whether one concrete Program type satisfies an expected type.
    unsafe extern "C-unwind" fn is_subtype(
        activation: *mut abi::Activation,
        concrete: u32,
        expected: u32,
    ) -> u32 {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let concrete = program::TypeId(concrete);
        let expected = program::TypeId(expected);
        match call.program.is_subtype(concrete, expected) {
            Ok(is_subtype) => u32::from(is_subtype),
            Err(error) => call.fail(error),
        }
    }

    /// Poll runtime work at one reconstructable native frame.
    unsafe extern "C-unwind" fn poll(
        activation: *mut abi::Activation,
        frame_map: u32,
        anchor: *const u8,
    ) -> ! {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            call.fail_boxed(error);
        }
        call.stop = Some(Stop::Poll);

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.stop(frame_map);

        call.retain_activation()
    }

    /// Stop native execution at one reconstructable native frame.
    unsafe extern "C-unwind" fn stop(
        activation: *mut abi::Activation,
        frame_map: u32,
        operation: u32,
        anchor: *const u8,
    ) -> ! {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            call.fail_boxed(error);
        }
        call.stop = Some(Stop::Instruction(operation));

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.stop(frame_map);

        call.retain_activation()
    }

    /// Deoptimize native execution at one reconstructable frame.
    unsafe extern "C-unwind" fn deopt(
        activation: *mut abi::Activation,
        frame_map: u32,
        anchor: *const u8,
    ) -> ! {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            call.fail_boxed(error);
        }

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.deoptimize(frame_map);

        call.retain_activation()
    }

    /// Increment one explicit profile counter when recording is active.
    unsafe extern "C-unwind" fn increment_profile(activation: *mut abi::Activation, counter: u32) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        if let Some(profile) = call.profile.as_deref_mut() {
            profile.increment_counter(program::CounterId(counter));
        }
    }

    /// Record one explicit profile sample when recording is active.
    unsafe extern "C-unwind" fn sample_profile(
        activation: *mut abi::Activation,
        sampler: u32,
        value: u64,
    ) {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        if let Some(profile) = call.profile.as_deref_mut() {
            profile.record_sample(program::SamplerId(sampler), value);
        }
    }

    /// Capture one native stack in canonical Program layout.
    unsafe fn capture(
        &mut self,
        frame_map: u32,
        anchor: *const u8,
    ) -> Result<(), Box<RuntimeError>> {
        let mut frame_map = frame_map;
        let mut anchor = anchor;

        loop {
            // retain the current frame before following its physical caller link
            let frame = unsafe { self.capture_frame(frame_map, anchor) }?;
            let state = program::FrameStateId(frame.state);
            let function = self
                .program
                .frame_state(state)
                .map(|state| state.point.function());
            let function = function.and_then(|function| {
                self.functions
                    .get(function.index())
                    .and_then(Option::as_ref)
            });
            if !function.is_some_and(Function::is_reconstructable) {
                break;
            }
            let frame_pointer = anchor.wrapping_offset(frame.frame_pointer_offset as isize);
            // SAFETY: native emission preserves frame pointers for every generated function
            let caller_pointer = unsafe { frame_pointer.cast::<usize>().read() };
            // SAFETY: the preserved frame record stores its return address after the caller link
            let return_address = unsafe { frame_pointer.cast::<usize>().add(1).read() };
            let Some((caller_map, caller)) = self.caller(return_address) else {
                break;
            };

            frame_map = caller_map;
            anchor = (caller_pointer as *const u8)
                .wrapping_offset(-(caller.frame_pointer_offset as isize));
        }

        Ok(())
    }

    /// Resolve one generated caller frame from its native return address.
    fn caller(&self, return_address: usize) -> Option<(u32, native::FrameMap)> {
        let sections = self.program.sections();

        for function in self.functions.iter().flatten() {
            let Some(code_offset) = function.code_offset(return_address) else {
                continue;
            };
            let frame = self
                .frames
                .frames(sections)
                .iter()
                .enumerate()
                .find(|(_, frame)| frame.return_offset == code_offset);
            if let Some((index, frame)) = frame {
                return Some((index as u32, *frame));
            }
        }

        None
    }

    /// Copy one native frame into canonical Program layout.
    unsafe fn capture_frame(
        &mut self,
        frame_map: u32,
        anchor: *const u8,
    ) -> Result<native::FrameMap, Box<RuntimeError>> {
        if anchor.is_null() {
            return Err(self.internal("native frame marker is null"));
        }
        let sections = self.program.sections();
        let frame = self
            .frames
            .frame(sections, frame_map)
            .ok_or_else(|| self.internal("native frame map is missing"))?;
        let state = program::FrameStateId(frame.state);
        let linked = self
            .program
            .frame_state(state)
            .ok_or_else(|| self.internal("native frame state is missing"))?;
        let layout = self
            .program
            .frame_layout(linked.layout)
            .ok_or_else(|| self.internal("native frame layout is missing"))?;
        let slots = self.program.frame_slots(layout);
        let values = self.frames.values(sections, frame);
        if slots.len() != values.len() {
            return Err(self.internal("native frame value count does not match its layout"));
        }
        let constants = self.frames.constants(sections);
        let mut bytes = vec![0; layout.byte_len() as usize];
        let mut segments = Vec::new();
        let base = self.memory.base_address();

        // project every physical value location into its canonical frame slot
        for (slot, value) in slots.iter().zip(values) {
            let mut written = vec![false; slot.byte_len as usize];
            for location in self.frames.locations(sections, *value) {
                let target = location.value_offset as usize;
                let byte_len = location.byte_len as usize;
                let target_end = target + byte_len;
                let Some(written) = written.get_mut(target..target_end) else {
                    return Err(self.internal("native frame location exceeds its value"));
                };
                if written.iter().any(|is_written| *is_written) {
                    return Err(self.internal("native frame locations overlap"));
                }
                written.fill(true);
                let frame_target = slot.offset as usize + target;
                let destination = &mut bytes[frame_target..frame_target + byte_len];

                // copy stack-relative or immutable source bytes
                match location.source {
                    native::FrameSource::Stack => {
                        let source = anchor.wrapping_offset(location.source_offset as isize);
                        // SAFETY: the emitted stack map keeps this exact source range live
                        let source = unsafe { std::slice::from_raw_parts(source, byte_len) };
                        destination.copy_from_slice(source);
                        // record the stack segment at its world offset
                        let start = (source.as_ptr() as usize)
                            .checked_sub(base)
                            .ok_or_else(|| self.internal("a native frame outside world memory"))?;
                        segments.push(program::FrameSegment {
                            source: start..start + byte_len,
                            target: frame_target,
                        });
                    }
                    native::FrameSource::Constant => {
                        let start = location.source_offset as usize;
                        let source = &constants[start..start + byte_len];
                        destination.copy_from_slice(source);
                    }
                }
            }
            if written.iter().any(|is_written| !is_written) {
                return Err(self.internal("native frame value is incomplete"));
            }
        }
        let point = linked
            .point
            .operation_point()
            .ok_or_else(|| self.internal("native frame state is not executable"))?;
        self.captures.push(FrameCapture {
            state,
            point,
            bytes,
            segments,
        });

        Ok(frame)
    }

    /// Recover the runtime call owning one ABI activation.
    pub(super) unsafe fn from_activation<'activation>(
        activation: *mut abi::Activation,
    ) -> &'activation mut Self {
        // SAFETY: abi::Activation.call was built from this exact Call type
        let call = unsafe { &mut *(*activation).call.cast::<Self>() };
        let context = unsafe { (*activation).context };
        *call.activation.context = program::Context::new(HeapReference::from_bits(context));

        call
    }

    /// Convert one native ABI space into the program memory domain.
    const fn space(space: abi::Space) -> Space {
        match space {
            abi::Space::Local => Space::Local,
            abi::Space::Shared => Space::Shared,
        }
    }

    /// Select one heap allocation payload.
    const fn payload(initialization: abi::AllocationInitialization) -> Payload<'static> {
        match initialization {
            abi::AllocationInitialization::Zeroed => Payload::Zeroed,
            abi::AllocationInitialization::Uninit => Payload::Uninit,
        }
    }

    /// Raise one runtime operation failure through native cleanup blocks.
    fn fail(&mut self, error: impl Into<Box<RuntimeError>>) -> ! {
        self.fail_boxed(error.into())
    }

    /// Raise one boxed runtime failure through native cleanup blocks.
    fn fail_boxed(&mut self, error: Box<RuntimeError>) -> ! {
        self.transfer = Some(Transfer::Error(error));

        Self::raise()
    }

    /// Return one captured activation without running native cleanup blocks.
    fn retain_activation(&mut self) -> ! {
        self.transfer = Some(Transfer::Retain);

        Self::raise()
    }

    /// Return the trap linked at one instruction address of the running code.
    pub(super) fn trap_at(&self, address: usize) -> Option<abi::Trap> {
        if !self.code.contains(&address) {
            return None;
        }

        // find the trap site at the address's code offset
        let offset = (address - self.code.start) as u32;
        let traps = self.frames.traps(self.program.sections());
        let index = traps
            .binary_search_by_key(&offset, |trap| trap.offset)
            .ok()?;

        Some(traps[index].trap)
    }

    /// Raise one language trap without running native cleanup blocks.
    pub(super) fn trap(&mut self, trap: abi::Trap) -> ! {
        self.transfer = Some(Transfer::Trap(trap));

        Self::raise()
    }

    /// Start one zero-cost platform unwind.
    fn raise() -> ! {
        std::panic::resume_unwind(Box::new(Unwind))
    }

    /// Record one payloadless language panic.
    unsafe extern "C-unwind" fn panic(activation: *mut abi::Activation) -> ! {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        call.transfer = Some(Transfer::Panic(None));

        Self::raise()
    }

    /// Record one typed language panic.
    unsafe extern "C-unwind" fn panic_value(
        activation: *mut abi::Activation,
        ty: u32,
        words: *const u64,
    ) -> ! {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let ty = program::TypeId(ty);
        let byte_len = call.program.type_byte_len(ty).unwrap_or_else(|| {
            call.fail(RuntimeError::Internal {
                message: format!("native panic type {ty:?} has no layout"),
            })
        });
        let word_count = byte_len.div_ceil(program::Word::BYTE_LEN);
        if word_count != 0 && words.is_null() {
            call.fail(RuntimeError::Internal {
                message: "native panic payload has no words".to_string(),
            });
        }

        // copy the payload before unwinding destroys the originating frame
        let words = if word_count == 0 {
            Vec::new()
        } else {
            // SAFETY: generated code supplies the exact payload word range
            unsafe { std::slice::from_raw_parts(words.cast(), word_count).to_vec() }
        };
        let value = call
            .program
            .value(ty, words)
            .unwrap_or_else(|error| call.fail(error));
        call.transfer = Some(Transfer::Panic(Some(value)));

        Self::raise()
    }

    /// Classify the active platform unwind for one cleanup landing pad.
    unsafe extern "C-unwind" fn unwind_classify(
        activation: *mut abi::Activation,
    ) -> abi::UnwindAction {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };

        match call.transfer {
            Some(Transfer::Error(_) | Transfer::Panic(_)) => abi::UnwindAction::Cleanup,
            Some(Transfer::Trap(_) | Transfer::Retain) => abi::UnwindAction::Skip,
            None => std::process::abort(),
        }
    }

    /// Continue one active platform unwind.
    unsafe extern "C-unwind" fn unwind_resume(
        _activation: *mut abi::Activation,
        unwind: *mut abi::Unwind,
    ) -> ! {
        // SAFETY: generated cleanup code passes the active platform unwind object
        unsafe { Platform::resume(unwind) }
    }

    /// Build one internal native capture failure.
    fn internal(&self, message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Return the runtime binding operation.
    pub const fn binding_entry() -> abi::BindingCall {
        Self::binding
    }
}

impl fmt::Debug for Call<'_, '_, '_, '_> {
    /// Format this native call without exposing borrowed runtime state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Call")
            .field("program", &self.program)
            .field("transfer", &self.transfer.is_some())
            .finish_non_exhaustive()
    }
}
