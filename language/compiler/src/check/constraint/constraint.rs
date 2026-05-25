use smallvec::SmallVec;

use super::PatternRelation;

use crate::check::{
    ConstraintOrigin, Place, StaticRelation, StaticTerm, TypeRelation, TypeTerm, VariableId,
};

/// One check constraint.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Constraint {
    /// Define one type variable from one type term.
    ///
    /// ```ts
    /// value.name
    /// ```
    DefineType {
        /// The type variable being solved.
        result: VariableId,
        /// The type term assigned to it.
        term: TypeTerm,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Define one static variable from one static term.
    ///
    /// ```ts
    /// type Both = L | R;
    /// ```
    DefineStatic {
        /// The static variable being solved.
        result: VariableId,
        /// The static term assigned to it.
        term: StaticTerm,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Relate two type variables.
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
    },
    /// Relate two static variables.
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
    },
    /// Require one place to accept a write.
    ///
    /// ```ts
    /// const value = 1;
    /// value = 2;
    /// ```
    RequirePlaceWrite {
        /// The place being written.
        place: Place,
        /// The source that produced this constraint.
        origin: ConstraintOrigin,
    },
    /// Relate one pattern to a value type.
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
    },
}

impl Constraint {
    /// Return variables whose changes should wake this constraint.
    pub(in crate::check) fn wake_variables(&self) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::DefineType {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::DefineStatic {
                result,
                term,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*result];
                variables.extend(term.referenced_variables());

                variables
            }
            Self::RelateType {
                relation: _,
                left,
                right,
                origin: _,
            }
            | Self::RelateStatic {
                relation: _,
                left,
                right,
                origin: _,
            } => smallvec::smallvec![*left, *right],
            Self::RequirePlaceWrite { place, origin: _ } => place.referenced_variables(),
            Self::RelatePattern {
                relation,
                value,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*value];
                variables.extend(relation.referenced_variables());

                variables
            }
        }
    }
}
