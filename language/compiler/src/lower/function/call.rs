use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one call expression through its checked call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        let resolution = self.lowerer.call_resolution(expression)?;

        match &resolution.target {
            // free(...)
            dir::CallTarget::Symbol(candidate) => {
                // receiver.method(...)
                if candidate.receiver.is_some() || !candidate.adjustments.is_empty() {
                    self.lower_method_call(expression, &resolution, candidate)
                }
                // generic<T>(...) selects its materialized instance
                else if candidate.generic_scope.is_some() {
                    self.lower_generic_call(&resolution)
                } else if self.has_type_generic_arguments(candidate)? {
                    self.lower_instance_call(candidate, &resolution)
                }
                // local or imported (...)
                else {
                    self.lower_function_call(candidate.symbol, &resolution)
                }
            }
            // value(...)
            dir::CallTarget::Expression { .. } => self.lower_indirect_call(&resolution),
            // (a | b).method(...)
            dir::CallTarget::Universal(_) => self.lower_universal_call(&resolution),
        }
    }

    /// Return whether one candidate binds generic arguments beyond lifetimes.
    fn has_type_generic_arguments(&self, candidate: &dir::CallCandidate) -> CompilerResult<bool> {
        for binding in &candidate.generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.lowerer.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Lower one call to a declared or imported function.
    fn lower_function_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared MIR function behind the symbol
        let function = self.function_for(symbol, &[])?;

        // lower the bound arguments in parameter order
        let mut values = Vec::with_capacity(resolution.arguments.len());
        for binding in &resolution.arguments {
            let dir::ArgumentSource::Provided(source) = binding.argument else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted or spread argument".to_string(),
                }
                .into());
            };
            values.push(self.lower_argument(source)?);
        }

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one method call through its checked candidate.
    fn lower_method_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
        candidate: &dir::CallCandidate,
    ) -> CompilerResult<Option<mir::Value>> {
        if candidate.symbol.module_id != self.lowerer.module {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method call into another module".to_string(),
            }
            .into());
        }

        // the callee member names the receiver expression
        let dir::Expression::Call { left: callee, .. } =
            *self.lowerer.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR called a method outside a call expression".to_string(),
            });
        };
        let dir::Expression::Member { left: receiver, .. } =
            *self.lowerer.source().tree().get(callee)
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method call without a member callee".to_string(),
            }
            .into());
        };

        // sealed adjustments take the receiver through its place
        let receiver = match candidate.adjustments.as_slice() {
            [] => self.lower_expression(receiver)?,
            [dir::Projection::Borrow { ty, .. }] => {
                let target = self.lowerer.lower_type_id(self.builder.tree_mut(), *ty)?;

                self.lower_borrowed_place(receiver, target)?
            }
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a '{other:?}' receiver adjustment"),
                }
                .into());
            }
        };

        // resolve the declared MIR function behind the method symbol
        let function = self.function_for(candidate.symbol, &[])?;

        // bind the arguments after the receiver
        let mut values = Vec::with_capacity(resolution.arguments.len() + 1);
        values.push(receiver);
        for binding in &resolution.arguments {
            let dir::ArgumentSource::Provided(source) = binding.argument else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted or spread argument".to_string(),
                }
                .into());
            };
            values.push(self.lower_argument(source)?);
        }

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one call instantiating a generic callable.
    fn lower_generic_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a generically scoped call".to_string(),
        }
        .into())
    }

    /// Lower one call to a materialized generic instance.
    fn lower_instance_call(
        &mut self,
        candidate: &dir::CallCandidate,
        resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        // the substituted arguments select the declared instance by carrier
        let arguments = self
            .lowerer
            .instance_arguments(candidate, &self.lowerer.substitution)?;
        let key = self
            .lowerer
            .instance_key(self.builder.tree_mut(), &arguments)?;
        let function = self.function_for(candidate.symbol, &key)?;

        // lower the bound arguments in parameter order
        let mut values = Vec::with_capacity(resolution.arguments.len());
        for binding in &resolution.arguments {
            let dir::ArgumentSource::Provided(source) = binding.argument else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted or spread argument".to_string(),
                }
                .into());
            };
            values.push(self.lower_argument(source)?);
        }

        Ok(self.builder.call_function(function, values))
    }

    /// Return the MIR function behind one callable symbol and its instance key.
    fn function_for(
        &self,
        symbol: dir::GlobalSymbolId,
        key: &[mir::Type],
    ) -> CompilerResult<mir::FunctionId> {
        self.lowerer
            .functions
            .get(&(symbol, key.to_vec()))
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR is missing a declared function behind one call symbol"
                    .to_string(),
            })
    }

    /// Lower one call through a function-typed value.
    fn lower_indirect_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "an indirect call".to_string(),
        }
        .into())
    }

    /// Lower one call selected across every union member.
    fn lower_universal_call(
        &mut self,
        _resolution: &dir::CallResolution,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a universal call".to_string(),
        }
        .into())
    }

    /// Lower one provided argument source to its value.
    pub(in crate::lower) fn lower_argument(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        let expression = self.argument_expression(source)?;

        self.lower_expression(expression)
    }

    /// Return the value expression provided by one checked argument source.
    pub(in crate::lower) fn argument_expression(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::LocalNodeId<dir::Expression>> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            let value = match self.lowerer.source().tree().get(argument) {
                dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } => *value,
                dir::Argument::Spread { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a spread argument".to_string(),
                    }
                    .into());
                }
                dir::Argument::Elision | dir::Argument::Error => {
                    return Err(CompilerError::Internal {
                        message: "checked DIR provided an empty argument".to_string(),
                    });
                }
            };

            return Ok(value);
        }

        // lower the source as the value expression itself
        let Ok(expression) = source.local_id.try_into_typed::<dir::Expression>() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "checked DIR provided a non-expression argument node {}",
                    source.local_id.id
                ),
            });
        };

        Ok(expression)
    }
}
