use destack_engine as engine;

/// Output from one engine run.
pub type Output = engine::Output<engine::Value>;

/// Outcome from one engine run.
pub type Outcome<C> = engine::Outcome<C, engine::Value>;
