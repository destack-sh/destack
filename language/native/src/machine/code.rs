use std::fmt;

use destack_heap as heap;
use destack_mir as mir;
use destack_program::{
    EntryPoint, FunctionId, Program, RuntimeCall, Value, native as program_native,
};

use crate::{
    Continuation, Entry, Error, LibraryHandle, MemoryMapping, NativeContext, NativeContinuation,
    NativeEntry, NativeExit, NativeExitKind, NativeResumeEntry, NativeTrap, NativeValue, Outcome,
    ResumeEntry,
};

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The process-local native image backing this code.
    image: CodeImage,
    /// Native code map for safepoints and deoptimization.
    map: program_native::CodeMap,
    /// Native entries keyed by program function id.
    function: Vec<Option<Entry>>,
    /// Native resume entries keyed by frame state id.
    resume: Vec<Option<ResumeEntry>>,
}

impl Code {
    /// Create one native code table.
    pub fn new(
        image: CodeImage,
        map: program_native::CodeMap,
        function: Vec<Option<Entry>>,
        resume: Vec<Option<ResumeEntry>>,
    ) -> Self {
        Self {
            image,
            map,
            function,
            resume,
        }
    }

    /// Link one durable native code payload into process-local native code.
    pub fn link(
        image: CodeImage,
        native: &program_native::Code,
        mut function: impl FnMut(&str) -> Option<NativeEntry>,
        mut resume: impl FnMut(&str) -> Option<NativeResumeEntry>,
    ) -> Result<Self, Error> {
        let entry = &native.entry;
        let mut code = Self {
            image,
            map: native.map.clone(),
            function: vec![None; entry.function_entry().len()],
            resume: vec![None; entry.resume_entry().len()],
        };

        for entry in entry.function_entry().iter().flatten() {
            let Some(function) = function(&entry.symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: entry.symbol.clone(),
                });
            };

