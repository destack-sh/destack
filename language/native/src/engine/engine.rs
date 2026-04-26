use std::sync::Arc;

use crate::{Image, Program};

/// Native backend execution error.
#[derive(Debug)]
pub enum Error {
    /// The requested entry is not present in the native program.
    EntryNotFound {
        /// The missing entry name.
        name: String,
    },
    /// Native code returned a trap status.
    Trapped,
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
