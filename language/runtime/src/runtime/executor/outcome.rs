use destack_program as program;

/// Outcome from one executor run.
pub type Outcome<C> = program::Outcome<C, program::Value>;
