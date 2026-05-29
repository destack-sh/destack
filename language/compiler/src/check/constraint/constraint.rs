use smallvec::SmallVec;

use super::PatternRelation;

use crate::check::{CheckState, Condition, Origin, TypeOperand, TypeRelation, VariableId};

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Constrain two type operands.
    ///
    /// ```ts
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
    /// Constrain one pattern against a value type.
    ///
    /// ```ts
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
            Self::Type { condition, .. } | Self::Pattern { condition, .. } => condition.clone(),
        }
    }

    /// Return variables watched by this constraint.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Type {
                relation: _,
                left,
                right,
                origin: _,
                condition,
            } => {
                let mut variables = left.referenced_variables(state);
                variables.extend(right.referenced_variables(state));
                variables.extend(condition.referenced_variables(state));

                variables
            }
            Self::Pattern {
                relation,
                value,
                origin: _,
                condition,
            } => {
                let mut variables = value.referenced_variables(state);
                variables.extend(condition.referenced_variables(state));
                variables.extend(relation.referenced_variables(state));

                variables
            }
        }
    }
}
