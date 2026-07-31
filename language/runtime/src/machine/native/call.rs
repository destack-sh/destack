use std::fmt;
use std::ops::Range;

use destack_heap::{HeapEdge, HeapReference, Payload, SharedHeapReference};
use destack_memory::MemoryMap;
use destack_mir::Space;
use destack_native as native;
use destack_native::abi;
use destack_program as program;
use destack_program::Runtime;

use crate::diagnostic::RuntimeError;
use crate::worker::Activation;

use super::{Error, Function};

/// Runtime owner for one active native call.
pub struct Call<'program, 'runtime, 'memory, 'state> {
    /// The executing Program.
    program: &'program program::Program,
    /// Native frame maps for this call.
    frames: native::CodeMap,
    /// Process-local native functions used to resolve caller return addresses.
    functions: &'program [Option<Function>],
    /// World memory receiving canonical native activation bytes.
    memory: &'program MemoryMap,
    /// Runtime and memory operations available to generated code.
    activation: &'program mut program::Activation<'runtime, 'memory, Activation<'state>>,
    /// The panic payload copied before native frames return.
    panic: Option<program::Value>,
    /// A failure raised by one native runtime operation.
    error: Option<Box<RuntimeError>>,
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
    Instruction,
}

/// One canonical frame captured while native execution unwinds.
struct FrameCapture {
    /// Canonical frame state identity.
    state: program::FrameStateId,
    /// Retained program operation cursor.
    point: program::ProgramPoint,
    /// Canonical bytes for this frame.
    bytes: Vec<u8>,
    /// Physical stack ranges projected into canonical frame offsets.
    ranges: Vec<FrameRange>,
}

/// One physical stack range and its canonical frame offset.
struct FrameRange {
    /// Native address range containing this value piece.
    source: Range<usize>,
    /// Byte offset inside the canonical frame.
    target: usize,
}

impl<'program, 'runtime, 'memory, 'state> Call<'program, 'runtime, 'memory, 'state> {
    /// Create one active native call.
    pub fn new(
        program: &'program program::Program,
        frames: native::CodeMap,
        functions: &'program [Option<Function>],
        memory: &'program MemoryMap,
        activation: &'program mut program::Activation<'runtime, 'memory, Activation<'state>>,
    ) -> Self {
        Self {
            program,
            frames,
            functions,
            memory,
            activation,
            panic: None,
            error: None,
            stop: None,
            captures: Vec::new(),
        }
    }

    /// Build the ABI activation borrowing this call.
    pub fn activation(&mut self, exit: &mut abi::Exit) -> abi::Activation {
        let call = (self as *mut Self).cast::<abi::Call>();
        let memory = &mut self.activation.memory;

        abi::Activation::new(
            call,
            memory.constant_space.native(self.program.sections()),
            memory.shared_static.native(),
            memory.local_static.native(),
            self.activation.runtime.poll_address(),
            exit,
        )
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
        let mut ranges = Vec::new();

        // concatenate canonical frames and retain their physical address projections
        for (capture, frame_offset) in captures.iter().zip(offsets.iter().copied()) {
            let end = frame_offset + capture.bytes.len();
            bytes[frame_offset..end].copy_from_slice(&capture.bytes);
            ranges.extend(capture.ranges.iter().map(|range| FrameRange {
                source: range.source.clone(),
                target: frame_offset + range.target,
            }));
        }

        // replace native stack pointers with canonical activation offsets
        for (frame_offset, layout) in offsets.iter().copied().zip(&layouts) {
            for slot in self.program.frame_slots(layout) {
                let start = frame_offset + slot.offset as usize;
                let end = start + slot.byte_len as usize;
                let value = &mut bytes[start..end];
                let mut is_valid = true;
                self.program
                    .visit_byte_frame_addresses(slot.ty, value, &mut |address| {
                        let Some(word) = program::Word::from_bytes(address) else {
                            is_valid = false;

                            return Ok(());
                        };
                        if word.is_nullish() {
                            return Ok(());
                        }
                        let pointer = word.bits() as usize;
                        let Some(range) =
                            ranges.iter().find(|range| range.source.contains(&pointer))
                        else {
                            is_valid = false;

                            return Ok(());
                        };
                        let target = range.target + pointer - range.source.start;
                        let word = program::Word::from_bits((target + 2) as u64);
                        address.copy_from_slice(&word.to_bytes());

                        Ok(())
                    })
                    .map_err(|error| Box::<RuntimeError>::from(Error::from(error)))?;
                if !is_valid {
                    return Err(self.internal("native frame contains an unknown frame pointer"));
                }
            }
        }
        let frames = captures
            .iter()
            .enumerate()
            .map(|(index, capture)| {
                let link = if index == 0 {
                    program::FrameLink::Root
                } else {
                    program::FrameLink::Call
                };

                program::FrameImage::new(capture.state, capture.point, link)
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
        ))
    }

