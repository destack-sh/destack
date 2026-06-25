use destack_dir as dir;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::{Origin, Relation};
use crate::{CompilerError, CompilerResult};

/// Component-global id of one collected constraint.
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
    /// The checker role of this relation.
    pub(in crate::check) role: ConstraintRole,
}

/// Role one constraint plays in the checker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintRole {
    /// Relation checked at its origin without creating an implicit coercion.
    ///
    /// Examples:
    /// ```ds
    /// value satisfies Display
    /// value as int32
    /// function value<T: Display>(input: T) {}
    /// ```
    Check,

    /// Runtime expression assigned into a storage or pattern target.
    ///
    /// Examples:
    /// ```ds
    /// const value: int32 = 1
    /// target = source
    /// const [first] = values
    /// ```
    Value,

    /// Runtime argument assigned into a call or subscript parameter.
    ///
    /// Examples:
    /// ```ds
    /// print(value)
    /// list[index]
    /// ```
    Argument,

    /// Function body value assigned into a return or yield channel.
    ///
    /// Examples:
    /// ```ds
    /// return value
    /// yield value
    /// ```
    Output,

    /// Control-flow condition assigned to boolean.
    ///
    /// Examples:
    /// ```ds
    /// if (condition) {}
    /// while (condition) {}
    /// ```
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

/// Solved state of one constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ConstraintState {
    /// The constraint has not finished solving.
    Pending,
    /// The constraint relation holds.
    Holds,
    /// The constraint relation failed and reported its diagnostic.
    Fails,
    /// The constraint guard decided false.
    Skipped,
}

impl ConstraintState {
    /// Return whether the constraint is done.
    pub(in crate::check) fn is_done(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// Collected constraints with solver state.
#[derive(Debug)]
pub(in crate::check) struct ConstraintTable {
    /// The collected constraints keyed by absolute constraint id.
    constraints: IndexMap<ConstraintId, Constraint>,
    /// Constraint states keyed by absolute constraint id.
    states: IndexMap<ConstraintId, ConstraintState>,
}

impl ConstraintTable {
    /// Create an empty constraint table.
    pub(in crate::check) fn new() -> Self {
        Self {
            constraints: IndexMap::new(),
            states: IndexMap::new(),
        }
    }

    /// Insert one exact constraint id.
    pub(in crate::check) fn insert(&mut self, id: ConstraintId, constraint: Constraint) {
        self.constraints.insert(id, constraint);
        self.states.insert(id, ConstraintState::Pending);
    }

    /// Remove one exact constraint id.
    pub(in crate::check) fn remove(
        &mut self,
        id: ConstraintId,
    ) -> (Option<Constraint>, Option<ConstraintState>) {
        let constraint = self.constraints.swap_remove(&id);
        let state = self.states.swap_remove(&id);

        (constraint, state)
    }

    /// Return one constraint.
    pub(in crate::check) fn get(&self, id: ConstraintId) -> CompilerResult<&Constraint> {
        self.constraints
            .get(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} is not allocated"),
            })
    }

    /// Iterate over the collected constraints with their ids.
    pub(in crate::check) fn iter(&self) -> impl Iterator<Item = (ConstraintId, &Constraint)> {
        self.constraints
            .iter()
            .map(|(id, constraint)| (*id, constraint))
    }

    /// Return one constraint state.
    pub(in crate::check) fn state(&self, id: ConstraintId) -> CompilerResult<ConstraintState> {
        self.states
            .get(&id)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check constraint {id:?} has no solver state"),
            })
    }

    /// Set one constraint state.
    pub(in crate::check) fn set_state(&mut self, id: ConstraintId, state: ConstraintState) {
        self.states.insert(id, state);
    }

    /// Return whether one constraint finished solving.
    pub(in crate::check) fn is_complete(&self, id: ConstraintId) -> bool {
        self.state(id).is_ok_and(ConstraintState::is_done)
    }

    /// Return the number of collected constraints.
    pub(in crate::check) fn count(&self) -> usize {
        self.constraints.len()
    }
}
