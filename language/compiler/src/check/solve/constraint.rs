use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Origin, Relation};
use crate::{CompilerError, CompilerResult};

/// Component-valid index of one collected constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ConstraintId(u32);

impl ConstraintId {
    /// Return the constraint id at one index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the constraint index.
    pub(in crate::check) fn index(self) -> usize {
        self.0 as usize
    }
}

/// One type relation to enforce.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct Constraint {
    /// The relation to enforce.
    pub(in crate::check) relation: Relation,
    /// The left operand, the source for directed relations.
    pub(in crate::check) left: dir::GlobalTypeId,
    /// The right operand, the target for directed relations.
    pub(in crate::check) right: dir::GlobalTypeId,
    /// The source that produced the constraint.
    pub(in crate::check) origin: Origin,
    /// The condition gating the constraint.
    pub(in crate::check) condition: Condition,
    /// The source context that failures report under.
    pub(in crate::check) cause: ConstraintCause,
}

/// Source context that produced one constraint.
/// Failures pick their diagnostic from the relation and this context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintCause {
    /// Plain value or annotation flow.
    General,
    /// Argument flowing into a parameter.
    Argument,
    /// Returned or completed value flowing into a result type.
    Return,
    /// Yielded value flowing into a generator channel.
    Yield,
    /// Runtime condition requiring a boolean.
    Condition,
}

/// Condition gating one constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Condition {
    /// The constraint always applies.
    Always,
    /// The constraint applies when every predicate reduces to a true literal.
    When(SmallVec<[dir::GlobalTypeId; 2]>),
}

impl Condition {
    /// Combine two conditions conjunctively.
    pub(in crate::check) fn and(self, other: Condition) -> Condition {
        match (self, other) {
            (Condition::Always, other) => other,
            (own, Condition::Always) => own,
            (Condition::When(mut left), Condition::When(right)) => {
                // keep predicates unique in source order
                for predicate in right {
                    if !left.contains(&predicate) {
                        left.push(predicate);
                    }
                }

                Condition::When(left)
            }
        }
    }
}

/// Collected constraints with completion tracking.
#[derive(Debug)]
pub(in crate::check) struct ConstraintTable {
    /// The collected constraints in allocation order.
    constraints: Vec<Constraint>,
    /// Whether each constraint finished solving.
    completed: Vec<bool>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self {
            constraints: Vec::new(),
            completed: Vec::new(),
        }
    }

    /// Allocate one constraint.
    pub(in crate::check) fn allocate(&mut self, constraint: Constraint) -> ConstraintId {
        let id = ConstraintId(self.constraints.len() as u32);
        self.constraints.push(constraint);
        self.completed.push(false);

        id
    }

    /// Return one constraint.
    pub(in crate::check) fn get(&self, id: ConstraintId) -> CompilerResult<&Constraint> {
        self.constraints
            .get(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected constraints with their ids.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| (ConstraintId::at(index), constraint))
    }

    /// Mark one constraint as finished.
    pub(in crate::check) fn complete(&mut self, id: ConstraintId) {
        self.completed[id.index()] = true;
    }

    /// Reopen one constraint during probe rollback.
    pub(in crate::check) fn reopen(&mut self, id: ConstraintId) {
        self.completed[id.index()] = false;
    }

    /// Return whether one constraint finished solving.
    pub(in crate::check) fn is_complete(&self, id: ConstraintId) -> bool {
        self.completed[id.index()]
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn count(&self) -> usize {
        self.constraints.len()
    }

    /// Truncate to a previous constraint count.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.constraints.truncate(count);
        self.completed.truncate(count);
    }
}
