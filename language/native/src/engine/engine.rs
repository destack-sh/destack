use std::sync::Arc;

use destack_engine::{self as engine, Context, Entry, Outcome, Value};
use destack_heap as heap;

use crate::{Continuation, Image, Program};

/// Native backend execution error.
#[derive(Debug)]
pub enum Error {
    /// Native execution has not been linked yet.
    Unsupported {
        /// The requested native operation.
        operation: &'static str,
    },
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
}

/// Worker-local native execution backend.
pub struct Engine {
    /// The live engine identity.
    engine_id: destack_engine::EngineId,
    /// The executable program used by this engine.
    program: Arc<Program>,
}

impl Engine {
    /// Create one native engine over one loaded program.
    pub const fn new(engine_id: destack_engine::EngineId, program: Arc<Program>) -> Self {
        Self { engine_id, program }
    }

    /// Return the live engine identity.
    pub const fn engine_id(&self) -> destack_engine::EngineId {
        self.engine_id
    }

    /// Borrow the executable program.
    pub fn program(&self) -> &Program {
        self.program.as_ref()
    }

    /// Capture one native engine image.
    pub const fn image(&self) -> Image {
        Image::empty(self.engine_id)
    }
}

impl engine::Engine for Engine {
    type Continuation = Continuation;
    type Error = Error;
    type Image = Image;

    fn run(
        &mut self,
        _context: Context<'_>,
        entry: &Entry,
        _args: &[Value],
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error> {
        let Some(_entry) = self.program.entry_by_name(entry.name()) else {
            return Err(Error::EntryNotFound {
                name: entry.name().to_string(),
            });
        };

        Err(Error::Unsupported { operation: "run" })
    }

    fn resume(
        &mut self,
        _context: Context<'_>,
        _continuation: Self::Continuation,
        _value: Value,
    ) -> Result<Outcome<Self::Continuation, Value>, Self::Error> {
        Err(Error::Unsupported {
            operation: "resume",
        })
    }

    fn fork(&mut self, _heap: &mut heap::Heap) -> Result<Self, Self::Error> {
        Err(Error::Unsupported { operation: "fork" })
    }

    fn image(&mut self) -> Result<Self::Image, Self::Error> {
        Ok(Engine::image(self))
    }

    fn restore(&mut self, _heap: &mut heap::Heap, _image: &Self::Image) -> Result<(), Self::Error> {
        Err(Error::Unsupported {
            operation: "restore",
        })
    }
}

impl std::fmt::Debug for Engine {
    /// Format the engine without exposing runtime function pointers.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("engine_id", &self.engine_id)
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}
