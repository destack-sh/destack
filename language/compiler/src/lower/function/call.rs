use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{CallableImplementation, FunctionLowerer, GenericInstanceKey};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one call expression through its checked call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        let resolution = self.call_resolution(expression)?;
        let dir::OperationResolution::One(call) = &resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a call on a union receiver".to_string(),
            }
            .into());
        };

        match &call.target {
            // free(...)
            dir::CallTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                // sealed intrinsic and binding callables bypass declared functions
                match self.lowerer.callable_implementation(function.symbol)? {
                    Some(CallableImplementation::Intrinsic { name }) => {
                        return self.lower_intrinsic_call(name, call);
                    }
                    Some(CallableImplementation::Binding { .. }) => {
                        return self.lower_binding_call(function.symbol, call);
                    }
                    None => {}
                }

                // receiver.method(...)
                if function.receiver.is_some() {
                    self.lower_method_call(expression, call, function)
                }
                // generic<T>(...) selects its concrete instance
                else if function.generic_scope.is_some() {
                    self.lower_generic_call(call)
                } else if self.has_instance_arguments(function)? {
                    self.lower_instance_call(function, call)
                }
                // local or imported (...)
                else {
                    self.lower_function_call(function.symbol, call)
                }
            }
            // value(...)
            dir::CallTarget::Expression { .. } => self.lower_indirect_call(call),
            // virtual and erased calls require their selected dispatch table
            dir::CallTarget::Symbol { .. } | dir::CallTarget::Dynamic { .. } => {
                Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a virtual or dynamic call".to_string(),
                }
                .into())
            }
        }
    }

    /// Return whether one function target binds generic arguments beyond lifetimes.
    fn has_instance_arguments(&self, function: &dir::FunctionTarget) -> CompilerResult<bool> {
        for binding in &function.generic_arguments {
            let parameter = binding.parameter;
            let generics = &self.lowerer.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.memory_parameter() != Some(dir::MemoryParameter::Lifetime) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Lower one sealed binding call through its declared dotted extern.
    fn lower_binding_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Lower the provided arguments of one resolution in parameter order.
    pub(in crate::lower) fn lower_provided_arguments(
        &mut self,
        resolution: &dir::Call,
    ) -> CompilerResult<Vec<mir::Value>> {
        let arguments = &resolution.arguments;
        let mut values = Vec::with_capacity(arguments.len());
        for binding in arguments {
            let source = match binding.argument {
                dir::ArgumentSource::Provided(source) => source,
                // omitted optional parameters receive their undefined slot
                dir::ArgumentSource::Omitted => {
                    values.push(self.lower_omitted_argument(binding.ty)?);

                    continue;
                }
                dir::ArgumentSource::Write => {
                    return Err(CompilerError::Internal {
                        message: "ordinary call lowering received an implicit write argument"
                            .to_string(),
                    });
                }
                dir::ArgumentSource::Static(_) | dir::ArgumentSource::Rest(_) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a static or rest argument".to_string(),
                    }
                    .into());
                }
            };
            values.push(self.lower_argument(source)?);
        }

        Ok(values)
    }

    /// Lower one call to a declared or imported function.
    fn lower_function_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared MIR function behind the symbol
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one method call through its checked candidate.
    fn lower_method_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<Option<mir::Value>> {
        // the callee member names the receiver expression
        let dir::Expression::Call { left: callee, .. } = *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "checked DIR called a method outside a call expression".to_string(),
            });
        };
        let dir::Expression::Member { left: receiver, .. } = *self.source().tree().get(callee)
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a method call without a member callee".to_string(),
            }
            .into());
        };

        self.lower_function_target_call(receiver, resolution, function)
    }

    /// Lower one function target over one explicit receiver expression.
    pub(in crate::lower) fn lower_function_target_call(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<Option<mir::Value>> {
        let adjusted = function
            .receiver
            .as_ref()
            .ok_or_else(|| CompilerError::Internal {
                message: "method call has no selected receiver".to_string(),
            })?;

        // apply the selected receiver adjustments
        let receiver = match adjusted.adjustments.as_slice() {
            [] => self.lower_expression(receiver)?,
            [dir::ReceiverAdjustment::Borrow { ty }] => {
                let target = self.lower_type(*ty)?;

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
        let function = self.function(&GenericInstanceKey::non_generic(function.symbol))?;

        // bind the arguments after the receiver
        let mut values = vec![receiver];
        values.extend(self.lower_provided_arguments(resolution)?);

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one call instantiating a generic callable.
    fn lower_generic_call(
        &mut self,
        _resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "a generically scoped call".to_string(),
        }
        .into())
    }

    /// Lower one call to a concrete generic instance.
    fn lower_instance_call(
        &mut self,
        function: &dir::FunctionTarget,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // the substituted arguments select the declared instance
        let arguments = self
            .lowerer
            .instance_arguments(function, &self.type_substitution)?;
        let key = self
            .type_lowerer()
            .generic_instance_key(function.symbol, &arguments)?;
        let function = self.function(&key)?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Return the MIR function behind one callable symbol and its instance key.
    fn function(&self, key: &GenericInstanceKey) -> CompilerResult<mir::FunctionId> {
        self.lowerer
            .functions
            .get(key)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked DIR is missing a declared function behind one call symbol"
                    .to_string(),
            })
    }

    /// Lower one call through a function-typed value.
    fn lower_indirect_call(
        &mut self,
        _resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        Err(LowerError::Unsupported {
            anchor: self.lowerer.module.into(),
            construct: "an indirect call".to_string(),
        }
        .into())
    }

    /// Lower one omitted optional argument to its undefined slot value.
    fn lower_omitted_argument(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<mir::Value> {
        let reduced = self.lowerer.reduced_type(ty)?;
        let carrier = self.lower_type(reduced)?;

        // inject the undefined case into union carriers
        if let dir::Type::Union(_) = self.lowerer.ty(reduced)? {
            let members = self.union_members(reduced)?;
            for (index, member) in members.into_iter().enumerate() {
                let member = self.lowerer.reduced_type(member)?;
                if matches!(self.lowerer.ty(member)?, dir::Type::Undefined) {
                    return Ok(self.builder.variant_new(carrier, index as u32, None));
                }
            }
        }

        // reference-like carriers store undefined directly
        Ok(self.builder.constant(mir::Constant::Undefined, carrier))
    }

    /// Lower one provided argument source to its value.
    pub(in crate::lower) fn lower_argument(
        &mut self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<mir::Value> {
        // unwrap the provided value from argument nodes
        if let Ok(argument) = source.local_id.try_into_typed::<dir::Argument>() {
            let value = match self.source().tree().get(argument) {
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

            return self.lower_expression(value);
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

        self.lower_expression(expression)
    }
}
