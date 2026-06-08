use crate::check::{ConstraintId, VariableId};

/// One scheduled solver task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum SolveTask {
    /// Recheck one constraint.
    Constraint(ConstraintId),
    /// Try to solve one variable from its bounds.
    Variable(VariableId),
}
