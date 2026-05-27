use smallvec::SmallVec;

use super::PatternRelation;

use crate::check::{
    CheckState, ConstraintOrigin, StaticCondition, StaticOperand, StaticRelation, TypeOperand,
    TypeRelation, VariableId,
};

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
        origin: ConstraintOrigin,
        /// The static condition under which this constraint exists.
        condition: StaticCondition,
    },
    /// Constrain two static operands.
    ///
    /// ```ts
    /// const size: 4 = value.length;
    /// ```
    #[allow(dead_code)]
    Static {
        /// The required relation.
        relation: StaticRelation,
        /// The left static value.
        left: StaticOperand,
        /// The right static value.
        right: StaticOperand,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
        /// The static condition under which this constraint exists.
        condition: StaticCondition,
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
        value: VariableId,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
        /// The static condition under which this constraint exists.
        condition: StaticCondition,
    },
}

impl Constraint {
    /// Return the source that produced this constraint.
    pub(in crate::check) fn origin(&self) -> ConstraintOrigin {
        match self {
            Self::Type { origin, .. }
            | Self::Static { origin, .. }
            | Self::Pattern { origin, .. } => *origin,
        }
    }

    /// Return the static condition guarding this constraint.
    pub(in crate::check) fn condition(&self) -> StaticCondition {
        match self {
            Self::Type { condition, .. }
            | Self::Static { condition, .. }
            | Self::Pattern { condition, .. } => condition.clone(),
        }
    }

    /// Return variables watched by this constraint.
    pub(in crate::check) fn watched_variables(
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
            Self::Static {
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
                let mut variables = smallvec::smallvec![*value];
                variables.extend(condition.referenced_variables(state));
                variables.extend(relation.referenced_variables(&state.terms));

                variables
            }
        }
    }
}