    /// Take one panic payload copied by generated native code.
    pub fn take_panic(&mut self) -> Option<program::Value> {
        self.panic.take()
    }

    /// Take one native runtime operation failure.
    pub fn take_error(&mut self) -> Option<Box<RuntimeError>> {
        self.error.take()
    }

    /// Take the exact source of one retained native stop.
    pub(super) fn take_stop(&mut self) -> Option<Stop> {
        self.stop.take()
    }

    /// Call one linked runtime binding.
    unsafe extern "C" fn binding(
        activation: *mut abi::Activation,
        function: u32,
        arguments: *const u64,
        argument_count: usize,
        result: *mut u64,
        result_count: usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let function = program::FunctionId(function);
        let Some(binding) = call.program.function_binding(function) else {
            return call.fail(RuntimeError::Internal {
                message: format!("function {function:?} has no runtime binding"),
            });
        };
        if (argument_count != 0 && arguments.is_null()) || (result_count != 0 && result.is_null()) {
            return call.fail(RuntimeError::Internal {
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
        match call
            .activation
            .runtime
            .call_binding(memory, binding, arguments, result)
        {
            Ok(()) => abi::RuntimeStatus::Continue.code(),
            Err(error) => call.fail_boxed(error),
        }
    }

    /// Allocate one fixed heap object.
    unsafe extern "C" fn allocate(
        activation: *mut abi::Activation,
        space: abi::Space,
        allocation: u32,
        initialization: abi::AllocationInitialization,
        result: *mut usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        if result.is_null() {
            return call.fail(RuntimeError::Internal {
                message: "native allocation has no result storage".to_string(),
            });
        }
        let site = program::AllocationSiteId(allocation);
        let space = Self::space(space);
        let Some(plan) = call.activation.memory.allocation_plan(site) else {
            return call.fail(RuntimeError::Internal {
                message: format!("native allocation site {site:?} has no plan"),
            });
        };
        let payload = Self::payload(initialization);
        let allocation =
            call.activation
                .memory
                .allocate(space, plan, payload, call.program.trace_view());

        // return the stable space-relative reference bits
        match allocation {
            Ok(reference) => {
                // SAFETY: generated code supplies writable pointer-width result storage
                unsafe { result.write(reference.bits()) };

                abi::RuntimeStatus::Continue.code()
            }
            Err(error) => call.fail(error),
        }
    }

    /// Allocate one repeated heap backing.
    unsafe extern "C" fn allocate_slice(
        activation: *mut abi::Activation,
        space: abi::Space,
        allocation: u32,
        length: usize,
        initialization: abi::AllocationInitialization,
        result: *mut usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        if result.is_null() {
            return call.fail(RuntimeError::Internal {
                message: "native slice allocation has no result storage".to_string(),
            });
        }
        let site = program::AllocationSiteId(allocation);
        let space = Self::space(space);
        let Some(element) = call.activation.memory.allocation_plan(site) else {
            return call.fail(RuntimeError::Internal {
                message: format!("native allocation site {site:?} has no plan"),
            });
        };

        // derive the exact repeated allocation plan
        let trace = match element.trace_map(call.program.trace_view()) {
            Ok(trace) => trace,
            Err(error) => return call.fail(error),
        };
        let shape = match element.repeat(&trace, length) {
            Ok(shape) => shape,
            Err(error) => return call.fail(error),
        };
        let plan = call.activation.memory.plan_allocation(space, &shape);
        let payload = Self::payload(initialization);
        let allocation =
            call.activation
                .memory
                .allocate(space, plan, payload, call.program.trace_view());

        // return the stable space-relative reference bits
        match allocation {
            Ok(reference) => {
                // SAFETY: generated code supplies writable pointer-width result storage
                unsafe { result.write(reference.bits()) };

                abi::RuntimeStatus::Continue.code()
            }
            Err(error) => call.fail(error),
        }
    }

    /// Release one unique heap object.
    unsafe extern "C" fn free(
        activation: *mut abi::Activation,
        space: abi::Space,
        value: usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let edge = Self::edge(space, value);

        match call.activation.memory.free(edge) {
            Ok(()) => abi::RuntimeStatus::Continue.code(),
            Err(error) => call.fail(error),
        }
    }

    /// Pin one managed heap object.
    unsafe extern "C" fn pin(
        activation: *mut abi::Activation,
        space: abi::Space,
        value: usize,
        result: *mut usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        if result.is_null() {
            return call.fail(RuntimeError::Internal {
                message: "native pin has no result storage".to_string(),
            });
        }
        let edge = Self::edge(space, value);

        match call.activation.memory.pin(edge) {
            Ok(reference) => {
                // SAFETY: generated code supplies writable pointer-width result storage
                unsafe { result.write(reference.bits()) };

                abi::RuntimeStatus::Continue.code()
            }
            Err(error) => call.fail(error),
        }
    }

    /// Release one managed heap pin.
    unsafe extern "C" fn unpin(
        activation: *mut abi::Activation,
        space: abi::Space,
        value: usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let edge = Self::edge(space, value);

        match call.activation.memory.unpin(edge) {
            Ok(()) => abi::RuntimeStatus::Continue.code(),
            Err(error) => call.fail(error),
        }
    }

    /// Record one managed-reference write.
    unsafe extern "C" fn write_barrier(
        activation: *mut abi::Activation,
        space: abi::Space,
        object: usize,
        offset: usize,
        byte_len: usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let edge = Self::edge(space, object);
        let barrier =
            call.activation
                .memory
                .barrier(edge, offset, byte_len, call.program.trace_view());

        match barrier {
            Ok(()) => abi::RuntimeStatus::Continue.code(),
            Err(error) => call.fail(error),
        }
    }

    /// Poll runtime work at one reconstructable native frame.
    unsafe extern "C" fn poll(
        activation: *mut abi::Activation,
        frame_map: u32,
        anchor: *const u8,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            return call.fail_boxed(error);
        }
        call.stop = Some(Stop::Poll);

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.stop(frame_map);

        abi::RuntimeStatus::Exit.code()
    }

    /// Stop native execution at one reconstructable native frame.
    unsafe extern "C" fn stop(
        activation: *mut abi::Activation,
        frame_map: u32,
        anchor: *const u8,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            return call.fail_boxed(error);
        }
        call.stop = Some(Stop::Instruction);

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.stop(frame_map);

        abi::RuntimeStatus::Exit.code()
    }

    /// Deoptimize native execution at one reconstructable frame.
    unsafe extern "C" fn deopt(
        activation: *mut abi::Activation,
        frame_map: u32,
        anchor: *const u8,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes its active frame marker
        let call = unsafe { Self::from_activation(activation) };
        if let Err(error) = unsafe { call.capture(frame_map, anchor) } {
            return call.fail_boxed(error);
        }
        call.stop = Some(Stop::Poll);
        call.activation.runtime.deoptimize();

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.stop(frame_map);

        abi::RuntimeStatus::Exit.code()
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
            let function = self
                .functions
                .get(frame.function as usize)
                .and_then(Option::as_ref);
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
            let Some(return_offset) = function.return_offset(return_address) else {
                continue;
            };
            let frame = self
                .frames
                .frames(sections)
                .iter()
                .enumerate()
                .find(|(_, frame)| {
                    frame.function == function.function.0 && frame.return_offset == return_offset
                });
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
        if linked.point.function().index() != frame.function as usize {
            return Err(self.internal("native frame map selects a different function"));
        }
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
        let mut ranges = Vec::new();

        // project every physical value piece into its canonical frame slot
        for (slot, value) in slots.iter().zip(values) {
            let mut written = vec![false; slot.byte_len as usize];
            for location in self.frames.locations(sections, *value) {
                let target = location.target_offset as usize;
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
                        let start = source.as_ptr() as usize;
                        ranges.push(FrameRange {
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
        let point = program::ProgramPoint::new(program::FunctionId(frame.function), frame.resume);
        self.captures.push(FrameCapture {
            state,
            point,
            bytes,
            ranges,
        });

        Ok(frame)
    }

    /// Record one payloadless language panic.
    unsafe extern "C" fn panic(activation: *mut abi::Activation) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        call.panic = None;

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.panic();

        abi::RuntimeStatus::Exit.code()
    }

    /// Copy one typed language panic payload into this call.
    unsafe extern "C" fn panic_value(
        activation: *mut abi::Activation,
        ty: u32,
        words: *const u64,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let ty = program::TypeId(ty);
        let Some(byte_len) = call.program.type_byte_len(ty) else {
            call.error = Some(Error::TypeMissing { ty }.into());

            // SAFETY: the active activation owns one live exit record
            let exit = unsafe { &mut *(*activation).exit };
            exit.panic();

            return abi::RuntimeStatus::Exit.code();
        };

        // SAFETY: generated code keeps the exact typed payload words live for this callback
        let word_count = byte_len.div_ceil(program::Word::BYTE_LEN);
        let words = unsafe { std::slice::from_raw_parts(words.cast(), word_count) };

        match call.program.value(ty, words.iter().copied()) {
            Ok(payload) => call.panic = Some(payload),
            Err(error) => call.error = Some(Error::from(error).into()),
        }

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };
        exit.panic();

        abi::RuntimeStatus::Exit.code()
    }

    /// Recover the runtime call owning one ABI activation.
    unsafe fn from_activation<'call>(activation: *mut abi::Activation) -> &'call mut Self {
        // SAFETY: abi::Activation.call was built from this exact Call type
        unsafe { &mut *(*activation).call.cast::<Self>() }
    }

    /// Build one heap edge from stable ABI bits.
    const fn edge(space: abi::Space, bits: usize) -> HeapEdge {
        match space {
            abi::Space::Local => HeapEdge::Local(HeapReference::from_bits(bits)),
            abi::Space::Shared => HeapEdge::Shared(SharedHeapReference::from_bits(bits)),
        }
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

    /// Retain one runtime operation failure and stop native execution.
    fn fail(&mut self, error: impl Into<Box<RuntimeError>>) -> abi::RuntimeStatusCode {
        self.fail_boxed(error.into())
    }

    /// Retain one boxed runtime failure and stop native execution.
    fn fail_boxed(&mut self, error: Box<RuntimeError>) -> abi::RuntimeStatusCode {
        self.error = Some(error);

        abi::RuntimeStatus::Exit.code()
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

    /// Return the fixed allocation operation.
    pub const fn allocate_entry() -> abi::Allocate {
        Self::allocate
    }

    /// Return the repeated allocation operation.
    pub const fn allocate_slice_entry() -> abi::AllocateSlice {
        Self::allocate_slice
    }

    /// Return the unique release operation.
    pub const fn free_entry() -> abi::Free {
        Self::free
    }

    /// Return the heap pin operation.
    pub const fn pin_entry() -> abi::Pin {
        Self::pin
    }

    /// Return the heap unpin operation.
    pub const fn unpin_entry() -> abi::Unpin {
        Self::unpin
    }

    /// Return the managed write-barrier operation.
    pub const fn write_barrier_entry() -> abi::WriteBarrier {
        Self::write_barrier
    }

    /// Return the runtime poll operation.
    pub const fn poll_entry() -> abi::Poll {
        Self::poll
    }

    /// Return the debugger stop operation.
    pub const fn stop_entry() -> abi::Stop {
        Self::stop
    }

    /// Return the native deoptimization operation.
    pub const fn deopt_entry() -> abi::Deopt {
        Self::deopt
    }

    /// Return the payloadless panic runtime operation.
    pub const fn panic_entry() -> abi::Panic {
        Self::panic
    }

    /// Return the typed panic runtime operation.
    pub const fn panic_value_entry() -> abi::PanicValue {
        Self::panic_value
    }
}

impl fmt::Debug for Call<'_, '_, '_, '_> {
    /// Format this native call without exposing borrowed runtime state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Call")
            .field("program", &self.program)
            .field("panic", &self.panic)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
