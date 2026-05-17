use destack_dir as dir;
use dir::{
    Block, Expression, IfCondition, LocalNodeId, LocalScope, LocalScopeId, MatchCase, MatchForm,
    MatchOrigin, MatchSelector, NodeType, Pattern, SymbolForm, SymbolRole, Type, TypeLiteral,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateError, ElaborateResult};

impl Compiler {
    /// Normalize if let expressions into match expressions.
    pub(super) fn transform_if_let(&self, state: &mut ElaborateState<'_>) -> ElaborateResult<()> {
        // collect if let expressions to transform
        let if_ids: Vec<_> = state
            .tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_active_in_state(state, id.into_any()))
            .filter(|id| {
                matches!(
                    state.tree.get(*id),
                    Expression::If {
                        condition: IfCondition::Let { .. },
                        ..
                    }
                )
            })
            .collect();

        // transform each if let into a match with a default case
        for if_id in if_ids {
            // read the if expression
            let Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } = state.tree.get(if_id).clone()
            else {
                continue;
            };

            // read the if let condition
            let IfCondition::Let { declarator, .. } = condition else {
                continue;
            };

            // read the declarator and matched value
            let declarator = state.tree.get(declarator).clone();
            let Some(value_id) = declarator.value else {
                continue;
            };

            // resolve the match metadata for the new expression
            let match_scope = state.tree.get_scope(declarator.pattern);
            let match_symbol = self.match_symbol_for_scope(state, match_scope.0)?;

            // build the then case from the pattern and then expression
            let then_case = self.insert_if_let_case(
                state,
                if_id,
                declarator.pattern,
                then_expression,
                state.tree.get_scope(then_expression).0,
            );

            // build the else case with a wildcard pattern
            let else_expression = match else_expression {
                Some(else_expression) => else_expression,
                None => self.insert_empty_block_expression(state, if_id, match_scope),
            };
            let else_pattern = self.insert_if_let_wildcard(state, if_id, match_scope);
            let else_case = self.insert_if_let_case(
                state,
                if_id,
                else_pattern,
                else_expression,
                state.tree.get_scope(else_expression).0,
            );

            // replace the if let with a match expression
            let match_expression = Expression::Match {
                form: MatchForm::Match,
                value: value_id,
                cases: vec![then_case, else_case],
                source: MatchOrigin::Match,
                scope: match_scope.0,
                symbol: match_symbol,
            };
            state.tree.replace(if_id, match_expression);

            // preserve the original if expression type on the match node
            let match_type_id = state.type_table().get_declared_or_inferred_type_id(if_id.into_global_any(state.tree.module_id));
            if let Some(match_type_id) = match_type_id {
                self.set_expression_type(state.types_tail, state.tree.module_id, if_id, match_type_id);
            } else {
                let then_type_id = state.type_table().get_declared_or_inferred_type_id(
                    then_expression.into_global_any(state.tree.module_id),
                );
                let else_type_id = state.type_table().get_declared_or_inferred_type_id(
                    else_expression.into_global_any(state.tree.module_id),
                );
                let (Some(then_type_id), Some(else_type_id)) = (then_type_id, else_type_id) else {
                    return Err(ElaborateError::UnsupportedConstruct {
                        anchor: state.module_id.into(),
                    });
                };

                let match_type_id = if then_type_id == else_type_id {
                    then_type_id
                } else {
                    self.union_type_from_list(
                        vec![then_type_id, else_type_id],
                        then_type_id,
                        state.types,
                        state.types_tail,
                    )
                };
                self.set_expression_type(state.types_tail, state.tree.module_id, if_id, match_type_id);
            }
        }

        Ok(())
    }

    /// Pick a match symbol from the current scope chain.
    fn match_symbol_for_scope(
        &self,
        state: &mut ElaborateState<'_>,
        scope_id: LocalScopeId,
    ) -> ElaborateResult<dir::LocalSymbolId> {
        let scope_mark = state.symbols.get_scope_mark(scope_id);
        let (symbol_id, _) = state.symbols.insert_symbol(
            SymbolRole::Item,
            SymbolForm::Variable,
            None,
            (scope_id, scope_mark),
            None,
        );
        Ok(symbol_id)
    }

    /// Insert a wildcard pattern node for the implicit else case.
    fn insert_if_let_wildcard(
        &self,
        state: &mut ElaborateState<'_>,
        if_id: LocalNodeId<Expression>,
        scope: LocalScope,
    ) -> LocalNodeId<Pattern> {
        // allocate and insert the wildcard pattern
        let pattern_id = state.tree.reserve_from(
            NodeType::Pattern,
            if_id.into_any(),
            scope,
            None,
        );
        state.tree.insert_as_owner(pattern_id, Pattern::Wildcard)
    }

    /// Insert a match case for an if let branch.
    fn insert_if_let_case(
        &self,
        state: &mut ElaborateState<'_>,
        if_id: LocalNodeId<Expression>,
        pattern: LocalNodeId<Pattern>,
        body: LocalNodeId<Expression>,
        scope_id: LocalScopeId,
    ) -> LocalNodeId<MatchCase> {
        // allocate and insert the match case
        let scope = state.tree.get_scope(if_id);
        let case_id = state.tree.reserve_from(
            NodeType::MatchCase,
            if_id.into_any(),
            scope,
            None,
        );
        state.tree.insert_as_owner(
            case_id,
            MatchCase::Expression {
                selector: MatchSelector::Pattern {
                    pattern,
                    guard: None,
                },
                body,
                scope: scope_id,
            },
        )
    }

    /// Insert an empty block expression for missing else branches.
    fn insert_empty_block_expression(
        &self,
        state: &mut ElaborateState<'_>,
        if_id: LocalNodeId<Expression>,
        scope: LocalScope,
    ) -> LocalNodeId<Expression> {
        // allocate the empty block
        let block_id = state.tree.reserve_from(
            NodeType::Block,
            if_id.into_any(),
            scope,
            None,
        );
        let block: LocalNodeId<Block> = state.tree.insert_as_owner(
            block_id,
            Block {
                context: dir::BlockContext::Expression,
                form: dir::BlockForm::Explicit,
                scope: scope.0,
                leading_expressions: Vec::new(),
                tail_expression: None,
            },
        );

        // wrap the block as an expression
        let block_expr_id = state.tree.reserve_from(
            NodeType::Expression,
            if_id.into_any(),
            scope,
            None,
        );
        let block_expr_id = state
            .tree
            .insert_as_owner(block_expr_id, Expression::Block(block));

        // record void types for the synthesized block
        let void_type = Type::Literal(dir::LiteralType {
            value: TypeLiteral::Void,
        });
        let void_type_id = state.types_tail.insert_type_from(void_type, block);
        let module_id = state.types_tail.module_id;
        state
            .types_tail
            .set_inferred_type(block.into_global_any(module_id), void_type_id);
        state
            .types_tail
            .set_inferred_type(block_expr_id.into_global_any(module_id), void_type_id);

        block_expr_id
    }
}
