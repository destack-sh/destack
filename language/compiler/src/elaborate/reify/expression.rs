use std::collections::HashSet;

use destack_dir as dir;
use destack_workspace::{Module, ProfileId};
use dir::{Expression, IfCondition, IfKind, LocalNodeId, MatchKind};

use crate::elaborate::common::{ElaborateContext, ElaborateState};
use crate::{Compiler, CompilerContext, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify a module to make abstractions concrete:
    /// - Implicit conversions → explicit cast nodes.
    /// - Operators → resolved method calls (`a + b` → `a.add(b)`).
    /// - Tree literals → constructor/function calls (`<div>` → `createElement(div, ...)`).
    /// - Nominal constructor calls → tagged expressions.
    pub(crate) fn elaborate_module_reify(
        &self,
        module: &Module,
        profile: ProfileId,
        context: &CompilerContext<'_>,
        tree: &mut dir::Tree,
        symbols: &mut dir::SymbolTable,
        types: &mut dir::TypeTable,
    ) -> ElaborateResult<()> {
        // skip non-code modules
        if !context.is_code_module(module.id) {
            return Ok(());
        }

        let ctx = ElaborateContext::new(context, module.id, module, profile);
        let mut state = ElaborateState::new(ctx, tree, symbols, types);

        // collect member expressions used as call or new callees
        let mut member_callees: HashSet<u32> = HashSet::new();
        for expression_id in state.tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_active_in_state(&state, expression_id.into_any()) {
                continue;
            }

            let (Expression::Call { left, .. } | Expression::New { left, .. }) =
                state.tree.get(expression_id)
            else {
                continue;
            };

            let mut callee_id = *left;
            loop {
                let Expression::Parenthesized { expression } = state.tree.get(callee_id) else {
                    break;
                };
                callee_id = *expression;
            }

            if matches!(state.tree.get(callee_id), Expression::Member { .. }) {
                member_callees.insert(callee_id.id);
            }
        }

        // reify expression nodes
        for expression_id in state.tree.iter_node_ids_of_type::<Expression>() {
            if !self.is_active_in_state(&state, expression_id.into_any()) {
                continue;
            }

            self.reify_expression(&mut state, expression_id, &member_callees)?
        }

        // normalize return if expressions introduced during reify
        self.transform_normalize_value_expressions(&mut state)?;

        // collapse redundant nested casts
        self.normalize_redundant_casts(&mut state)?;

        Ok(())
    }

    /// Reify an expression.
    fn reify_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        member_callees: &HashSet<u32>,
    ) -> ElaborateResult<()> {
        let expression = state.tree.get(expression_id).clone();
        match expression {
            // tree literals to constructor calls
            Expression::TreeExpression { .. } => {
                self.reify_tree_expression(state, expression_id)?;
            }

            Expression::As {
                operator: _,
                source: _,
                expression: left,
                target_type: right,
            } => {
                self.reify_explicit_cast_expression(state, expression_id, left, right)?;
            }

            // non-null assertion to explicit downcast
            Expression::Must { left } => {
                self.reify_must_expression(state, expression_id, left)?;
            }

            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                self.reify_implicit_casts_in_reference(state, expression_id, target_symbol)?;
            }

            Expression::Let { declarators, .. } | Expression::Using { declarators, .. } => {
                self.reify_implicit_casts_in_binding(state, &declarators)?;
            }

            Expression::Assign { left, right } => {
                self.reify_implicit_casts_in_assignment(state, expression_id, left, right)?;
            }

            Expression::Return { value } => {
                self.reify_implicit_casts_in_return(state, expression_id, value)?;
            }

            Expression::If {
                kind: IfKind::Ternary,
                condition,
                then_expression,
                else_expression,
            } => {
                if let IfCondition::Expression { condition } = condition {
                    self.reify_implicit_casts_in_ternary(
                        state,
                        expression_id,
                        condition,
                        then_expression,
                        else_expression,
                    )?;
                }
            }

            Expression::Match { kind, cases, .. } => {
                if kind == MatchKind::Match {
                    self.reify_implicit_casts_in_match(state, expression_id, &cases)?;
                }
            }

            // operators to resolved method calls
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                self.reify_implicit_casts_in_binary(state, expression_id, left, operator, right)?;
                self.reify_operator_expression(state, expression_id)?;
                self.reify_resolution(state, expression_id)?;
            }

            // unary operators may have resolutions
            Expression::Unary { .. } => {
                self.reify_operator_expression(state, expression_id)?;
                self.reify_resolution(state, expression_id)?;
            }

            // compound assignments keep their surface shape here
            Expression::AssignBinary {
                left,
                operator,
                right,
            } => {
                self.reify_implicit_casts_in_assign_binary(
                    state,
                    expression_id,
                    left,
                    operator,
                    right,
                )?;
            }

            // nominal constructor calls to tagged expressions
            Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => {
                let did_reify_constructor = self.reify_tagged_constructor_call(
                    state,
                    expression_id,
                    left,
                    &generic_arguments,
                    &arguments,
                )?;
                // only insert call casts when the expression stays as a call
                if !did_reify_constructor {
                    self.reify_implicit_casts_in_call(state, expression_id, &arguments)?;
                }
                self.reify_resolution(state, expression_id)?;
            }

            // member access may have resolutions (for union types with different fields)
            Expression::Member { .. } => {
                // skip member reify when used as a call callee
                if member_callees.contains(&expression_id.id) {
                    return Ok(());
                }

                // reify member resolution when applicable
                self.reify_resolution(state, expression_id)?;
            }

            // index access may have resolutions (for union types with different Index implementations)
            Expression::Index { .. } => {
                self.reify_resolution(state, expression_id)?;
            }

            // standalone try unwrap is still pending full control flow reification
            Expression::Maybe { .. } => {}

            // type expressions are consumed by cast and type-binary reification paths
            Expression::Type { .. } | Expression::TypeLiteral { .. } => {}

            _ => {}
        }

        Ok(())
    }
}
