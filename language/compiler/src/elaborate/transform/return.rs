use destack_dir as dir;
use dir::{
    Declaration, Expression, GlobalSymbolId, IfKind, LocalNodeId, LocalTypeId, Member, Type,
    TypeLiteral,
};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform implicit returns into explicit `return` statements.
    ///
    /// ```ds
    /// function f() { 42 }
    /// ```
    /// ->
    /// ```ds
    /// function f() { return 42; }
    /// ```
    ///
    /// This also handles if/else branches:
    /// ```ds
    /// function f(cond) {
    ///     if (cond) { 1 }
    ///     else { 2 }
    /// }
    /// ```
    /// ->
    /// ```ds
    /// function f(cond) {
    ///     if (cond) { return 1; }
    ///     else { return 2; }
    /// }
    /// ```
    pub(super) fn transform_explicit_return(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        // collect function and method bodies to rewrite
        let mut body_ids = Vec::new();
        let module_id = state.types.module_id;
        for declaration_id in state.tree.iter_node_ids_of_type::<Declaration>() {
            if !self.is_active_in_state(state, declaration_id.into_any()) {
                continue;
            }

            let declaration = state.tree.get(declaration_id).clone();

            // function declarations
            if let Declaration::Function(declaration) = &declaration
                && let Some(body_id) = declaration.body
            {
                let symbol = declaration.symbol.into_global(module_id);
                if self.function_returns_void(state, symbol) {
                    continue;
                }
                body_ids.push(body_id);
            }

            // method bodies on structured declarations
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    let Member::Method {
                        body: Some(body_id),
                        symbol,
                        ..
                    } = state.tree.get(*member_id)
                    else {
                        continue;
                    };

                    let symbol = symbol.into_global(module_id);
                    if self.function_returns_void(state, symbol) {
                        continue;
                    }
                    body_ids.push(*body_id);
                }
            }
        }

        // rewrite implicit returns in each body
        for body_id in body_ids {
            let scope = state.tree.get_scope(body_id);
            self.make_return_explicit(state, body_id, scope)?;
        }

        Ok(())
    }

    /// Make the implicit return in an expression explicit.
    /// For blocks, this transforms the last expression.
    /// For if/else, this transforms both branches recursively.
    fn make_return_explicit(
        &self,
        state: &mut ElaborateState<'_>,
        expr_id: LocalNodeId<Expression>,
        scope: dir::LocalScope,
    ) -> ElaborateResult<()> {
        let expr = state.tree.get(expr_id).clone();

        match expr {
            Expression::Block(block) => {
                let block_node = state.tree.get::<dir::Block>(block).clone();
                if let Some(last_expr_id) = block_node.tail_expression {
                    // recursively transform the last expression
                    self.make_return_explicit(state, last_expr_id, scope)?;
                }

                self.update_block_expression_type(state, block, expr_id)?;
            }

            Expression::If {
                kind: IfKind::If,
                condition: _,
                then_expression,
                else_expression,
            } => {
                // for if else (not ternary), transform both branches
                self.make_return_explicit(state, then_expression, scope)?;
                if let Some(else_expr) = else_expression {
                    self.make_return_explicit(state, else_expr, scope)?;
                }

                self.set_void_expression_type(state.types, state.types.module_id, expr_id);
            }

            Expression::If {
                kind: IfKind::Ternary,
                ..
            } => {
                // for ternary, return the whole expression as a statement
                self.replace_expression_with_explicit_return(state, expr_id, scope);
            }

            Expression::Match { .. } => {
                unreachable!("match should be transformed to if/else in transform_match_chain");
            }

            Expression::Return { .. } => {
                // already explicit, do nothing
            }

            Expression::Let { .. }
            | Expression::Using { .. }
            | Expression::Labelled { .. }
            | Expression::Loop { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Break { .. }
            | Expression::UnresolvedBreak { .. }
            | Expression::Continue { .. }
            | Expression::UnresolvedContinue { .. }
            | Expression::Throw { .. }
            | Expression::Debugger => {}

            _ => {
                let Some(type_id) = state
                    .types
                    .get_declared_or_inferred_type_id(expr_id.into_global_any(state.ctx.module_id))
                else {
                    self.replace_expression_with_explicit_return(state, expr_id, scope);
                    return Ok(());
                };

                if self.type_is_void_or_never(state, type_id) {
                    return Ok(());
                }

                // wrap value expression in return statement
                self.replace_expression_with_explicit_return(state, expr_id, scope);
            }
        }

        Ok(())
    }

    /// Check whether a function symbol has an explicit void return type.
    fn function_returns_void(&self, state: &ElaborateState<'_>, symbol: GlobalSymbolId) -> bool {
        let Some(value_type_id) = state.types.get_value_type_id(symbol) else {
            return false;
        };

        self.return_type_is_void(state, value_type_id)
    }

    /// Check whether a function return type is void.
    fn return_type_is_void(&self, state: &ElaborateState<'_>, type_id: LocalTypeId) -> bool {
        match state.types.get_type(type_id) {
            Type::Function { return_type, .. } => {
                let Some(return_type_id) = return_type else {
                    return false;
                };
                self.type_is_void(state, *return_type_id)
            }
            Type::Value { value } => self.return_type_is_void(state, *value),
            _ => false,
        }
    }

    /// Check whether a type id resolves to void.
    fn type_is_void(&self, state: &ElaborateState<'_>, type_id: LocalTypeId) -> bool {
        match state.types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Void,
            } => true,
            Type::Value { value } => self.type_is_void(state, *value),
            _ => false,
        }
    }

    /// Check whether a type id resolves to void or never.
    fn type_is_void_or_never(&self, state: &ElaborateState<'_>, type_id: LocalTypeId) -> bool {
        match state.types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Void | TypeLiteral::Never,
            } => true,
            Type::Value { value } => self.type_is_void_or_never(state, *value),
            _ => false,
        }
    }

    /// Update the inferred type for a block and its block expression.
    fn update_block_expression_type(
        &self,
        state: &mut ElaborateState<'_>,
        block_id: dir::LocalNodeId<dir::Block>,
        expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        let block = state.tree.get(block_id);
        let type_id = match block.tail_expression {
            Some(expression_id) => {
                self.expression_type_id_or_error(state.types.module_id, expression_id, state.types)?
            }
            None => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                state.types.insert_type_from(ty, block_id)
            }
        };

        state
            .types
            .set_inferred_type(block_id.into_global_any(state.types.module_id), type_id);
        state.types.set_inferred_type(
            expression_id.into_global_any(state.types.module_id),
            type_id,
        );

        Ok(())
    }
}
