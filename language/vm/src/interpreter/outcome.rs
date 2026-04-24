use destack_engine as engine;

use super::Continuation;

/// Output from executing MIR code.
pub type RunOutput = engine::RunOutput<engine::MaterializedValue>;

/// Outcome from a coroutine-capable execution entry.
pub type RunOutcome = engine::RunOutcome<Continuation, engine::MaterializedValue>;
