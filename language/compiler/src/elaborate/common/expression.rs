use destack_dir as dir;
use dir::{
    BinaryOperator, Expression, GenericArgument, LocalNodeId, LocalTypeId, NodeType, Type,
    TypeExpression, TypeLiteral,
};

use crate::Compiler;
use crate::elaborate::ElaborateState;

impl Compiler {
    /// Clone one type generic argument into the requested scope.
    fn clone_type_generic_argument_into_scope(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<GenericArgument>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<GenericArgument> {
        let argument = state.tree.get(argument_id).clone();

        // clone type-only generic arguments and reject value-space ones loudly
        let cloned_argument = match argument {
            GenericArgument::Type { value } => {
                let value = self.clone_type_expression_into_scope(state, origin_id, value, scope);
                GenericArgument::Type { value }
            }
            GenericArgument::SpreadType { value } => {
                let value = self.clone_type_expression_into_scope(state, origin_id, value, scope);
                GenericArgument::SpreadType { value }
            }
            GenericArgument::Value { .. } | GenericArgument::SpreadValue { .. } => {
                todo!("FUGU #Incomplete: clone value generic arguments in elaborate guards")
            }
            GenericArgument::Error => GenericArgument::Error,
        };

        // insert the cloned generic argument
        let cloned_id = state.tree.reserve_from(
            NodeType::GenericArgument,
            origin_id.into_any(),
            scope,
            None,
        );
        state.tree.insert_as_owner(cloned_id, cloned_argument)
    }

    /// Clone one type expression into the requested scope.
    fn clone_type_expression_into_scope(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        expression_id: LocalNodeId<TypeExpression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<TypeExpression> {
        let expression = state.tree.get(expression_id).clone();

        // clone the type-expression shapes used by elaborated type synthesis
        let cloned_expression = match expression {
            TypeExpression::Parenthesized { expression } => {
                let expression =
                    self.clone_type_expression_into_scope(state, origin_id, expression, scope);
                TypeExpression::Parenthesized { expression }
            }
            TypeExpression::ScalarLiteral { value } => TypeExpression::ScalarLiteral { value },
            TypeExpression::Literal { value } => TypeExpression::Literal { value },
            TypeExpression::Intrinsic => TypeExpression::Intrinsic,
            TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let generic_arguments = generic_arguments
                    .into_iter()
                    .map(|argument_id| {
                        self.clone_type_generic_argument_into_scope(
                            state,
                            origin_id,
                            argument_id,
                            scope,
                        )
                    })
                    .collect();

                TypeExpression::Reference {
                    path,
                    generic_arguments,
                }
            }
            _ => todo!("FUGU #Incomplete: clone elaborate guard type expressions"),
        };

        // insert the cloned type expression
        let cloned_id = state.tree.reserve_from(
            NodeType::TypeExpression,
            origin_id.into_any(),
            scope,
            None,
        );
        let cloned_id = state.tree.insert_as_owner(cloned_id, cloned_expression);
        state.types_tail.copy_node_relations(
            expression_id.into_global_any(state.module_id),
            cloned_id.into_global_any(state.module_id),
        );
        state.resolutions_tail.copy_node_relations(
            expression_id.into_global_any(state.module_id),
            cloned_id.into_global_any(state.module_id),
        );

        cloned_id
    }

    /// Insert a type expression for one local type id.
    pub(crate) fn insert_type_expression_for_type_id(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        type_id: LocalTypeId,
        scope: dir::LocalScope,
    ) -> LocalNodeId<TypeExpression> {
        // choose the compact type expression form
        let expression = match state.type_table().get_type(type_id) {
            Type::Literal(dir::LiteralType { value }) => TypeExpression::Literal {
                value: value.clone(),
            },
            _ => {
                let source_id = state.type_table().get_type_source(type_id);
                let source_expression_id = match source_id.ty {
                    // clone one existing type expression source when available
                    NodeType::TypeExpression => source_id.into_typed::<TypeExpression>(),

                    // unwrap prior type and value expressions back to their type expression
                    NodeType::Expression => {
                        let expression = state.tree.get(source_id.into_typed::<Expression>());
                        let Expression::Type { value, .. } = expression else {
                            panic!("type value source must be a type expression");
                        };

                        *value
                    }

                    // fail loudly on unexpected type sources
                    _ => panic!("type value source must resolve to type syntax"),
                };

                return self.clone_type_expression_into_scope(
                    state,
                    origin_id,
                    source_expression_id,
                    scope,
                );
            }
        };

        // insert one elaborated type expression
        let expression_id = state.tree.reserve_from(
            NodeType::TypeExpression,
            origin_id.into_any(),
            scope,
            None,
        );
        state.tree.insert_as_owner(expression_id, expression)
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
        let literal_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let literal_id = state.tree.insert_as_owner(
            literal_id,
            Expression::TypeLiteral {
                value: value.clone(),
            },
        );

        // annotate with its literal type
        let literal_type = Type::Literal(dir::LiteralType { value });
        let literal_type_id = state.types_tail.insert_type_from(literal_type, literal_id);
        state.types_tail.set_inferred_type(
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
        let expression_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let expression_id = state.tree.insert_as_owner(
            expression_id,
            Expression::Binary {
                left,
                operator,
                right,
            },
        );

        // annotate with boolean type
        self.set_boolean_expression_type(state.types_tail, state.tree.module_id, expression_id);
        expression_id
    }

    /// Insert a boolean-typed `is` type check expression.
    pub(crate) fn insert_is_type_check_expression(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value: LocalNodeId<Expression>,
        target_type: LocalNodeId<TypeExpression>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        // insert one type check expression
        let expression_id = state.tree.reserve_from(
            NodeType::Expression,
            origin_id.into_any(),
            scope,
            None,
        );
        let expression_id = state
            .tree
            .insert_as_owner(expression_id, Expression::Is { value, target_type });

        // annotate with boolean type
        self.set_boolean_expression_type(state.types_tail, state.tree.module_id, expression_id);
        expression_id
    }
}
