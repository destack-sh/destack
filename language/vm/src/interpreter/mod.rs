mod continuation;
mod frame;
mod interpreter;
mod outcome;
mod stack;
mod step;

pub use crate::telemetry::Statistics;
pub use continuation::Continuation;
pub(crate) use continuation::{stabilize_materialized_value, visit_materialized_value_roots};
pub use destack_heap::GcStats;
pub use frame::Frame;
pub(crate) use frame::stabilize_value;
pub use interpreter::Interpreter;
pub use outcome::{RunOutcome, RunOutput};
pub(crate) use stack::StackAllocation;
pub(crate) use step::StepState;
