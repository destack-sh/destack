use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::lower::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one let statement's declarators.
    pub(in crate::lower) fn lower_let(
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
                    anchor: self.lowerer.module.into(),
                    construct: "an uninitialized let binding".to_string(),
                }
                .into());
            };

            // destructuring patterns bind through the pattern walker
            let dir::Pattern::Binding { pattern: None, .. } = self.source().tree().get(pattern)
            else {
                let value = self.lower_expression(value)?;
                self.lower_pattern_bindings(pattern, value, mutability)?;

                continue;
            };

            // resolve the binding symbol at the pattern node
            let node = pattern.into_global_any(self.source);
            let Some(symbol) = self.lowerer.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "missing a symbol for one let binding".to_string(),
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
                    let ty = self.lowerer.symbol_type(symbol)?;
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

    /// Lower one assignment statement.
    pub(in crate::lower) fn lower_assign(
        &mut self,
        statement: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // require a place assignment
        let dir::AssignPatternDecision::Place = self.assign_resolution(left)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a destructuring assignment".to_string(),
            }
            .into());
        };
        let dir::AssignPattern::Place { expression } = *self.source().tree().get(left) else {
            return Err(CompilerError::Internal {
                message: "a non-place pattern resolved as a place".to_string(),
            });
        };

        // resolve how the place is written
        let resolution = self.assignment_decision(expression)?;

        // write through the setter member when one is selected
        if let dir::WriteResolution::Member(dir::OperationResolution::One(access)) =
            &resolution.write
            && let dir::MemberTarget::Call(call) = &access.target
        {
            let call = call.clone();

            self.lower_accessor_write(expression, operator, &call, right)
        }
        // store the right value directly for plain assignment
        else if operator == dir::AssignOperator::Assign {
            let place = self.place(&resolution)?;
            let value = self.lower_expression(right)?;
            self.write_place(&place, value)?;

            Ok(())
        }
        // apply the builtin operation for compound assignment
        else {
            let place = self.place(&resolution)?;
            let resolution = self.operator_decision(statement)?;
            let dir::OperationResolution::One(dir::OperatorApplication::Binary {
                operator,
                target: dir::OperatorTarget::Builtin(_),
                ..
            }) = resolution
            else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a protocol compound assignment".to_string(),
                }
                .into());
            };
            let current = self.read_place(&place)?;
            let operator = self.binary_operator(operator)?;
            let value = self.lower_expression(right)?;
            let value = self.builder.binary(operator, current, value);
            self.write_place(&place, value)?;

            Ok(())
        }
    }

    /// Lower one assignment into a setter member through its call.
    fn lower_accessor_write(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::AssignOperator,
        call: &dir::Call,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // accessors take a whole value
        if operator != dir::AssignOperator::Assign {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a compound assignment through an accessor".to_string(),
            }
            .into());
        }

        // require a statically selected setter
        let dir::CallableTarget::Symbol {
            function,
            dispatch: dir::FunctionDispatch::Direct,
        } = &call.target
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a dynamic property write".to_string(),
            }
            .into());
        };

        // read the receiver the setter is called on
        let dir::Expression::Member { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "an accessor write outside a member place".to_string(),
            });
        };

        // call the setter with the assigned value
        self.lower_function_target_call(left, call, function, Some(right))?;

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
            let ty = self.lowerer.symbol_type(symbol.into_global(self.source))?;
            let exact = self.lower_type(ty)?;
            if Some(exact) == self.builder.value_type(incoming) {
                continue;
            }

            // unwrap the present value or evaluate the default
            let resolved = self.lower_absent_fallback(incoming, exact, |lowerer| {
                lowerer.lower_expression(default).map(Some)
            })?;
            self.values.insert(symbol, Binding::Value(resolved));
        }

        Ok(())
    }
}
