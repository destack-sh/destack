use std::fmt;
use std::sync::Arc;

use destack_engine::{
    CallContext, Engine as EngineTrait, EngineId, EntryPoint, MemoryContext, Outcome, StaticSpace,
    Value,
};
use destack_heap as heap;

use crate::{
    Continuation, Image, NativeContext, NativeExit, NativeStatus, NativeStatusError, NativeTrap,
    NativeTrapError, NativeValue, NativeValueError, Program,
};

/// Native backend execution error.
#[derive(Debug)]
pub enum Error {
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
    /// Native execution yielded without a materialized continuation.
    YieldedWithoutContinuation {
        /// The safepoint that yielded.
        safepoint: u32,
    },
    /// Native execution reported a trap.
    Trapped {
        /// The reported trap.
        trap: NativeTrap,
    },
    /// Native execution requested deoptimization without materialization.
    DeoptimizedWithoutMaterialization {
        /// The safepoint that requested deoptimization.
        safepoint: u32,
    },
    /// Native execution reported a language panic.
    Panicked {
        /// The panic payload.
        payload: Value,
    },
    /// A native exit status code could not be decoded.
    InvalidStatus(NativeStatusError),
    /// A native trap code could not be decoded.
    InvalidTrap(NativeTrapError),
    /// A native ABI value could not be decoded.
    Value(NativeValueError),
    /// Native continuation state is not resumable by this engine.
    ContinuationUnavailable,
    /// A native image belongs to another engine.
    ImageEngineMismatch {
        /// The current engine id.
        engine_id: EngineId,
        /// The captured image engine id.
        image_engine_id: EngineId,
    },
    /// Native root metadata is not available.
    RootMapUnavailable,
}

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

    fn initialize(&mut self, _context: MemoryContext<'_>) -> Result<(), Self::Error> {
        Ok(())
    }

    fn run(
        &mut self,
        context: CallContext<'_>,
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
        let mut context = NativeContext::new(context.runtime.as_ptr(), &mut exit);
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
        _context: CallContext<'_>,
        _continuation: Self::Continuation,
        _value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error> {
        Err(Error::ContinuationUnavailable)
    }

    fn fork(&self, _context: MemoryContext<'_>) -> Result<Self, Self::Error> {
        Ok(Self::new(self.engine_id, self.program.clone()))
    }

    fn image(&self, _context: MemoryContext<'_>) -> Result<Self::Image, Self::Error> {
        Ok(Engine::image(self))
    }

    fn restore(
        &mut self,
        _context: MemoryContext<'_>,
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

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryNotFound { name } => write!(formatter, "native entry not found: {name}"),
            Self::YieldedWithoutContinuation { safepoint } => {
                write!(
                    formatter,
                    "native execution yielded at safepoint {safepoint} without a continuation"
                )
            }
            Self::Trapped { trap } => {
                write!(formatter, "native execution trapped: {trap:?}")
            }
            Self::DeoptimizedWithoutMaterialization { safepoint } => {
                write!(
                    formatter,
                    "native execution deoptimized at safepoint {safepoint} without materialization"
                )
            }
            Self::Panicked { payload } => {
                write!(formatter, "native execution panicked with {payload:?}")
            }
            Self::InvalidStatus(error) => write!(formatter, "native status error: {error}"),
            Self::InvalidTrap(error) => write!(formatter, "native trap error: {error}"),
            Self::Value(error) => write!(formatter, "native value error: {error}"),
            Self::ContinuationUnavailable => {
                write!(formatter, "native continuation is not resumable")
            }
            Self::ImageEngineMismatch {
                engine_id,
                image_engine_id,
            } => write!(
                formatter,
                "native image belongs to engine {}, not {}",
                image_engine_id.get(),
                engine_id.get()
            ),
            Self::RootMapUnavailable => write!(formatter, "native root map is not available"),
        }
    }
}

impl std::error::Error for Error {}

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
