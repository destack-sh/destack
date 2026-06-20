use std::fmt;
use std::sync::Arc;

use destack_heap as heap;
use destack_program::{EntryPoint, RuntimeCall, RuntimeMemory, StaticSpace, Value};

use crate::{
    Continuation, Error, Image, NativeContext, NativeContinuation, NativeExit, NativeExitKind,
    NativeTrap, NativeValue, Outcome, Program,
};

/// Worker-local native machine.
pub struct Machine {
    /// The native program.
    program: Arc<Program>,
}

impl Machine {
    /// Create one native machine over one native program.
    pub const fn new(program: Arc<Program>) -> Self {
        Self { program }
    }

    /// Borrow the native program.
    pub fn program(&self) -> &Program {
        self.program.as_ref()
    }

    /// Resolve one runtime entry name into an execution entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<EntryPoint, Error> {
        let Some(function) = self.program.program().function_id_by_name(name) else {
            return Err(Error::EntryNotFound {
                name: name.to_string(),
            });
        };

        Ok(EntryPoint::from(function))
    }

    /// Capture one native machine image.
    pub const fn image(&self) -> Image {
        Image::empty()
    }

    /// Initialize native machine memory.
    pub fn initialize(&mut self, context: RuntimeMemory<'_>) -> Result<(), Error> {
        self.program
            .program()
            .initialize_statics(context.local_static, context.shared_static);

        Ok(())
    }

    /// Run one native entrypoint.
    pub fn run(
        &mut self,
        context: &mut RuntimeCall<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome, Error> {
        let Some(entry) = self.program.entry_point(entry) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            });
        };
        let args = args.iter().map(NativeValue::from_value).collect::<Vec<_>>();
        let mut exit = NativeExit::default();
        let mut context = NativeContext::new(
            context.state.as_ptr().cast(),
            context.memory.constant_space.as_native_constants(),
            context.memory.shared_static.as_native_statics(),
            context.memory.local_static.as_native_statics(),
            &mut exit,
        );
        let mut out = NativeValue::VOID;

        let code = entry.call(&mut context, &args, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Resume one native continuation.
    pub fn resume(
        &mut self,
        context: &mut RuntimeCall<'_>,
        continuation: Continuation,
        value: Value,
    ) -> Result<Outcome, Error> {
        let frame = continuation.frames.last().ok_or(Error::EmptyContinuation)?;
        let Some(entry) = self.program.resume(frame.frame_state) else {
            return Err(Error::ResumeEntryNotFound {
                frame_state: frame.frame_state,
            });
        };
        let frames = continuation.abi_frames();
        let continuation = NativeContinuation {
            frames: frames.as_ptr(),
            frame_count: frames.len(),
        };
        let received = NativeValue::from_value(&value);
        let mut exit = NativeExit::default();
        let mut context = NativeContext::new(
            context.state.as_ptr().cast(),
            context.memory.constant_space.as_native_constants(),
            context.memory.shared_static.as_native_statics(),
            context.memory.local_static.as_native_statics(),
            &mut exit,
        );
        let mut out = NativeValue::VOID;

        let code = entry.call(&mut context, continuation, received, &mut out);

        self.outcome_from_exit(code, out, exit)
    }

    /// Fork this native machine.
    pub fn fork(&self, _context: RuntimeMemory<'_>) -> Result<Self, Error> {
        Ok(Self::new(self.program.clone()))
    }

    /// Capture one native machine image.
    pub fn image_with_memory(&self, _context: RuntimeMemory<'_>) -> Result<Image, Error> {
        Ok(Self::image(self))
    }

    /// Restore one native machine image.
    pub fn restore(&mut self, _context: RuntimeMemory<'_>, _image: &Image) -> Result<(), Error> {
        Ok(())
    }

    /// Visit mutable heap root slots from active native state.
    pub fn visit_root_slots(
        &mut self,
        _local_static: &mut StaticSpace,
        _visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Error> {
        Ok(())
    }

    /// Visit mutable heap root slots from one native continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        continuation: &mut Continuation,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Error> {
        continuation.visit_root_slots(self.program.program(), visit)
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
        let materialization =
            unsafe { exit.materialization.to_program() }.map_err(Error::InvalidMaterialization)?;

        Ok(Outcome::Deoptimized { materialization })
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

impl fmt::Debug for Machine {
    /// Format the machine without exposing runtime function pointers.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Machine")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}
