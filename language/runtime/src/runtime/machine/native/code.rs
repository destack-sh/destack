use std::fmt;

use destack_program::native::{
    NativeContext, NativeContinuation, NativeEntry, NativeExit, NativeExitKind, NativeFrame,
    NativeResumeEntry, NativeTrap, NativeValue,
};
use destack_program::{
    Continuation, EntryPoint, FrameStateId, FunctionId, Program, ProgramActivation, ProgramPoint,
    StopReason, Value, native,
};

use super::{Entry, Error, LibraryHandle, MemoryMapping, Outcome, ResumeEntry};

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The process-local native image backing this code.
    image: CodeImage,
    /// Native code map for safepoints and deoptimization.
    map: native::CodeMap,
    /// Native entries keyed by program function id.
    function: Vec<Option<Entry>>,
    /// Native resume entries keyed by frame state id.
    resume: Vec<Option<ResumeEntry>>,
    /// Program points keyed by native safepoint id.
    safepoint_points: Vec<Option<ProgramPoint>>,
}

impl Code {
    /// Create one native code table.
    pub fn new(
        image: CodeImage,
        map: native::CodeMap,
        function: Vec<Option<Entry>>,
        resume: Vec<Option<ResumeEntry>>,
        safepoint_points: Vec<Option<ProgramPoint>>,
    ) -> Self {
        Self {
            image,
            map,
            function,
            resume,
            safepoint_points,
        }
    }

    /// Link one durable native code payload into process-local native code.
    pub fn link(
        image: CodeImage,
        program: &Program,
        native: &native::Code,
        mut function: impl FnMut(&str) -> Option<NativeEntry>,
        mut resume: impl FnMut(&str) -> Option<NativeResumeEntry>,
    ) -> Result<Self, Error> {
        let entries = &native.entries;
        let sections = program.sections();
        let safepoint_points = native
            .map
            .safepoints(sections)
            .iter()
            .map(|safepoint| {
                safepoint
                    .get()
                    .and_then(|safepoint| program.point_for_frame_state(safepoint.frame_state))
            })
            .collect::<Vec<_>>();
        let mut code = Self {
            image,
            map: native.map.clone(),
            function: vec![None; entries.functions(sections).len()],
            resume: vec![None; entries.resumes(sections).len()],
            safepoint_points,
        };

        for entry in entries
            .functions(sections)
            .iter()
            .filter_map(|entry| entry.get())
        {
            let Some(symbol) = program.string(entry.symbol) else {
                return Err(Error::ProgramStringMissing {
                    string: entry.symbol,
                });
            };
            let Some(function) = function(symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: symbol.to_owned(),
                });
            };

