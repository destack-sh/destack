use destack_dir as dir;

use crate::check::{
    CheckState, Constraint, ConstraintOrigin, StaticOperand, TypeLiteralTerm, TypeOperand,
    TypeTerm, VariableId,
};

/// A relation between two type variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum TypeRelation {
    /// Types must be equal.
    Equal,
    /// Source must be assignable to target.
    Assignable,
    /// Source must be explicitly castable to target.
    Castable,
    /// Value must satisfy a constraint.
    Satisfies,
    /// Subtype must extend supertype.
    Extends,
    /// Implementor must implement contract.
    Implements,
}

/// A relation between two static variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    Equal,
}

impl CheckState<'_> {
    /// Constrain two type variables.
    pub(in crate::check) fn constrain_type(
        &mut self,
        origin: ConstraintOrigin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
    ) {
        let constraint = Constraint::Type {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition: self.active_static_condition(origin.module()),
        };

        self.add_constraint(constraint);
    }

    /// Constrain two static variables.
    #[allow(dead_code)]
    pub(in crate::check) fn constrain_static(
        &mut self,
        origin: ConstraintOrigin,
        relation: StaticRelation,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) {
        let constraint = Constraint::Static {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition: self.active_static_condition(origin.module()),
        };

        self.add_constraint(constraint);
    }

    /// Constrain one expression condition to boolean.
    pub(in crate::check) fn constrain_condition(
        &mut self,
        source: dir::LocalNodeIdAny,
        condition: VariableId,
    ) {
        let origin = ConstraintOrigin::Node(source.into_global(condition.module));
        let expected = self
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::boolean()));

        self.constrain_type(origin, TypeRelation::Assignable, condition, expected);
    }
}
