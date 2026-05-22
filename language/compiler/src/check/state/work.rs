use smallvec::SmallVec;

use super::{CheckFailure, VariableId};

/// Result of one solver step.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Step {
    /// Work is waiting for more solved inputs.
    Pending,
    /// Work applied and changed these variables.
    Applied(SmallVec<[VariableId; 4]>),
    /// Work found a check failure.
    Failed(CheckFailure),
}

impl Step {
    /// Return an applied step for one changed variable.
    pub(in crate::check) fn applied(variable: VariableId) -> Self {
        Self::Applied(smallvec::smallvec![variable])
    }
}
