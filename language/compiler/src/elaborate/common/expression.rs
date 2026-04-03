use destack_dir as dir;
use dir::{
    BinaryOperator, Expression, LocalNodeId, LocalTypeId, NodeType, Type, TypeBinaryOperator,
    TypeLiteral,
};

use super::ElaborateState;
use crate::Compiler;

impl Compiler {
    /// Insert a type expression for one local type id.
    pub(crate) fn insert_type_expression_for_type_id(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // choose the compact type expression form
        let expression = match state.types.get_type(type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type { value: type_id },
        };

        // insert the type expression node
        let expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let expression_id = state.tree.insert_as_owner(expression_id, expression);

        // annotate with Type::Value(type_id)
        let type_value = Type::Value { value: type_id };
        let type_value_id = state.types.insert_type_from(type_value, expression_id);
        state.types.set_inferred_type(
            expression_id.into_global_any(state.ctx.module_id),
            type_value_id,
        );

        expression_id
    }

    /// Insert a type literal expression with inferred type metadata.
    pub(crate) fn insert_type_literal_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value: TypeLiteral,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // insert one type literal expression
        let literal_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let literal_id = state.tree.insert_as_owner(
            literal_id,
            Expression::TypeLiteral {
                value: value.clone(),
            },
        );

        // annotate with its literal type
        let literal_type = Type::TypeLiteral { value };
        let literal_type_id = state.types.insert_type_from(literal_type, literal_id);
        state.types.set_inferred_type(
            literal_id.into_global_any(state.tree.module_id),
            literal_type_id,
        );

        literal_id
    }

    /// Insert a boolean-typed binary expression.
    pub(crate) fn insert_boolean_binary_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        operator: BinaryOperator,
        right: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // insert one binary expression
        let expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let expression_id = state.tree.insert_as_owner(
            expression_id,
            Expression::Binary {
                left,
                operator,
                right,
            },
        );

        // annotate with boolean type
        self.set_boolean_expression_type(state.types, state.tree.module_id, expression_id);
        expression_id
    }

    /// Insert a boolean-typed `is` type check expression.
    pub(crate) fn insert_is_type_check_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        left: LocalNodeId<Expression>,
        right: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // insert one type check expression
        let expression_id =
            state
                .tree
                .reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let expression_id = state.tree.insert_as_owner(
            expression_id,
            Expression::TypeBinary {
                left,
                operator: TypeBinaryOperator::Is,
                right,
            },
        );

        // annotate with boolean type
        self.set_boolean_expression_type(state.types, state.tree.module_id, expression_id);
        expression_id
    }
}
