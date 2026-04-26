use destack_engine as engine;

use super::Continuation;

/// Output from executing MIR code.
pub type Output = engine::Output<engine::Value>;

/// Outcome from a coroutine-capable execution entry.
pub type Outcome = engine::Outcome<Continuation, engine::Value>;
