use super::{PatternRelation, StaticRelation};

use crate::check::{CheckState, Condition, Origin, StaticOperand, TypeOperand, TypeRelation};

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
    /// Constrain two static operands.
    ///
    /// ```ts
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
            Self::Type { condition, .. }
            | Self::Static { condition, .. }
            | Self::Pattern { condition, .. } => condition.clone(),
        }
    }
}

impl CheckState<'_> {
    /// Store one solver constraint.
    pub(super) fn push_constraint(&mut self, constraint: Constraint) {
        self.inference.constraints.push(constraint);
    }
}