            code.set_entry(entry.function, function);
        }

        for entry in entry.resume_entry().iter().flatten() {
            let Some(resume) = resume(&entry.symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: entry.symbol.clone(),
                });
            };

            code.set_resume(entry.frame_state, resume);
        }

        Ok(code)
    }

    /// Borrow the process-local native image backing this code.
    pub const fn image(&self) -> &CodeImage {
        &self.image
    }

    /// Borrow the native code map.
    pub const fn map(&self) -> &program_native::CodeMap {
        &self.map
    }

    /// Return one native entry for one function.
    pub fn entry(&self, function: FunctionId) -> Option<&Entry> {
        self.function.get(function.index()).and_then(Option::as_ref)
    }

    /// Return one native resume entry for one frame state.
    pub fn resume_entry(&self, frame_state: mir::FrameStateId) -> Option<&ResumeEntry> {
        self.resume
            .get(frame_state.0 as usize)
            .and_then(Option::as_ref)
    }

    /// Insert one native function pointer.
    pub fn set_entry(&mut self, function: FunctionId, entry: NativeEntry) {
        let index = function.index();
        if index >= self.function.len() {
            self.function.resize_with(index + 1, || None);
        }

        self.function[index] = Some(Entry::new(function, entry));
    }

    /// Insert one native resume function pointer.
    pub fn set_resume(&mut self, frame_state: mir::FrameStateId, entry: NativeResumeEntry) {
        let index = frame_state.0 as usize;
        if index >= self.resume.len() {
            self.resume.resize_with(index + 1, || None);
        }

        self.resume[index] = Some(ResumeEntry::new(frame_state, entry));
    }

    /// Return native function entries in dense function id order.
    pub fn function_table(&self) -> &[Option<Entry>] {
        &self.function
    }

    /// Return native resume entries in dense frame state id order.
    pub fn resume_table(&self) -> &[Option<ResumeEntry>] {
        &self.resume
    }

    /// Resolve one runtime entry name into one entrypoint.
    pub fn entry_by_name(&self, program: &Program, name: &str) -> Result<EntryPoint, Error> {
        let Some(function) = program.function_id_by_name(name) else {
            return Err(Error::EntryNotFound {
                name: name.to_string(),
            });
        };

        Ok(EntryPoint::from(function))
    }

    /// Run one native entrypoint.
    pub fn run(
        &self,
        context: &mut RuntimeCall<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome, Error> {
        let Some(entry) = self.entry_point(entry) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            });
        };

        // build native ABI inputs
        let args = args.iter().map(NativeValue::from_value).collect::<Vec<_>>();
        let mut exit = NativeExit::default();
        let mut context = Self::native_context(context, &mut exit);
        let mut out = NativeValue::VOID;

        // enter generated native code
        let code = entry.call(&mut context, &args, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Resume one native continuation.
    pub fn resume(
        &self,
        context: &mut RuntimeCall<'_>,
        continuation: Continuation,
        value: Value,
    ) -> Result<Outcome, Error> {
        let frame = continuation
            .image
            .frames
            .last()
            .ok_or(Error::EmptyContinuation)?;
        let Some(entry) = self.resume_entry(frame.frame_state) else {
            return Err(Error::ResumeEntryNotFound {
                frame_state: frame.frame_state,
            });
        };

        // build native continuation input
        let frames = continuation.abi_frames()?;
        let continuation = NativeContinuation {
            frames: frames.as_ptr(),
            frame_count: frames.len(),
        };
        let received = NativeValue::from_value(&value);
        let mut exit = NativeExit::default();
        let mut context = Self::native_context(context, &mut exit);
        let mut out = NativeValue::VOID;

        // enter generated native resume code
        let code = entry.call(&mut context, continuation, received, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Visit mutable heap root slots from one native continuation.
    pub fn visit_continuation_root_slots(
        &self,
        program: &Program,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Error> {
        continuation.visit_root_slots(program, visit)
    }

    /// Return one native entry for one program entrypoint.
    fn entry_point(&self, entry: EntryPoint) -> Option<&Entry> {
        self.entry(entry.function())
    }

    /// Build one native ABI context.
    fn native_context(context: &mut RuntimeCall<'_>, exit: &mut NativeExit) -> NativeContext {
        NativeContext::new(
            context.state.as_ptr().cast(),
            context.memory.constant_space.as_native_constants(),
            context.memory.shared_static.as_native_statics(),
            context.memory.local_static.as_native_statics(),
            exit,
        )
    }

    /// Return one yielded outcome from native continuation state.
    fn yielded_outcome(&self, out: NativeValue, exit: NativeExit) -> Result<Outcome, Error> {
        if exit.continuation.is_empty() {
            return Err(Error::YieldedWithoutContinuation {
                safepoint: exit.safepoint,
            });
        }

        let value = out.to_value().map_err(Error::Value)?;

        // SAFETY: generated native code owns the ABI contract for the exit continuation
        let continuation =
            unsafe { exit.continuation.to_native() }.map_err(Error::InvalidContinuation)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Return one deoptimized outcome from native materialization.
    fn deoptimized_outcome(&self, exit: NativeExit) -> Result<Outcome, Error> {
        if exit.materialization.is_empty() {
            return Err(Error::DeoptimizedWithoutMaterialization {
                safepoint: exit.safepoint,
            });
        }

        // SAFETY: generated native code owns the ABI contract for the exit materialization
        let continuation = unsafe { exit.materialization.to_continuation_image() }
            .map_err(Error::InvalidMaterialization)?;

        Ok(Outcome::Deoptimized { continuation })
    }

    /// Return one native outcome from native exit code and payloads.
    fn outcome_from_exit(
        &self,
        code: u32,
        out: NativeValue,
        exit: NativeExit,
    ) -> Result<Outcome, Error> {
        let kind = NativeExitKind::try_from(code).map_err(Error::InvalidExit)?;

        match kind {
            NativeExitKind::Completed => {
                let value = out.to_value().map_err(Error::Value)?;

                Ok(Outcome::Completed { value })
            }
            NativeExitKind::Yielded => self.yielded_outcome(out, exit),
            NativeExitKind::Trapped => {
                let trap = NativeTrap::try_from(exit.trap).map_err(Error::InvalidTrap)?;

                Err(Error::Trapped { trap })
            }
            NativeExitKind::Deoptimized => self.deoptimized_outcome(exit),
            NativeExitKind::Panicked => {
                let payload = exit.payload.to_value().map_err(Error::Value)?;

                Err(Error::Panicked { payload })
            }
        }
    }
}

/// Process-local native image backing callable code pointers.
#[derive(Debug, Clone)]
pub enum CodeImage {
    /// Native symbols are already resident in this process.
    Resident,
    /// Native symbols are owned by one loaded library handle.
    Library(LibraryHandle),
    /// Native symbols are owned by one executable memory mapping.
    Object(MemoryMapping),
}

impl CodeImage {
    /// Create one resident code image.
    pub const fn resident() -> Self {
        Self::Resident
    }

    /// Create one loaded library code image.
    pub fn library(owner: impl fmt::Debug + Send + Sync + 'static) -> Self {
        Self::Library(LibraryHandle::new(owner))
    }

    /// Create one mapped object code image.
    pub fn object(
        base: usize,
        byte_len: usize,
        owner: impl fmt::Debug + Send + Sync + 'static,
    ) -> Self {
        Self::Object(MemoryMapping::new(base, byte_len, owner))
    }
}
