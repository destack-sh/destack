use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{Binding, FunctionLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

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
                    let value = value
                        .map(|value| lower.lower_expression(value))
                        .transpose()?;
                    lower.builder.return_(value);

                    Ok(true)
                }

                // bind the declarators of a let
                dir::Expression::Let {
                    mutability,
                    declarators,
                    ..
                } => {
                    lower.lower_let(mutability, &declarators)?;

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

                // run an optional chain for its effects
                dir::Expression::Chain { .. } => {
                    lower.lower_expression(statement)?;

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

                // call in statement position
                dir::Expression::Call { .. } => {
                    lower.lower_call(statement)?;

                    Ok(false)
                }

                // reject every other statement
                other => Err(LowerError::Unsupported {
                    anchor: lower.lower.module.into(),
                    construct: format!("'{}' statements", other.variant_name()),
                }
                .into()),
            }
        })
    }

    /// Lower one let statement's declarators.
    fn lower_let(
        &mut self,
        mutability: dir::Mutability,
        declarators: &[dir::LocalNodeId<dir::Declarator>],
    ) -> CompilerResult<()> {
        for declarator_id in declarators {
            // read the declarator's pattern and initializer
            let declarator = self.source().tree().get(*declarator_id);
            let (pattern, value) = (declarator.pattern, declarator.value);
            let Some(value) = value else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "an uninitialized let binding".to_string(),
                }
                .into());
            };

            // bind destructuring patterns through the pattern walker
            let dir::Pattern::Binding { pattern: None, .. } = self.source().tree().get(pattern)
            else {
                let value = self.lower_expression(value)?;
                self.lower_pattern_bindings(pattern, value, mutability)?;

                continue;
            };

            // resolve the binding symbol at the pattern node
            let node = pattern.into_global_any(self.source);
            let Some(symbol) = self.lower.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one let binding".to_string(),
                });
            };

            // evaluate the initializer
            let value = self.lower_expression(value)?;

            // give lifted bindings their frame home ahead of local storage
            if self.bind_lifted(symbol, value)? {
                continue;
            }

            // keep immutable bindings as pure values; give mutable ones a local
            let binding = match mutability {
                dir::Mutability::Immutable => Binding::Value(value),
                _ => {
                    let ty = self.lower.symbol_type(symbol)?;
                    let ty = self.lower_type(ty)?;
                    let local = self.builder.local(ty, mir::Mutability::Mutable);
                    self.builder.local_set(local, value);

                    Binding::Local(local)
                }
            };

            self.values.insert(symbol.local_id, binding);
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
            let Some(Binding::Value(incoming)) = self.values.get(&symbol).copied() else {
                return Err(CompilerError::Internal {
                    message: "a defaulted parameter without its bound value".to_string(),
                });
            };

            // keep the parameters already incoming at their bound type
            let ty = self.lower.symbol_type(symbol.into_global(self.source))?;
            let exact = self.lower_type(ty)?;
            if Some(exact) == self.builder.value_type(incoming) {
                continue;
            }

            // unwrap the present value or evaluate the default
            let resolved = self.lower_absent_fallback(incoming, exact, |lower| {
                lower.lower_expression(default).map(Some)
            })?;
            self.values.insert(symbol, Binding::Value(resolved));
        }

        Ok(())
    }
}
