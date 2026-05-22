use destack_dir as dir;

use crate::check::{
    CheckModuleState, Constraint, Predicate, Relation, Term, TypeRelation, TypeTerm, VariableId,
};

impl CheckModuleState {
    /// Walk one type expression and collect check work.
    pub(in crate::check) fn walk_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        let variable = self.bind_type_expression(id);

        // collect extends relations
        if let dir::TypeExpression::Extends { left, right } = type_expression {
            self.push_type_relation(TypeRelation::Extends, *left, *right);
            self.bind_type_literal(variable, dir::TypeLiteral::Boolean);
        }

        // collect implements relations
        if let dir::TypeExpression::Implements { left, right } = type_expression {
            self.push_type_relation(TypeRelation::Implements, *left, *right);
            self.bind_type_literal(variable, dir::TypeLiteral::Boolean);
        }

        dir::walk_type_expression(self, tree, id, type_expression);
    }

    /// Walk one type member and collect check work.
    pub(in crate::check) fn walk_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        type_member: &dir::TypeMember,
    ) {
        dir::walk_type_member(self, tree, id, type_member);
    }

    /// Walk one type mapped parameter and collect check work.
    pub(in crate::check) fn walk_type_mapped_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMappedParameter>,
        type_mapped_parameter: &dir::TypeMappedParameter,
    ) {
        dir::walk_type_mapped_parameter(self, tree, id, type_mapped_parameter);
    }

    /// Bind one source type expression to a type variable.
    fn bind_type_expression(&mut self, id: dir::LocalNodeId<dir::TypeExpression>) -> VariableId {
        let node = id.into_global(self.module());
        let variable = self.node_type_variable(node.clone().into_any());
        let term = Term::Type(TypeTerm::TypeExpression(node));
        let constraint = Constraint::bind(Predicate::Always, variable, term);

        self.push_constraint(constraint);

        variable
    }

    /// Bind one type expression to a literal type.
    fn bind_type_literal(
        &mut self,
        variable: VariableId,
        literal: dir::TypeLiteral,
    ) {
        let ty = dir::Type::from(literal);
        let term = Term::Type(TypeTerm::Type(ty));
        let constraint = Constraint::bind(Predicate::Always, variable, term);

        self.push_constraint(constraint);
    }

    /// Add one type relation constraint.
    fn push_type_relation(
        &mut self,
        relation: TypeRelation,
        left: dir::LocalNodeId<dir::TypeExpression>,
        right: dir::LocalNodeId<dir::TypeExpression>,
    ) {
        let left = left.into_global_any(self.module());
        let right = right.into_global_any(self.module());
        let left = self.node_type_variable(left);
        let right = self.node_type_variable(right);
        let relation = Relation::Type {
            relation,
            left,
            right,
        };
        let constraint = Constraint::relate(Predicate::Always, relation);

        self.push_constraint(constraint);
    }
}
