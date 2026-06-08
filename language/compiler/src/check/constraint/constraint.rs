use smallvec::SmallVec;

use super::{PatternRelation, StaticRelation};

use crate::check::{
    CheckState, Condition, Origin, StaticOperand, TypeOperand, TypeRelation, VariableId,
};

/// Component-valid id for one check constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::check) struct ConstraintId {
    /// The constraint index inside the checked component.
    pub(in crate::check) index: u32,
}

impl ConstraintId {
    /// Create one constraint id.
    pub(in crate::check) fn new(index: u32) -> Self {
        Self { index }
    }

    /// Return the constraint index.
    pub(in crate::check) fn index(self) -> usize {
        self.index as usize
    }
}

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Constrain two type operands.
    ///
    /// ```ds
    /// const value: int32 = 1;
    /// ```
    Type {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: TypeOperand,
        /// The right type.
        right: TypeOperand,
        /// The source that produced this constraint.
        origin: Origin,
        /// The static condition under which this constraint exists.
        condition: Condition,
    },
    /// Constrain two static operands.
    ///
    /// ```ds
    /// const value: [int32; _] = [1, 2];
    /// ```
    Static {
        /// The required relation.
        relation: StaticRelation,
        /// The left static value.
        left: StaticOperand,
        /// The right static value.
        right: StaticOperand,
        /// The source that produced this constraint.
        origin: Origin,
        /// The static condition under which this constraint exists.
        condition: Condition,
    },
    /// Constrain one pattern against a value type.
    ///
    /// ```ds
    /// const Some(value) = result;
    /// ```
    Pattern {
        /// The pattern relation being checked.
        relation: PatternRelation,
        /// The value type being matched.
        value: TypeOperand,
        /// The source that produced this constraint.
        origin: Origin,
        /// The static condition under which this constraint exists.
        condition: Condition,
    },
}

impl Constraint {
    /// Return the static condition guarding this constraint.
    pub(in crate::check) fn condition(&self) -> Condition {
        match self {
            Self::Type { condition, .. }
            | Self::Static { condition, .. }
            | Self::Pattern { condition, .. } => condition.clone(),
        }
    }

    /// Return variables referenced by this constraint.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.extend(self.condition().referenced_variables(state));

        match self {
            Self::Type { left, right, .. } => {
                variables.extend(left.referenced_variables(state));
                variables.extend(right.referenced_variables(state));
            }
            Self::Static { left, right, .. } => {
                variables.extend(left.referenced_variables(state));
                variables.extend(right.referenced_variables(state));
            }
            Self::Pattern {
                relation, value, ..
            } => {
                variables.extend(value.referenced_variables(state));
                variables.extend(relation.referenced_variables(state));
            }
        }

        variables
    }
}

impl CheckState<'_> {
    /// Store one solver constraint.
    pub(super) fn push_constraint(&mut self, constraint: Constraint) {
        let variables = constraint.referenced_variables(self);

        self.inference.push_constraint(constraint, variables);
    }
}
