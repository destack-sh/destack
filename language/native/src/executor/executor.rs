use std::fmt;
use std::sync::Arc;

use destack_heap as heap;
use destack_program::{EntryPoint, ExecutionCall, ExecutionMemory, Outcome, StaticSpace, Value};

use crate::{
    Continuation, Error, Image, NativeContext, NativeExit, NativeStatus, NativeTrap, NativeValue,
    Program,
};

/// Worker-local native executor.
pub struct Executor {
    /// The loaded native program.
    program: Arc<Program>,
}

impl Executor {
    /// Create one native executor over one loaded program.
    pub const fn new(program: Arc<Program>) -> Self {
        Self { program }
    }

    /// Borrow the loaded native program.
    pub fn program(&self) -> &Program {
        self.program.as_ref()
    }

    /// Resolve one runtime entry name into an execution entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<EntryPoint, Error> {
        let Some(entry) = self.program.object().entry_by_name(name) else {
            return Err(Error::EntryNotFound {
                name: name.to_string(),
            });
        };

        Ok(EntryPoint::new(entry.id.0))
    }

    /// Capture one native executor image.
    pub const fn image(&self) -> Image {
        Image::empty()
    }

    /// Initialize native executor memory.
    pub fn initialize(&mut self, context: ExecutionMemory<'_>) -> Result<(), Error> {
        // initialize shared static once per runtime
        if context.shared_static.is_empty() {
            context
                .shared_static
                .clone_from(self.program.shared_statics());
        }

        // initialize local static once per worker
        context
            .local_static
            .clone_from(self.program.local_statics());

        Ok(())
    }

    /// Run one native entrypoint.
    pub fn run(
        &mut self,
        context: ExecutionCall<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome<Continuation, Value>, Error> {
        let Some(entry) = self.program.entry(crate::EntryId(entry.index())) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            });
        };
        let args = args.iter().map(NativeValue::from_value).collect::<Vec<_>>();
        let mut exit = NativeExit::default();
        let mut context = NativeContext::new(context.host.as_ptr(), &mut exit);
        let mut out = NativeValue::VOID;

        let status = entry.call(&mut context, &args, &mut out);
        let status = NativeStatus::try_from(status).map_err(Error::InvalidStatus)?;
        match status {
            NativeStatus::Completed => {
                let value = out.to_value().map_err(Error::Value)?;

                Ok(Outcome::Completed { value })
            }
            NativeStatus::Yielded => Err(Error::YieldedWithoutContinuation {
                safepoint: exit.safepoint,
            }),
            NativeStatus::Trapped => {
                let trap = NativeTrap::try_from(exit.trap).map_err(Error::InvalidTrap)?;

                Err(Error::Trapped { trap })
            }
            NativeStatus::Deoptimized => Err(Error::DeoptimizedWithoutMaterialization {
                safepoint: exit.safepoint,
            }),
            NativeStatus::Panicked => {
                let payload = exit.payload.to_value().map_err(Error::Value)?;

                Err(Error::Panicked { payload })
            }
        }
    }

    /// Resume one native continuation.
    pub fn resume(
        &mut self,
        _context: ExecutionCall<'_>,
        _continuation: Continuation,
        _value: Value,
    ) -> Result<Outcome<Continuation, Value>, Error> {
        Err(Error::ContinuationUnavailable)
    }

    /// Fork this native executor.
    pub fn fork(&self, _context: ExecutionMemory<'_>) -> Result<Self, Error> {
        Ok(Self::new(self.program.clone()))
    }

    /// Capture one native executor image.
    pub fn image_with_memory(&self, _context: ExecutionMemory<'_>) -> Result<Image, Error> {
        Ok(Executor::image(self))
    }

    /// Restore one native executor image.
    pub fn restore(&mut self, _context: ExecutionMemory<'_>, _image: &Image) -> Result<(), Error> {
        Ok(())
    }

    /// Visit mutable heap root slots from active native state.
    pub fn visit_root_slots(
        &mut self,
        _local_static: &mut StaticSpace,
        _visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Error> {
        Err(Error::RootMapUnavailable)
    }

    /// Visit mutable heap root slots from one native continuation.
    pub fn visit_continuation_root_slots(
        &mut self,
        _continuation: &mut Continuation,
        _visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Error> {
        Err(Error::RootMapUnavailable)
    }
}

impl fmt::Debug for Executor {
    /// Format the executor without exposing runtime function pointers.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Executor")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}
