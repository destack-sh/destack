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
        // require the call to resolve to exactly one callable
        let resolution = self.call_decision(expression)?;
        let dir::OperationResolution::One(call) = &resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a call on a union receiver".to_string(),
            }
            .into());
        };

        // lower by the callable the resolution selected
        match &call.target {
            // free(...)
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                // route intrinsic and binding callables before declared functions
                match self.lowerer.callable_implementation(function.key.symbol)? {
                    Some(CallableImplementation::Intrinsic { name }) => {
                        return self.lower_intrinsic_call(expression, name, call);
                    }
                    Some(CallableImplementation::Binding { .. }) => {
                        return self.lower_binding_call(function.key.symbol, call);
                    }
                    None => {}
                }

                // receiver.method(...)
                if function.receiver.is_some() {
                    self.lower_method_call(expression, call, function)
                }
                // generic<T>(...) selects its concrete instance
                else if self.has_instance_arguments(function)? {
                    self.lower_instance_call(function, call)
                }
                // local or imported (...)
                else {
                    self.lower_function_call(function.key.symbol, call)
                }
            }
            // value(...)
            dir::CallableTarget::Expression { .. } => self.lower_indirect_call(expression, call),
            // erased.method(...) dispatches through the constraint's entries
            dir::CallableTarget::Dynamic { dispatch, .. } => {
                self.lower_dynamic_call(expression, call, dispatch)
            }
            // reject virtual calls
            dir::CallableTarget::Symbol { .. } => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a virtual call".to_string(),
            }
            .into()),
        }
    }

    /// Return whether one function target binds generic arguments beyond lifetimes.
    fn has_instance_arguments(&self, function: &dir::FunctionTarget) -> CompilerResult<bool> {
        // any argument beyond a region parameter selects a concrete instance
        for binding in &function.key.arguments {
            let parameter = binding.parameter;
            let generics = &self.lowerer.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.memory_parameter() != Some(dir::MemoryParameter::Region) {
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
        // call the declared extern behind the binding
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;
        let parameters = self.function_parameters(function);
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, None)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Return the declared parameter representations of one function.
    pub(in crate::lower) fn function_parameters(
        &self,
        function: mir::FunctionId,
    ) -> Vec<mir::TypeId> {
        self.builder
            .tree()
            .get(function)
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect()
    }

    /// Return the parameter representations of one callable signature.
    pub(in crate::lower) fn signature_parameters(
        &self,
        signature: mir::TypeId,
    ) -> CompilerResult<Vec<mir::TypeId>> {
        // read each parameter representation the signature declares
        match self.builder.tree().get(signature) {
            mir::Type::FunctionSignature { parameters, .. } => {
                Ok(parameters.iter().map(|parameter| parameter.ty).collect())
            }
            _ => Err(CompilerError::Internal {
                message: "a call through a non-signature callable type".to_string(),
            }),
        }
    }

    /// Lower one argument list against its declared parameter representations.
    ///
    /// An empty parameter list lowers the arguments without representation adaptation.
    pub(in crate::lower) fn lower_call_arguments(
        &mut self,
        arguments: &[dir::ArgumentBinding],
        parameters: &[mir::TypeId],
        write: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Vec<mir::Value>> {
        // lower each argument at its parameter representation
        let mut values = Vec::with_capacity(arguments.len());
        for (index, binding) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).copied();
            let value = match binding.source {
                // lower a written argument
                dir::ArgumentSource::Provided(source) => self.lower_argument(source)?,
                // store omission at the parameter representation
                dir::ArgumentSource::Omitted => {
                    values.push(self.lower_omitted_argument(binding.parameter_type, parameter)?);

                    continue;
                }
                // fill the write argument with the assigned value for setter calls
                dir::ArgumentSource::Write => {
                    let Some(expression) = write else {
                        return Err(CompilerError::Internal {
                            message: "ordinary call lowering received an implicit write argument"
                                .to_string(),
                        });
                    };

                    self.lower_expression(expression)?
                }
                // materialize a static argument as its literal constant
                dir::ArgumentSource::Static(argument) => {
                    let dir::Type::Literal(literal) = self.lowerer.ty(argument)? else {
                        return Err(LowerError::Unsupported {
                            anchor: self.lowerer.module.into(),
                            construct: "a non-literal static argument".to_string(),
                        }
                        .into());
                    };
                    let representation = self.lower_type(argument)?;
                    let representation = self.builder.tree().get(representation).clone();

                    self.lower_constant(literal, representation)?
                }
                // pack the trailing arguments into the rest collection
                dir::ArgumentSource::Rest {
                    ref elements,
                    ref pack,
                } => {
                    let elements = elements.clone();
                    let pack = pack.clone();

                    self.lower_rest_pack(&elements, binding.argument_type, pack.as_ref())?
                }
            };

            // adapt the value to its declared parameter representation
            match parameter {
                Some(parameter) => values.push(self.adapt_to_representation(value, parameter)?),
                None => values.push(value),
            }
        }

        Ok(values)
    }

    /// Pack the rest elements into their parameter's collection.
    pub(in crate::lower) fn lower_rest_pack(
        &mut self,
        elements: &[dir::GlobalNodeIdAny],
        element_type: dir::GlobalTypeId,
        pack: Option<&dir::InstanceKey>,
    ) -> CompilerResult<mir::Value> {
        // materialize the elements into fixed stack storage
        let element = self.lower_type(element_type)?;
        let mut values = Vec::with_capacity(elements.len());
        for source in elements {
            values.push(self.lower_argument(*source)?);
        }

        // store the aggregate in one frame slot
        let storage = self.builder.tree_mut().intern_type(mir::Type::FixedArray {
            element: mir::TypeId::from(element),
            length: values.len() as u64,
            copy: mir::Copy::No,
        });
        let aggregate = self.builder.aggregate(storage, values);
        let slot = self.builder.local(storage, mir::Mutability::Immutable);
        self.builder.local_set(slot, aggregate);

        // view the storage as a borrowed slice of the elements
        let address = self.insert_reference(
            mir::ReferenceKind::Borrowed,
            mir::Access::Readonly,
            mir::Storage::Frame,
            storage,
        );
        let address = self.builder.local_addr(slot, address);
        let slice = self.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            element: mir::TypeId::from(element),
            storage: mir::Storage::Frame,
            access: mir::Access::Readonly,
            nullability: mir::Nullability::None,
        });
        let start = self.builder.iconst(0, 64, false);
        let length = self.builder.usize_const(elements.len() as u128);
        let view = self.builder.slice_view(address, start, length, slice);

        // take the view directly for a slice parameter
        let Some(pack) = pack else {
            return Ok(view);
        };

        // build the collection by calling its pack constructor over the view
        let function = self.selection_function(pack)?;
        let parameters = self.function_parameters(function);
        let Some(parameter) = parameters.first() else {
            return Err(CompilerError::Internal {
                message: "a pack constructor without a declared slice slot".to_string(),
            });
        };
        let view = self.adapt_to_representation(view, *parameter)?;
        let Some(packed) = self.builder.call_function(function, vec![view]) else {
            return Err(CompilerError::Internal {
                message: "a pack constructor call without a value".to_string(),
            });
        };

        Ok(packed)
    }

    /// Lower one omitted argument at its parameter representation.
    fn lower_omitted_argument(
        &mut self,
        ty: dir::GlobalTypeId,
        parameter: Option<mir::TypeId>,
    ) -> CompilerResult<mir::Value> {
        // take the parameter representation, else the declared type
        let representation = match parameter {
            Some(parameter) => parameter,
            None => self.lower_type(ty)?,
        };

        Ok(self.absent_argument_value(representation))
    }

    /// Lower one call to a declared or imported function.
    fn lower_function_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared function behind the symbol
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;
        let parameters = self.function_parameters(function);
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, None)?;

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
        // require the receiver the selection named
        let adjusted = function
            .receiver
            .as_ref()
            .ok_or_else(|| CompilerError::Internal {
                message: "method call has no selected receiver".to_string(),
            })?;

        // apply the selected receiver adjustments
        let receiver = self.lower_adjusted_receiver(receiver, adjusted)?;

        // resolve the declared function behind the selected method instance
        let function = self.selection_function(&function.key)?;
        let parameters = self.function_parameters(function);
        let Some((_, parameters)) = parameters.split_first() else {
            return Err(CompilerError::Internal {
                message: "a method call without a declared receiver slot".to_string(),
            });
        };

        // bind the arguments after the receiver
        let mut values = vec![receiver];
        values.extend(self.lower_call_arguments(&resolution.arguments, parameters, write)?);

        Ok(self.builder.call_function(function, values))
    }

    /// Lower one call to a concrete generic instance.
    fn lower_instance_call(
        &mut self,
        function: &dir::FunctionTarget,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // select the declared instance from the substituted arguments
        let function = self.selection_function(&function.key)?;
        let parameters = self.function_parameters(function);
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, None)?;

        Ok(self.builder.call_function(function, values))
    }

    /// Return the instance key one symbol takes under one selection's arguments.
    pub(in crate::lower) fn selection_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<GenericInstanceKey> {
        // resolve the selection's arguments through the enclosing instance
        let bindings = self
            .lowerer
            .instance_bindings(&selection.arguments, self.instance)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

        // resolve the receiver through the enclosing instance's types
        let receiver = match selection.receiver {
            Some(receiver) => Some(self.lowerer.instance_type(self.instance, receiver)?),
            None => None,
        };

        self.generic_instance_key(symbol, receiver, &arguments)
    }

    /// Return the declared function behind one selection's instance.
    pub(in crate::lower) fn selection_function(
        &mut self,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<mir::FunctionId> {
        let key = self.selection_key(selection.symbol, selection)?;

        self.function(&key)
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

        // call the function each declaration state names
        match declaration {
            // call the declared instance
            Some(FunctionDeclaration::Declared(function)) => Ok(*function),
            // cascade from declarations that already reported their diagnostics
            Some(FunctionDeclaration::Failed) => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a call into an undeclared callable".to_string(),
            }
            .into()),
            // an undeclared symbol reaching a call is a declaration failure
            None => {
                let path = self.lowerer.symbol_path(key.symbol)?;
                let declared: Vec<_> = self
                    .lowerer
                    .functions
                    .keys()
                    .filter(|candidate| candidate.symbol == key.symbol)
                    .map(|candidate| format!("{:?}", candidate.arguments))
                    .collect();

                Err(CompilerError::Internal {
                    message: format!(
                        "missing a declared function behind the callable symbol '{path}' at {:?}; declared: {declared:?}",
                        key.arguments
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
        // lower the callee value out of the call expression
        let dir::Expression::Call { left, .. } = *self.source().tree().get(expression) else {
            return Err(CompilerError::Internal {
                message: "a non-call through the indirect path".to_string(),
            });
        };
        let callee = self.lower_expression(left)?;

        // call through the value's declared signature
        let ty = self.value_representation(callee)?;
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
        let parameters = self.signature_parameters(mir::TypeId::from(signature))?;
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, None)?;

        Ok(self
            .builder
            .call(mir::Callee::Indirect { value: callee }, signature, values))
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
