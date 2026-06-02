use std::fmt;
use std::sync::Arc;

use destack_engine::{
    Engine as EngineTrait, EngineCall, EngineId, EngineMemory, EntryPoint, Outcome, StaticSpace,
    Value,
};
use destack_heap as heap;

use crate::{
    Continuation, Error, Image, NativeContext, NativeExit, NativeStatus, NativeTrap, NativeValue,
    Program,
};

/// Worker-local native execution backend.
pub struct Engine {
    /// The live engine identity.
    engine_id: EngineId,
    /// The loaded native program.
    program: Arc<Program>,
}

impl Engine {
    /// Create one native engine over one loaded program.
    pub const fn new(engine_id: EngineId, program: Arc<Program>) -> Self {
        Self { engine_id, program }
    }

    /// Return the live engine identity.
    pub const fn engine_id(&self) -> EngineId {
        self.engine_id
    }

    /// Borrow the loaded native program.
    pub fn program(&self) -> &Program {
        self.program.as_ref()
    }

    /// Resolve one runtime entry name into an engine entry handle.
    pub fn entry_by_name(&self, name: &str) -> Result<EntryPoint, Error> {
        let Some(entry) = self.program.object().entry_by_name(name) else {
            return Err(Error::EntryNotFound {
                name: name.to_string(),
            });
        };

        Ok(EntryPoint::new(entry.id.0))
    }

    /// Capture one native engine image.
    pub const fn image(&self) -> Image {
        Image::empty(self.engine_id)
    }
}

impl EngineTrait for Engine {
    type Continuation = Continuation;
    type Error = Error;
    type Image = Image;

    fn initialize(&mut self, _context: EngineMemory<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn run(
        &mut self,
        context: EngineCall<'_>,
        entry: EntryPoint,
        args: &[Value],
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error> {
        let Some(entry) = self.program.entry(crate::EntryId(entry.index())) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            });
        };
        let args = args
            .iter()
            .map(NativeValue::from_engine)
            .collect::<Vec<_>>();
        let mut exit = NativeExit::default();
        let mut context = NativeContext::new(context.host.as_ptr(), &mut exit);
        let mut out = NativeValue::VOID;

        let status = entry.call(&mut context, &args, &mut out);
        let status = NativeStatus::try_from(status).map_err(Error::InvalidStatus)?;
        match status {
            NativeStatus::Completed => {
                let value = out.to_engine().map_err(Error::Value)?;

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
                let payload = exit.payload.to_engine().map_err(Error::Value)?;

                Err(Error::Panicked { payload })
            }
        }
    }

    fn resume(
        &mut self,
        _context: EngineCall<'_>,
        _continuation: Self::Continuation,
        _value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error> {
        Err(Error::ContinuationUnavailable)
    }

    fn fork(&self, _context: EngineMemory<'_>) -> Result<Self, Self::Error> {
        Ok(Self::new(self.engine_id, self.program.clone()))
    }

    fn image(&self, _context: EngineMemory<'_>) -> Result<Self::Image, Self::Error> {
        Ok(Engine::image(self))
    }

    fn restore(
        &mut self,
        _context: EngineMemory<'_>,
        image: &Self::Image,
    ) -> Result<(), Self::Error> {
        if image.engine_id == self.engine_id {
            Ok(())
        } else {
            Err(Error::ImageEngineMismatch {
                engine_id: self.engine_id,
                image_engine_id: image.engine_id,
            })
        }
    }

    fn visit_root_slots(
        &mut self,
        _statics: &mut StaticSpace,
        _visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Err(Error::RootMapUnavailable)
    }

    fn visit_continuation_root_slots(
        &mut self,
        _continuation: &mut Self::Continuation,
        _visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> Result<(), Self::Error> {
        Err(Error::RootMapUnavailable)
    }
}

impl fmt::Debug for Engine {
    /// Format the engine without exposing runtime function pointers.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("engine_id", &self.engine_id)
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}
