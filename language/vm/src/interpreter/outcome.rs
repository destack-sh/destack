use destack_engine::{Outcome as EngineOutcome, Value};

use super::Continuation;

/// Outcome from a coroutine-capable execution entry.
pub type Outcome = EngineOutcome<Continuation, Value>;
