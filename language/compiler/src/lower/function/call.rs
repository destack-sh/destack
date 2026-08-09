use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    CallableImplementation, FunctionDeclaration, FunctionLowerer, GenericInstanceKey,
};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one call expression through its call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        let resolution = self.call_decision(expression)?;
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
                // route intrinsic and binding callables before declared functions
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
            dir::CallTarget::Expression { .. } => self.lower_indirect_call(expression, call),
            // erased.method(...) dispatches through the constraint's entries
            dir::CallTarget::Dynamic { dispatch, .. } => {
                self.lower_dynamic_call(expression, call, dispatch)
            }
            // reject virtual calls
            dir::CallTarget::Symbol { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a virtual call".to_string(),
            }
            .into()),
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

    /// Lower one binding call through its declared dotted extern.
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
        self.lower_call_arguments(resolution, None)
    }

    /// Lower one resolution's arguments, filling its write slot when one exists.
    pub(in crate::lower) fn lower_call_arguments(
        &mut self,
        resolution: &dir::Call,
        write: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Vec<mir::Value>> {
        let arguments = &resolution.arguments;
        let mut values = Vec::with_capacity(arguments.len());
        for binding in arguments {
            let source = match binding.argument {
                dir::ArgumentSource::Provided(source) => source,
                // pass the undefined slot for omitted optional parameters
                dir::ArgumentSource::Omitted => {
                    values.push(self.lower_omitted_argument(binding.ty)?);

                    continue;
                }
                // fill the write slot with the assigned value for setter calls
                dir::ArgumentSource::Write => {
                    let Some(expression) = write else {
                        return Err(CompilerError::Internal {
                            message: "ordinary call lowering received an implicit write argument"
                                .to_string(),
                        });
                    };
                    values.push(self.lower_expression(expression)?);

                    continue;
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
        // resolve the declared function behind the symbol
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one method call through its candidate.
    fn lower_method_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<Option<mir::Value>> {
        // read the receiver expression from the callee member
        let dir::Expression::Call { left: callee, .. } = *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "a method call outside a call expression".to_string(),
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

        self.lower_function_target_call(receiver, resolution, function, None)
    }

    /// Lower one function target over one explicit receiver expression.
    pub(in crate::lower) fn lower_function_target_call(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
        write: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<mir::Value>> {
        let adjusted = function
            .receiver
            .as_ref()
            .ok_or_else(|| CompilerError::Internal {
                message: "method call has no selected receiver".to_string(),
            })?;

        // apply the selected receiver adjustments
        let receiver = self.lower_adjusted_receiver(receiver, adjusted)?;

        // resolve the declared function behind the selected method instance
        let bindings = self
            .lowerer
            .instance_bindings(&function.generic_arguments, &self.type_substitution)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(function.symbol, &arguments)?;
        let function = self.function(&key)?;

        // bind the arguments after the receiver
        let mut values = vec![receiver];
        values.extend(self.lower_call_arguments(resolution, write)?);

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
        // select the declared instance from the substituted arguments
        let bindings = self
            .lowerer
            .instance_bindings(&function.generic_arguments, &self.type_substitution)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(function.symbol, &arguments)?;
        let function = self.function(&key)?;
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Return the function behind one callable symbol and its instance key.
    pub(in crate::lower) fn function(
        &self,
        key: &GenericInstanceKey,
    ) -> CompilerResult<mir::FunctionId> {
        // resolve the exact instance, treating a failed symbol as a failed instance
        let symbol = GenericInstanceKey::non_generic(key.symbol);
        let declaration = match self.lowerer.functions.get(key) {
            Some(declaration) => Some(declaration),
            None => self
                .lowerer
                .functions
                .get(&symbol)
                .filter(|fallback| matches!(fallback, FunctionDeclaration::Failed)),
        };

        match declaration {
            Some(FunctionDeclaration::Declared(function)) => Ok(*function),
            // cascade from declarations that already reported their diagnostics
            Some(FunctionDeclaration::Failed) => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a call into an undeclared callable".to_string(),
            }
            .into()),
            None => {
                let path = self.lowerer.symbol_path(key.symbol)?;

                Err(CompilerError::Internal {
                    message: format!(
                        "missing a declared function behind the callable symbol '{path}'"
                    ),
                })
            }
        }
    }

    /// Lower one call through a function-typed value.
    fn lower_indirect_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let dir::Expression::Call { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "a non-call through the indirect path".to_string(),
            });
        };
        let callee = self.lower_expression(left)?;

        // call through the value's declared signature
        let Some(ty) = self.builder.value_type(callee) else {
            return Err(CompilerError::Internal {
                message: "the lowered callee value has no type".to_string(),
            });
        };
        let signature = match self.builder.tree().get(ty) {
            mir::Type::Function { signature, .. } | mir::Type::FunctionPointer { signature } => {
                *signature
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "a call through a non-callable value".to_string(),
                });
            }
        };
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self
            .builder
            .call(mir::Callee::Indirect { value: callee }, signature, values))
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

        // store undefined directly in reference-like carriers
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
                dir::Argument::Positional { value } => *value,
                dir::Argument::Spread { .. } => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a spread argument".to_string(),
                    }
                    .into());
                }
                dir::Argument::Elision | dir::Argument::Error => {
                    return Err(CompilerError::Internal {
                        message: "an empty argument".to_string(),
                    });
                }
            };

            return self.lower_expression(value);
        }

        // lower the source as the value expression itself
        let Ok(expression) = source.local_id.try_into_typed::<dir::Expression>() else {
            return Err(CompilerError::Internal {
                message: format!("a non-expression argument node {}", source.local_id.id),
            });
        };

        self.lower_expression(expression)
    }
}