            code.set_entry(entry.function, function);
        }

        for entry in entries
            .resumes(sections)
            .iter()
            .filter_map(|entry| entry.get())
        {
            let Some(symbol) = program.string(entry.symbol) else {
                return Err(Error::ProgramStringMissing {
                    string: entry.symbol,
                });
            };
            let Some(resume) = resume(symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: symbol.to_owned(),
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
    pub const fn map(&self) -> &native::CodeMap {
        &self.map
    }

    /// Return one native entry for one function.
    pub fn entry(&self, function: FunctionId) -> Option<&Entry> {
        self.function.get(function.index()).and_then(Option::as_ref)
    }

    /// Return one native resume entry for one frame state.
    pub fn resume_entry(&self, frame_state: FrameStateId) -> Option<&ResumeEntry> {
        self.resume
            .get(frame_state.0 as usize)
            .and_then(Option::as_ref)
    }

    /// Return the program point for one native safepoint.
    pub fn safepoint_point(&self, safepoint: u32) -> Result<ProgramPoint, Error> {
        self.safepoint_points
            .get(safepoint as usize)
            .and_then(|point| *point)
            .ok_or(Error::StopPointMissing { safepoint })
    }

    /// Return whether this code can resume one continuation.
    pub fn can_resume(&self, continuation: &Continuation) -> Result<bool, Error> {
        let frame = continuation.frames.last().ok_or(Error::EmptyContinuation)?;

        Ok(self.resume_entry(frame.frame_state).is_some())
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
    pub fn set_resume(&mut self, frame_state: FrameStateId, entry: NativeResumeEntry) {
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
        program: &Program,
        context: &mut ProgramActivation<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome, Error> {
        let Some(entry) = self.entry(entry.function()) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            });
        };

        // build native ABI inputs
        let args = args.iter().map(NativeValue::from_value).collect::<Vec<_>>();
        let mut exit = NativeExit::default();
        let mut context = Self::native_context(program, context, &mut exit);
        let mut out = NativeValue::VOID;

        // enter generated native code
        let code = entry.call(&mut context, &args, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Resume one native continuation.
    pub fn resume(
        &self,
        program: &Program,
        context: &mut ProgramActivation<'_>,
        continuation: Continuation,
        value: Value,
    ) -> Result<Outcome, Error> {
        let frame = continuation.frames.last().ok_or(Error::EmptyContinuation)?;
        let Some(entry) = self.resume_entry(frame.frame_state) else {
            return Err(Error::ResumeEntryNotFound {
                frame_state: frame.frame_state,
            });
        };

        // build native continuation input
        let frames = Self::abi_frames(&continuation)?;
        let continuation = NativeContinuation {
            frames: frames.as_ptr(),
            frame_count: frames.len(),
        };
        let received = NativeValue::from_value(&value);
        let mut exit = NativeExit::default();
        let mut context = Self::native_context(program, context, &mut exit);
        let mut out = NativeValue::VOID;

        // enter generated native resume code
        let code = entry.call(&mut context, continuation, received, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Project one continuation into native ABI frame records.
    fn abi_frames(continuation: &Continuation) -> Result<Vec<NativeFrame>, Error> {
        continuation
            .frames
            .iter()
            .map(|frame| {
                let bytes =
                    continuation
                        .frame_bytes(frame)
                        .ok_or(Error::InvalidContinuationFrame {
                            frame_state: frame.frame_state,
                        })?;

                Ok(NativeFrame {
                    frame_state: frame.frame_state,
                    normal_state: frame.normal_state.into(),
                    unwind_state: frame.unwind_state.into(),
                    bytes: bytes.as_ptr(),
                    stack_offset: frame.stack_offset,
                    byte_len: bytes.len(),
                })
            })
            .collect()
    }

    /// Build one native ABI context.
    fn native_context(
        program: &Program,
        context: &mut ProgramActivation<'_>,
        exit: &mut NativeExit,
    ) -> NativeContext {
        NativeContext::new(
            context.state.as_ptr().cast(),
            context
                .storage
                .constant_space
                .as_native_constants(program.sections()),
            context.storage.shared_static.as_native_statics(),
            context.storage.local_static.as_native_statics(),
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

        // SAFETY: generated native code supplies valid exit continuation frame pointers
        let continuation =
            unsafe { exit.continuation.to_continuation() }.map_err(Error::InvalidContinuation)?;

        Ok(Outcome::Yielded {
            continuation,
            value,
        })
    }

    /// Return one deoptimized outcome from native continuation state.
    fn deoptimized_outcome(&self, exit: NativeExit) -> Result<Outcome, Error> {
        if exit.continuation.is_empty() {
            return Err(Error::DeoptimizedWithoutContinuation {
                safepoint: exit.safepoint,
            });
        }

        // SAFETY: generated native code supplies valid exit continuation frame pointers
        let continuation =
            unsafe { exit.continuation.to_continuation() }.map_err(Error::InvalidContinuation)?;

        Ok(Outcome::Deoptimized { continuation })
    }

    /// Return one stopped outcome from native continuation state.
    fn stopped_outcome(&self, exit: NativeExit) -> Result<Outcome, Error> {
        if exit.continuation.is_empty() {
            return Err(Error::StoppedWithoutContinuation {
                safepoint: exit.safepoint,
            });
        }

        // SAFETY: generated native code supplies valid exit continuation frame pointers
        let continuation =
            unsafe { exit.continuation.to_continuation() }.map_err(Error::InvalidContinuation)?;

        let point = self.safepoint_point(exit.safepoint)?;

        Ok(Outcome::Stopped {
            continuation,
            reason: StopReason::Instruction { point },
        })
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
            NativeExitKind::Stopped => self.stopped_outcome(exit),
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
