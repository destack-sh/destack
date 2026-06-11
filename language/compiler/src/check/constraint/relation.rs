use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Condition, Constraint, ConstraintId, Origin, StaticOperand, TypeLiteralTerm,
    TypeOperand, TypeTerm, VariableId,
};

/// Relation between two type operands.
///
/// Examples:
/// ```ds
/// const value: int32 = 1
/// value as string
/// type Box<T extends Item> = T
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum TypeRelation {
    /// Types must be equal.
    ///
    /// Examples:
    /// ```ds
    /// const value: int32 = 1
    /// ```
    Equal,
    /// Source must be assignable to target.
    ///
    /// Examples:
    /// ```ds
    /// const value: string | null = name
    /// ```
    Assignable,
    /// Source must be explicitly castable to target.
    ///
    /// Examples:
    /// ```ds
    /// value as string
    /// ```
    Castable,
    /// Value must satisfy a constraint.
    ///
    /// Examples:
    /// ```ds
    /// const value = input satisfies Named
    /// ```
    Satisfies,
    /// Subtype must extend supertype.
    ///
    /// Examples:
    /// ```ds
    /// type Box<T extends Item> = T
    /// ```
    Extends,
    /// Implementor must implement contract.
    ///
    /// Examples:
    /// ```ds
    /// impl Iterator for Items
    /// ```
    Implements,
}

/// Relation between two static operands.
///
/// Examples:
/// ```ds
/// [int32; 4]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum StaticRelation {
    /// Static values must be equal.
    ///
    /// Examples:
    /// ```ds
    /// [int32; 4]
    /// ```
    Equal,
    /// Source must be assignable to target.
    ///
    /// Examples:
    /// ```ds
    /// const length: usize = 4
    /// ```
    Assignable,
}

impl CheckState<'_> {
    /// Equate one type variable to one type term.
    pub(in crate::check) fn equate_type(
        &mut self,
        variable: VariableId,
        term: TypeTerm,
        condition: Condition,
    ) {
        let origin = self.variable(variable).source;
        let term = self.inference.push_term(term);

        self.constrain_type(origin, TypeRelation::Equal, variable, term, condition);
    }

    /// Equate two static operands.
    pub(in crate::check) fn equate_static(
        &mut self,
        origin: Origin,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
        condition: Condition,
    ) {
        self.constrain_static(origin, StaticRelation::Equal, left, right, condition);
    }

    /// Relate two static operands.
    pub(in crate::check) fn constrain_static(
        &mut self,
        origin: Origin,
        relation: StaticRelation,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
        condition: Condition,
    ) -> ConstraintId {
        let constraint = Constraint::Static {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition,
        };

        self.push_constraint(constraint)
    }

    /// Relate two type operands.
    pub(in crate::check) fn constrain_type(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
        condition: Condition,
    ) -> ConstraintId {
        let constraint = Constraint::Type {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition,
            coercion: None,
        };

        self.push_constraint(constraint)
    }

    /// Relate two type operands and emit one checked coercion.
    pub(in crate::check) fn constrain_coercion(
        &mut self,
        origin: Origin,
        relation: TypeRelation,
        left: impl Into<TypeOperand>,
        right: impl Into<TypeOperand>,
        condition: Condition,
        coercion: dir::CastOrigin,
    ) -> ConstraintId {
        let constraint = Constraint::Type {
            relation,
            left: left.into(),
            right: right.into(),
            origin,
            condition,
            coercion: Some(coercion),
        };

        self.push_constraint(constraint)
    }

    /// Check one expression condition to be boolean.
    pub(in crate::check) fn constrain_condition(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        condition: TypeOperand,
        static_condition: Condition,
    ) {
        let origin = Origin::Node(source.into_global(module));
        let expected = self
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::boolean()));

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            condition,
            expected,
            static_condition,
        );
    }
}
