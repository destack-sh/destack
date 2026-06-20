use destack_program as program;

/// Outcome from one machine run.
pub type Outcome<C> = program::Outcome<C, program::Value>;
