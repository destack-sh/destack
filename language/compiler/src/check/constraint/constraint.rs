use destack_dir as dir;

use crate::check::{CheckState, Condition, Origin, StaticOperand, TypeOperand, TypeRelation};

use super::{PatternRelation, StaticRelation};

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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        /// The coercion emitted by this relation.
        coercion: Option<dir::CastOrigin>,
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
    /// Return the source that produced this constraint.
    pub(in crate::check) fn origin(&self) -> Origin {
        match self {
            Self::Type { origin, .. }
            | Self::Static { origin, .. }
            | Self::Pattern { origin, .. } => *origin,
        }
    }

    /// Return the static condition guarding this constraint.
    pub(in crate::check) fn condition(&self) -> &Condition {
        match self {
            Self::Type { condition, .. }
            | Self::Static { condition, .. }
            | Self::Pattern { condition, .. } => condition,
        }
    }
}

impl CheckState<'_> {
    /// Store one solver constraint.
    pub(super) fn push_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        self.inference.push_constraint(constraint)
    }
}
