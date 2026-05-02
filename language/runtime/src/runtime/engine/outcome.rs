use destack_engine as engine;

/// Outcome from one engine run.
pub type Outcome<C> = engine::Outcome<C, engine::Value>;
