pub(crate) mod machine;
mod state;

pub use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput, ExecutionYield};
pub use crate::telemetry::Statistics;
pub use destack_heap::GcStats;
pub use state::{Frame, Interpreter};
pub(crate) use state::{StackAllocation, StepState};
