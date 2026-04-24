use destack_engine as engine;

/// Output from one engine run.
pub type RunOutput = engine::RunOutput<engine::MaterializedValue>;

/// Outcome from one engine run.
pub type RunOutcome<C> = engine::RunOutcome<C, engine::MaterializedValue>;
