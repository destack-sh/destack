use tspp_dir as dir;

use crate::lower::{Binding, FunctionLowerer};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one statement, returning whether it terminated the block.
    pub(in crate::lower) fn lower_statement(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        self.lower_anchored(statement, |lower| {
            match lower.source().tree().get(statement).clone() {
                // return the function result
                dir::Expression::Return { value } => {
                    let value = value.map(|value| lower.lower_value(value)).transpose()?;
                    lower.dispose_down_to(0)?;
                    lower.return_value(value)?;

                    Ok(true)
                }

                // bind the resources of a using, disposed at the scope exit
                dir::Expression::Using { declarators, .. } => {
                    lower.lower_using(&declarators)?;

                    Ok(false)
                }

                // bind the declarators of a let
                dir::Expression::Let { declarators, .. } => {
                    lower.lower_let(&declarators)?;

                    Ok(false)
                }

                // assign through the target place
                dir::Expression::Assign {
                    left,
                    operator,
                    right,
                } => {
                    lower.lower_assign(statement, left, operator, right)?;

                    Ok(false)
                }

                // update the target in place
                dir::Expression::Unary {
                    operator:
                        dir::UnaryOperator::PostIncrement
                        | dir::UnaryOperator::PostDecrement
                        | dir::UnaryOperator::PreIncrement
                        | dir::UnaryOperator::PreDecrement,
                    right,
                } => {
                    lower.lower_update(statement, right)?;

                    Ok(false)
                }

                // branch on the condition
                dir::Expression::If {
                    form: dir::IfForm::If,
                    condition,
                    then_expression,
                    else_expression,
                } => lower.lower_if(&condition, then_expression, else_expression),

                // emit a breakpoint
                dir::Expression::Debugger => {
                    lower.builder.breakpoint();

                    Ok(false)
                }

                // dispatch on the switch value
                dir::Expression::Switch { value, cases } => lower.lower_switch(value, &cases),

                // dispatch on the matched value
                dir::Expression::Match { value, arms } => lower.lower_match_statement(value, &arms),

                // run the body, its residuals caught, then the finally on every way out
                dir::Expression::Try {
                    body,
                    catch,
                    finally,
                } => Ok(!lower.lower_try(statement, body, catch, finally, None)?),

                // run an optional chain for its effects
                dir::Expression::Chain { .. } => {
                    lower.lower_value(statement)?;

                    Ok(false)
                }

                // loop while the condition holds
                dir::Expression::While {
                    label,
                    form,
                    condition,
                    body,
                } => lower.lower_while(label, form, &condition, body),

                // loop over the initialization, condition, and increment
                dir::Expression::For {
                    label,
                    initialization,
                    condition,
                    increment,
                    body,
                } => lower.lower_for(label, initialization, condition, increment, body),

                // loop unconditionally
                dir::Expression::Loop { label, body } => lower.lower_loop(label, body),

                // iterate the source through its recorded protocol calls
                dir::Expression::ForEach {
                    label,
                    binding,
                    iterator,
                    body,
                    ..
                } => lower.lower_for_each(statement, label, binding, iterator, body),

                // break out of the enclosing statement
                dir::Expression::Break { label, value } => lower.lower_break(label, value),

                // continue the enclosing loop
                dir::Expression::Continue { label } => lower.lower_continue(label),

                // construct a value in statement position
                dir::Expression::Call { .. }
                    if let Some(resolution) = lower.construct_decision(statement) =>
                {
                    lower.lower_construct(statement, &resolution)?;

                    Ok(false)
                }

                // call in statement position, an await running its recorded park call
                dir::Expression::Call { .. } | dir::Expression::Await { .. } => {
                    lower.lower_call(statement)?;

                    Ok(false)
                }

                // yield in statement position
                dir::Expression::Yield { .. } => {
                    lower.lower_yield(statement)?;

                    Ok(false)
                }

                // reject every other statement
                other => Err(lower.unsupported(format!("'{}' statements", other.variant_name()))),
            }
        })
    }

    /// Lower one let statement's declarators.
    fn lower_let(
        &mut self,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            // read the declarator's pattern and initializer
            let declarator = self.source().tree().get(*declarator_id);
            let (pattern, value) = (declarator.pattern, declarator.value);
            let Some(value) = value else {
                return Err(self.unsupported("an uninitialized let binding"));
            };

            // bind destructuring patterns through the pattern walker
            let dir::Pattern::Binding { pattern: None, .. } = self.source().tree().get(pattern)
            else {
                let place = self.lower_place(value)?;
                self.lower_anchored(value, |lower| lower.lower_pattern_bindings(pattern, &place))?;

                continue;
            };

            // resolve the binding symbol at the pattern node
            let node = pattern.into_global_any(self.source);
            let Some(symbol) = self.lower.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one let binding".to_string(),
                });
            };

            // evaluate the initializer and bind it
            let value = self.lower_value(value)?;
            self.bind_symbol(symbol, value)?;
        }

        Ok(())
    }

    /// Resolve the defaulted parameters at the head of one declared body.
    pub(in crate::lower) fn lower_parameter_defaults(
        &mut self,
        parameters: &[dir::LocalSymbolId],
        defaults: &[Option<dir::LocalNodeId<dir::Expression>>],
    ) -> CompilerResult<()> {
        for (index, default) in defaults.iter().enumerate() {
            let Some(default) = *default else {
                continue;
            };

            // read the value bound for the parameter on entry
            let symbol = parameters[index];
            let Some(Binding::Local(home)) = self.values.get(&symbol).copied() else {
                return Err(CompilerError::Internal {
                    message: "a defaulted parameter without its home".to_string(),
                });
            };
            let incoming = self.builder.local_get(home);

            // keep the parameters already incoming at their bound type
            let ty = self.lower.symbol_type(symbol.into_global(self.source))?;
            let exact = self.lower_type(ty)?;
            if Some(exact) == self.builder.value_type(incoming) {
                continue;
            }

            // unwrap the present value or evaluate the default
            let declaration = self
                .lower
                .declaration_node(symbol.into_global(self.source))?
                .ok_or_else(|| self.internal("a defaulted parameter without its declaration"))?;
            let source = self.node_type_id(declaration.local_id)?;
            let resolved = self.lower_absent_fallback(incoming, source, ty, |lower| {
                lower.lower_value(default).map(Some)
            })?;
            let local = self.home(resolved);
            self.values.insert(symbol, Binding::Local(local));
        }

        Ok(())
    }
}
