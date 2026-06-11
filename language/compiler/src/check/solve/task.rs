use destack_dir as dir;

use crate::check::{ConstraintId, InferenceProbe, VariableId};

/// One dependency that can wake solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Dependency {
    /// A solver constraint was completed.
    Constraint(ConstraintId),
    /// A solver variable received new bounds.
    Bounds(VariableId),
    /// A solver variable was solved.
    Variable(VariableId),
    /// A check decision was selected.
    Decision(DecisionKey),
}

impl Dependency {
    /// Return whether this dependency names state outside one probe.
    pub(in crate::check) fn precedes_probe(self, probe: InferenceProbe) -> bool {
        match self {
            Self::Constraint(constraint) => probe.precedes_constraint(constraint),
            Self::Bounds(variable) => probe.precedes_variable(variable),
            Self::Variable(variable) => probe.precedes_variable(variable),
            Self::Decision(_) => true,
        }
    }

    /// Return the solved variable dependency when this dependency names one.
    pub(in crate::check) fn solution_variable(self) -> Option<VariableId> {
        match self {
            Self::Constraint(_) => None,
            Self::Bounds(_) => None,
            Self::Variable(variable) => Some(variable),
            Self::Decision(_) => None,
        }
    }
}

/// One check decision that can wake solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum DecisionKey {
    /// Runtime callable decision.
    Callable(dir::GlobalNodeIdAny),
    /// Runtime call decision.
    Call(dir::GlobalNodeIdAny),
    /// Runtime construct decision.
    Construct(dir::GlobalNodeIdAny),
    /// Runtime operator decision.
    Operator(dir::GlobalNodeIdAny),
    /// Runtime identity decision.
    Identity(dir::GlobalNodeIdAny),
    /// Layout decision.
    Layout(dir::GlobalNodeIdAny),
    /// Runtime member decision.
    Member(dir::GlobalNodeIdAny),
    /// Pattern decision.
    Pattern(dir::GlobalNodeIdAny),
    /// Contextual receiver decision.
    Receiver(dir::GlobalNodeIdAny),
}

/// One scheduled solver task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum Task {
    /// Recheck one constraint.
    Constraint(ConstraintId),
    /// Try to solve one variable from its bounds.
    Variable(VariableId),
    /// Try to solve one variable from its generic argument default.
    ArgumentDefault(VariableId),
}

impl Task {
    /// Return the scheduling priority for one task.
    pub(in crate::check) fn priority(self) -> u8 {
        match self {
            Self::Constraint(_) => 0,
            Self::Variable(_) => 1,
            Self::ArgumentDefault(_) => 2,
        }
    }
}
