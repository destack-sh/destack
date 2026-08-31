use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{
    Binding, CallableImplementation, FunctionDeclaration, FunctionLowerer, GenericInstanceKey,
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
                anchor: self.lower.module.into(),
                construct: "a call on a union receiver".to_string(),
            }
            .into());
        };

        // lower by the callable the resolution selected
        match &call.target {
            // call a directly dispatched symbol
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                // route intrinsic and binding callables before declared functions
                match self.lower.callable_implementation(function.key.symbol)? {
                    Some(CallableImplementation::Intrinsic { name }) => {
                        return self.lower_intrinsic_call(expression, name, call);
                    }
                    Some(CallableImplementation::Binding { .. }) => {
                        return self.lower_function_call(function.key.symbol, call);
                    }
                    None => {}
                }

                // call a method over its selected receiver
                if function.receiver.is_some() {
                    self.lower_method_call(expression, call, function)
                }
                // call the concrete instance a generic selection names
                else if self.has_instance_arguments(function)? {
                    self.lower_instance_call(function, call)
                }
                // call the local or imported declaration
                else {
                    self.lower_function_call(function.key.symbol, call)
                }
            }
            // call through a function-typed value
            dir::CallableTarget::Expression { .. } => self.lower_indirect_call(expression, call),
            // dispatch through the erased receiver's constraint entries
            dir::CallableTarget::Dynamic { dispatch, .. } => {
                self.lower_dynamic_call(expression, call, dispatch)
            }
            // reject virtual calls
            dir::CallableTarget::Symbol { .. } => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a virtual call".to_string(),
            }
            .into()),
        }
    }

    /// Return whether one function target binds generic arguments beyond lifetimes.
    fn has_instance_arguments(&self, function: &dir::FunctionTarget) -> CompilerResult<bool> {
        // accept any argument beyond a region parameter
        for binding in &function.key.arguments {
            let parameter = binding.parameter;
            let generics = &self.lower.state(parameter.module_id)?.generics;
            let declared = generics.get_parameter(parameter.local_id);
            if declared.memory_parameter() != Some(dir::MemoryParameter::Region) {
                return Ok(true);
            }
        }

        Ok(false)
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
            // reject a call through a non-signature type
            _ => Err(CompilerError::Internal {
                message: "a call through a non-signature callable type".to_string(),
            }),
        }
    }

    /// Lower one call to a declared or imported function.
    fn lower_function_call(
        &mut self,
        symbol: dir::GlobalSymbolId,
        resolution: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // resolve the declared function behind the symbol
        let function = self.function(&GenericInstanceKey::non_generic(symbol))?;

        // lower the arguments at their declared parameter representations
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
                anchor: self.lower.module.into(),
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
                message: "a method call without a selected receiver".to_string(),
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

        // lower the arguments at their declared parameter representations
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
            .lower
            .instance_bindings(&selection.arguments, self.instance)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

        // resolve the receiver through the enclosing instance's types
        let receiver = match selection.receiver {
            Some(receiver) => Some(self.lower.instance_type(self.instance, receiver)?),
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
        &mut self,
        key: &GenericInstanceKey,
    ) -> CompilerResult<mir::FunctionId> {
        // declare the callable on its first reach from a body
        if !self.lower.functions.contains_key(key) {
            self.declare_callable(key)?;
        }

        // resolve the exact instance, treating a failed symbol as a failed instance
        let symbol = GenericInstanceKey::non_generic(key.symbol);
        let declaration = match self.lower.functions.get(key) {
            Some(declaration) => Some(declaration),
            None => self
                .lower
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
                anchor: self.lower.module.into(),
                construct: "a call into an undeclared callable".to_string(),
            }
            .into()),
            // report an undeclared symbol reached by a call
            None => {
                let path = self.lower.symbol_path(key.symbol)?;

                Err(CompilerError::Internal {
                    message: format!(
                        "a missing declared function behind the callable symbol '{path}'"
                    ),
                })
            }
        }
    }

    /// Declare one callable first reached from a body: a binding extern, an import, or a closure.
    fn declare_callable(&mut self, key: &GenericInstanceKey) -> CompilerResult<()> {
        // leave every instantiated key to its own declaration
        if key.receiver.is_some() || !key.arguments.is_empty() {
            return Ok(());
        }

        // declare a dotted host extern for a binding and emit an intrinsic inline
        let symbol = key.symbol;
        let (tree, effects) = self.builder.tree_and_effects_mut();
        match self.lower.callable_implementation(symbol)? {
            Some(CallableImplementation::Binding { .. }) => {
                return self.lower.declare_binding_function(tree, effects, symbol);
            }
            Some(CallableImplementation::Intrinsic { .. }) => return Ok(()),
            None => {}
        }

        // import a foreign concrete callable under its canonical name
        if symbol.module_id != self.lower.module {
            return self.lower.declare_imported_function(tree, symbol);
        }

        // declare and queue a local callable a body reaches as a closure
        let declared = self
            .lower
            .state(symbol.module_id)?
            .bindings
            .get_symbol(symbol.local_id)
            .declaration;
        if let Some(node) = declared
            && let Ok(declaration) = node.local_id.try_into_typed::<dir::Declaration>()
            && let dir::Declaration::Function(function) =
                self.lower.state(symbol.module_id)?.tree().get(declaration)
            && let Some(body) = function.body
        {
            let definition = self.lower.declare_function(tree, declaration, body)?;
            self.lower.pending.push(definition);
        }

        Ok(())
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
                message: "a non-call expression in an indirect call".to_string(),
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

    /// Bind the incoming receiver, giving owned receivers a mutable home.
    pub(in crate::lower) fn bind_receiver(&mut self, value: mir::Value) -> CompilerResult<Binding> {
        // take indirect receivers as values, writing through their reference
        let representation = self.value_representation(value)?;
        if self
            .builder
            .tree()
            .get(representation)
            .is_reference_representation()
        {
            return Ok(Binding::Value(value));
        }

        // give owned receivers a mutable local
        let local = self.builder.local(representation, mir::Mutability::Mutable);
        self.builder.local_set(local, value);

        Ok(Binding::Local(local))
    }

    /// Lower one receiver expression through its selected adjustments.
    pub(in crate::lower) fn lower_adjusted_receiver(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        adjusted: &dir::AdjustedReceiver,
    ) -> CompilerResult<mir::Value> {
        // take the receiver place directly when a borrow leads the adjustments
        let (value, rest) = match adjusted.adjustments.as_slice() {
            [dir::ReceiverAdjustment::Borrow { ty }, rest @ ..] => {
                let target = self.lower_type(*ty)?;

                (self.lower_borrowed_place(receiver, target)?, rest)
            }
            // otherwise lower the receiver as a value
            rest => (self.lower_expression(receiver)?, rest),
        };

        self.lower_receiver_adjustments(value, rest)
    }

    /// Apply receiver adjustments to one lowered value in order.
    fn lower_receiver_adjustments(
        &mut self,
        mut value: mir::Value,
        adjustments: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<mir::Value> {
        for adjustment in adjustments {
            value = match adjustment {
                // borrow spilled storage when no source place exists
                dir::ReceiverAdjustment::Borrow { ty } => {
                    let target = self.lower_type(*ty)?;

                    self.spill_borrow(value, target)?
                }
                // read through one reference or pointer receiver
                dir::ReceiverAdjustment::Dereference(dereference) => {
                    self.lower_dereference(value, dereference)?
                }
                // unwrap a newtype value or stored newtype place
                dir::ReceiverAdjustment::NewtypePayload { ty, .. } => {
                    let value_type = self.value_representation(value)?;

                    // retain the address form of stored receivers
                    match self.builder.tree().get(value_type) {
                        mir::Type::Reference { .. } | mir::Type::Pointer { .. } => {
                            let target = self.lower_type(*ty)?;

                            self.builder.field_addr(value, 0, target)
                        }
                        _ => self.builder.field_get(value, 0),
                    }
                }
                // project a narrowed union value or stored union place
                dir::ReceiverAdjustment::UnionPayload { union, arm, .. } => {
                    let members = self.union_members(*union)?;
                    let Some(index) = members.iter().position(|member| member == arm) else {
                        return Err(CompilerError::Internal {
                            message: "a union payload adjustment selecting an absent arm"
                                .to_string(),
                        });
                    };

                    let value_type = self.value_representation(value)?;

                    // retain the address form of stored receivers
                    match self.builder.tree().get(value_type) {
                        mir::Type::Reference { .. } | mir::Type::Pointer { .. } => {
                            let target = self.lower_type(adjustment.ty())?;

                            self.builder
                                .variant_payload_addr(value, index as u32, target)
                        }
                        _ => self.builder.variant_payload(value, index as u32),
                    }
                }
            };
        }

        Ok(value)
    }

    /// Borrow one value through spilled local storage.
    fn spill_borrow(
        &mut self,
        value: mir::Value,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let ty = self.value_representation(value)?;
        let local = self.builder.local(ty, mir::Mutability::Immutable);
        self.builder.local_set(local, value);

        Ok(self.builder.local_addr(local, target))
    }

    /// Dereference one receiver value through its selected target.
    fn lower_dereference(
        &mut self,
        value: mir::Value,
        dereference: &dir::Dereference,
    ) -> CompilerResult<mir::Value> {
        match &dereference.target {
            // load through the physical reference
            dir::DereferenceTarget::Direct => {
                let ty = self.lower_type(dereference.ty)?;

                Ok(self.builder.load(value, ty))
            }
            // call the selected method for a protocol dereference
            dir::DereferenceTarget::Call(call) => {
                let dir::CallableTarget::Symbol {
                    function,
                    dispatch: dir::FunctionDispatch::Direct,
                } = &call.target
                else {
                    return Err(CompilerError::Internal {
                        message: "a protocol dereference without a direct method".to_string(),
                    });
                };
                let target = self.selection_function(&function.key)?;
                let result = self.builder.call_function(target, vec![value]);

                result.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a protocol dereference".to_string(),
                })
            }
        }
    }
}
