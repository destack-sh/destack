use smallvec::SmallVec;

use super::PatternRelation;

use crate::check::{ConstraintOrigin, StaticCondition, StaticRelation, TypeRelation, VariableId};

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Constrain two type variables.
    ///
    /// ```ts
    /// const value: int32 = 1;
    /// ```
    RelateType {
        /// The required relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
        /// The static condition under which this constraint exists.
        condition: StaticCondition,
    },
    /// Constrain two static variables.
    ///
    /// ```ts
    /// const size: 4 = value.length;
    /// ```
    RelateStatic {
        /// The required relation.
        relation: StaticRelation,
        /// The left static value.
        left: VariableId,
        /// The right static value.
        right: VariableId,
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
    RelatePattern {
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
    /// Return variables whose changes should wake this constraint.
    pub(in crate::check) fn wake_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::RelateType {
                relation: _,
                left,
                right,
                origin: _,
                condition,
            }
            | Self::RelateStatic {
                relation: _,
                left,
                right,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*left, *right];
                variables.extend(condition.referenced_variables());

                variables
            }
            Self::RelatePattern {
                relation,
                value,
                origin: _,
                condition,
            } => {
                let mut variables = smallvec::smallvec![*value];
                variables.extend(condition.referenced_variables());
                variables.extend(relation.referenced_variables());

                variables
            }
        }
    }
}
