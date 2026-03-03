use destack_dir::{
    Expression, IfCondition, IfKind, LocalNodeId, LocalTypeId, NodeTree, NodeType, Resolution,
    ResolutionCandidate, SymbolTable, Type, TypeBinaryOperator, TypeTable,
};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify dynamic resolutions into explicit type checks.
    ///
    /// Transforms `Resolution::Dynamic` into if-else chains with `is` type checks
    /// and `Resolution::Static` branches. After this pass, only `Builtin` and
    /// `Static` resolutions remain in the DIR.
    pub(super) fn reify_resolution(
        &self,
        module_id: ModuleId,
        _profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &mut NodeTree,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> ElaborateResult<()> {
        // get the resolution for this expression
        let Some(resolution_id) =
            types.get_resolution_for_node(expression_id.into_global_any(module_id))
        else {
            return Ok(());
        };
        let resolution = types.get_resolution(resolution_id).clone();

        // only transform Dynamic resolutions
        let Resolution::Dynamic {
            receiver,
            candidates,
        } = resolution
        else {
            return Ok(());
        };

        // need at least 2 candidates to split
        if candidates.len() < 2 {
            return Ok(());
        }

        // need a receiver type to dispatch on
        let Some(_receiver_type_id) = receiver else {
            return Ok(());
        };

        // get the receiver expression from the original expression
        let expression = tree.get(expression_id).clone();
        let Some(receiver_expression_id) = self.receiver_expression_for(&expression, tree) else {
            return Ok(());
        };

        // get the scope for creating synthetic nodes
        let scope = tree.get_scope(expression_id);

        // build the if-else chain from bottom up
        // start with the last candidate as the final else branch
        let mut else_branch = self.build_static_resolution_branch(
            module_id,
            expression_id,
            &expression,
            &candidates[candidates.len() - 1],
            tree,
            types,
            scope,
        )?;

        // build if-else chain for remaining candidates (in reverse order)
        for candidate in candidates.iter().rev().skip(1) {
            // get the type to check against from the dispatch key
            let Some(check_type_id) = self.type_for_dispatch_candidate(candidate, types) else {
                continue;
            };

            // build: if (receiver is CheckType) { staticBranch } else { elseBranch }
            let then_branch = self.build_static_resolution_branch(
                module_id,
                expression_id,
                &expression,
                candidate,
                tree,
                types,
                scope,
            )?;

            // build the is check: receiver is Type
            let is_check = self.build_is_type_check(
                module_id,
                expression_id,
                receiver_expression_id,
                check_type_id,
                tree,
                types,
                scope,
            )?;

            // build the if expression
            let if_id =
                tree.reserve_from(NodeType::Expression, expression_id.into_any(), scope, None);
            else_branch = tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression {
                        condition: is_check,
                    },
                    then_expression: then_branch,
                    else_expression: Some(else_branch),
                },
            );

            // set the type for the if expression (same as original expression)
            if let Some(expr_type_id) =
                types.get_declared_or_inferred_type_id(expression_id.into_global_any(module_id))
            {
                types.set_inferred_type(if_id.into_global(module_id), expr_type_id);
            }
        }

        // replace the original expression with the if-else chain
        let final_expression = tree.get(else_branch).clone();
        tree.replace(expression_id, final_expression);

        Ok(())
    }

    /// Get the receiver expression from an expression that has a resolution.
    fn receiver_expression_for(
        &self,
        expression: &Expression,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<Expression>> {
        match expression {
            // binary operators: receiver is the left operand
            Expression::Binary { left, .. } => Some(*left),
            // unary operators: receiver is the operand
            Expression::Unary { right, .. } => Some(*right),
            // call expressions: receiver is from the left (could be member access)
            Expression::Call { left, .. } => {
                // unwrap parenthesized callee
                let mut callee_id = *left;
                loop {
                    let Expression::Parenthesized { expression } = tree.get(callee_id) else {
                        break;
                    };
                    callee_id = *expression;
                }

                // use the receiver for member calls
                if let Expression::Member { left, .. } = tree.get(callee_id) {
                    Some(*left)
                } else {
                    Some(callee_id)
                }
            }
            // member access: receiver is the left
            Expression::Member { left, .. } => Some(*left),
            // index access: receiver is the left
            Expression::Index { left, .. } => Some(*left),
            _ => None,
        }
    }

    /// Get the type to check from a resolution candidate's dispatch key.
    fn type_for_dispatch_candidate(
        &self,
        candidate: &ResolutionCandidate,
        _types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // the dispatch key contains the type(s) for runtime selection
        let key = candidate.key.as_ref()?;

        // for single dispatch, use the single type
        // for multiple dispatch, we'd need more complex logic
        let types = key.types();
        types.first().copied()
    }

    /// Build a branch with static resolution for a candidate.
    fn build_static_resolution_branch(
        &self,
        module_id: ModuleId,
        origin_id: LocalNodeId<Expression>,
        expression: &Expression,
        candidate: &ResolutionCandidate,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // clone the original expression
        let cloned_id = tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let cloned_id = tree.insert(cloned_id, expression.clone());

        // copy the original type to the cloned expression
        if let Some(type_id) =
            types.get_declared_or_inferred_type_id(origin_id.into_global_any(module_id))
        {
            types.set_inferred_type(cloned_id.into_global_any(module_id), type_id);
        }

        // get the receiver type for the static resolution
        let receiver_type = self.type_for_dispatch_candidate(candidate, types);

        // create a static resolution for this branch
        let static_resolution = Resolution::Static {
            receiver: receiver_type,
            candidate: candidate.clone(),
        };
        let resolution_id = types.insert_resolution(static_resolution);
        types.set_resolution_for_node(cloned_id.into_global_any(module_id), resolution_id);

        Ok(cloned_id)
    }

    /// Build an `is` type check expression: `value is Type`.
    fn build_is_type_check(
        &self,
        module_id: ModuleId,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        check_type_id: LocalTypeId,
        tree: &mut NodeTree,
        types: &mut TypeTable,
        scope: (destack_dir::LocalScopeId, destack_dir::LocalScopeMark),
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // create the type expression for the right side of `is`
        let type_expr_id =
            tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let type_expression = match types.get_type(check_type_id) {
            Type::TypeLiteral { value } => Expression::TypeLiteral {
                value: value.clone(),
            },
            _ => Expression::Type {
                value: check_type_id,
            },
        };
        let type_expr_id = tree.insert(type_expr_id, type_expression);

        // set the type for the type expression (Type::Value wrapping the check type)
        let type_value = Type::Value {
            value: check_type_id,
        };
        let type_value_id = types.insert_type_from(type_value, type_expr_id);
        types.set_inferred_type(type_expr_id.into_global_any(module_id), type_value_id);

        // build the is expression: value is Type
        let is_id = tree.reserve_from(NodeType::Expression, origin_id.into_any(), scope, None);
        let is_id = tree.insert(
            is_id,
            Expression::TypeBinary {
                left: value_id,
                operator: TypeBinaryOperator::Is,
                right: type_expr_id,
            },
        );

        // set the type for the is expression (boolean)
        let bool_type = Type::TypeLiteral {
            value: destack_dir::TypeLiteral::Primitive(destack_dir::PrimitiveType::Boolean),
        };
        let bool_type_id = types.insert_type_from(bool_type, is_id);
        types.set_inferred_type(is_id.into_global_any(module_id), bool_type_id);

        Ok(is_id)
    }
}
