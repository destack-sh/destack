use destack_dir as dir;
use destack_mir as mir;
use destack_mir::substitute_type;

use crate::lower::function::argument::Argument;
use crate::lower::function::operand::Operand;
use crate::lower::{
    Binding, CallableImplementation, FunctionDeclaration, FunctionLowerer, GenericInstanceKey,
    GenericScope, Instance, ModuleLowerer, Receiver,
};
use crate::{CompilerError, CompilerResult};

/// The use one lowered receiver serves, a member access reading storage and a call reading the
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::lower) enum ReceiverUse {
    /// The receiver's storage, projected by a member access.
    Storage,
    /// The receiver's value, passed to a call.
    Value,
}

/// One resolved callee with the signature the call takes it at.
pub(in crate::lower) struct Callee {
    /// The callee.
    pub(in crate::lower) target: mir::Callee,
    /// The signature the call takes the callee at.
    pub(in crate::lower) signature: mir::TypeId,
    /// The parameter types the call passes its arguments at, in the caller's regions.
    pub(in crate::lower) parameters: Vec<mir::TypeId>,
    /// The type the call defines its result at, in the caller's regions.
    pub(in crate::lower) result: mir::TypeId,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one call expression through its call resolution.
    pub(in crate::lower) fn lower_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Value>> {
        // require the call to resolve to exactly one callable
        let resolution = self.call_decision(expression)?;
        let dir::OperationResolution::One(call) = &resolution else {
            return Err(self.unsupported("a call on a union receiver"));
        };

        // lower by the callable the resolution selected
        match &call.target {
            // call a directly dispatched symbol
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                // instrument profile sites at their call sites, named by the receiver type
                if let Some(item) = self.lower.language_item(function.key.symbol)
                    && matches!(
                        item,
                        dir::LanguageItem::ProfileCounterIncrement
                            | dir::LanguageItem::ProfileSamplerSample
                    )
                {
                    return self.lower_profile_call(call, item);
                }

                // route intrinsic callables before declared functions, a method-form one over
                // its lowered receiver
                if let Some(CallableImplementation::Intrinsic { name }) =
                    self.lower.callable_implementation(function.key.symbol)?
                {
                    let receiver = match self.lower.callable_header(function.key.symbol)?.receiver {
                        Receiver::None => None,
                        Receiver::This(_) | Receiver::Constructs(_) => {
                            let (receiver, is_optional) = self.member_call_receiver(expression)?;
                            Some(match &function.receiver {
                                Some(adjusted) => self.lower_adjusted_receiver(
                                    receiver,
                                    adjusted,
                                    is_optional,
                                    ReceiverUse::Value,
                                )?,
                                None => self.lower_value(receiver)?,
                            })
                        }
                    };

                    return self.lower_intrinsic_call(expression, name, call, receiver);
                }

                // call a method over its selected receiver, else the selected function
                let is_method = function.receiver.is_some()
                    && matches!(
                        self.source().tree().get(expression),
                        dir::Expression::Call { .. }
                    );
                match is_method {
                    true => self.lower_method_call(expression, call, function),
                    false => self.lower_direct_call(None, &function.key, call),
                }
            }
            // call through a function-typed value
            dir::CallableTarget::Expression { .. } | dir::CallableTarget::Constructor(_) => {
                self.lower_indirect_call(expression, call)
            }
            // dispatch through the erased receiver's constraint entries
            dir::CallableTarget::Dynamic { dispatch, .. } => {
                self.lower_dynamic_call(expression, call, dispatch)
            }
            // reject virtual calls
            dir::CallableTarget::Symbol { .. } => Err(self.unsupported("a virtual call")),
        }
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

    /// Resolve one symbol at a selection's receiver and arguments.
    pub(in crate::lower) fn resolve_callee_of(
        &mut self,
        symbol: dir::GlobalSymbolId,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Callee> {
        let instance = self.instance_of(symbol, selection)?;

        Ok(self.callee_of(instance))
    }

    /// Return the callee of one instance, an applied template calling at its arguments.
    pub(in crate::lower) fn callee_of(&mut self, instance: Instance) -> Callee {
        let (function, arguments) = match instance {
            Instance::Declared(function) => (function, Vec::new()),
            Instance::Applied {
                template,
                arguments,
            } => (template, arguments),
        };
        let (signature, parameters, result) = self.function_signature(function, &arguments);

        Callee {
            target: mir::Callee::Direct {
                function,
                arguments,
            },
            signature,
            parameters,
            result,
        }
    }

    /// Instantiate one signature's late-bound regions.
    pub(in crate::lower) fn instantiate_signature(
        &mut self,
        parameters: &mut [mir::TypeId],
        result: &mut mir::TypeId,
        scope: &GenericScope,
        bindings: &[dir::GenericArgumentBinding],
        receiver: Option<mir::Lifetime>,
    ) -> CompilerResult<()> {
        // lower the lifetime bound at each slot in the caller's scope
        let slots = &scope.slots;
        let mut lifetimes = vec![None; scope.names.len()];
        if lifetimes.is_empty() {
            return Ok(());
        }
        if let Some(slot) = scope.receiver_slot {
            lifetimes[slot.0 as usize] = receiver;
        }
        for binding in bindings {
            let name = self.lower.format_parameter_name(binding.parameter)?;
            let Some(slot) = slots.get(&binding.parameter) else {
                if self.lower.is_region_parameter(binding.parameter)? {
                    return Err(CompilerError::Internal {
                        message: format!("a region bound at '{name}' without a slot at the call"),
                    });
                }
                continue;
            };
            let lifetime = self
                .lower
                .lower_lifetime(binding.argument, &self.scope)
                .map_err(|error| match error {
                    CompilerError::Internal { message } => CompilerError::Internal {
                        message: format!("{message} bound at '{name}'"),
                    },
                    error => error,
                })?;
            lifetimes[slot.0 as usize] = Some(lifetime);
        }
        let mut instantiation = Vec::with_capacity(lifetimes.len());
        for (slot, lifetime) in lifetimes.into_iter().enumerate() {
            let Some(lifetime) = lifetime else {
                let parameter = slots.get_index(slot).map(|(parameter, _)| *parameter);
                let name = match parameter {
                    Some(parameter) => self.lower.format_parameter_name(parameter)?,
                    None => "?".to_string(),
                };
                return Err(CompilerError::Internal {
                    message: format!("a region slot {slot} ({name}) unbound at a call"),
                });
            };
            instantiation.push(lifetime);
        }

        // instantiate the signature's parameters and result
        let tree = self.builder.tree_mut();
        for parameter in parameters {
            *parameter = mir::instantiate_slots(tree, *parameter, &instantiation);
        }
        *result = mir::instantiate_slots(tree, *result, &instantiation);

        Ok(())
    }

    /// Instantiate one symbol callee at the regions a call binds.
    pub(in crate::lower) fn instantiate_symbol_callee(
        &mut self,
        callee: &mut Callee,
        symbol: dir::GlobalSymbolId,
        call: &dir::Call,
    ) -> CompilerResult<()> {
        let scope = self.lower.symbol_scope(symbol)?;
        let path = self.lower.symbol_path(symbol)?;
        let Callee {
            parameters, result, ..
        } = callee;

        self.instantiate_signature(parameters, result, &scope, &call.regions, None)
            .map_err(|error| match error {
                CompilerError::Internal { message } => CompilerError::Internal {
                    message: format!("{message} calling '{path}'"),
                },
                error => error,
            })
    }

    /// Return the signature one function takes at generic arguments, with its parameters and
    /// result.
    fn function_signature(
        &mut self,
        function: mir::FunctionId,
        arguments: &[mir::GenericArgument],
    ) -> (mir::TypeId, Vec<mir::TypeId>, mir::TypeId) {
        let tree = self.builder.tree_mut();
        let declared = tree.get(function).clone();
        let parameters: Vec<_> = declared
            .parameters
            .iter()
            .map(|parameter| substitute_type(tree, parameter.ty, arguments))
            .collect();
        let result = substitute_type(tree, declared.return_type, arguments);
        let signature = tree.intern_type(mir::Type::FunctionSignature {
            lifetimes: declared.lifetimes,
            parameters: parameters
                .iter()
                .map(|parameter| mir::SignatureParameter::new(*parameter))
                .collect(),
            result,
        });

        (mir::TypeId::from(signature), parameters, result)
    }

    /// Call one callee at its signature.
    pub(in crate::lower) fn call(
        &mut self,
        callee: &Callee,
        values: Vec<mir::Value>,
    ) -> Option<mir::Value> {
        self.builder.call(
            callee.target.clone(),
            callee.signature,
            values,
            callee.result,
        )
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

    /// Call the function one key selects over an optional receiver and the recorded arguments.
    pub(in crate::lower) fn lower_direct_call(
        &mut self,
        receiver: Option<mir::Value>,
        key: &dir::InstanceKey,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let mut callee = self.resolve_callee(key)?;
        self.instantiate_symbol_callee(&mut callee, key.symbol, call)?;
        let parameters = callee.parameters.clone();

        // pass the receiver at its declared slot ahead of the arguments
        let mut values = Vec::with_capacity(parameters.len());
        let parameters = match receiver {
            Some(receiver) => {
                values.push(receiver);
                let Some((_, parameters)) = parameters.split_first() else {
                    return Err(CompilerError::Internal {
                        message: "a receiver call without a declared receiver slot".to_string(),
                    });
                };

                parameters
            }
            None => parameters.as_slice(),
        };
        values.extend(self.lower_call_arguments(&call.arguments, parameters, &[])?);
        let result = self.call(&callee, values);
        self.mark_park(call)?;

        Ok(result)
    }

    /// Lower one method call through its candidate.
    fn lower_method_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<Option<mir::Value>> {
        let (receiver, is_optional) = self.member_call_receiver(expression)?;

        self.lower_function_target_call(receiver, resolution, function, None, is_optional)
    }

    /// Return the receiver expression one method call's member callee reads, with its optionality.
    fn member_call_receiver(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(dir::LocalNodeId<dir::Expression>, bool)> {
        let dir::Expression::Call { left: callee, .. } = *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "a method call outside a call expression".to_string(),
            });
        };
        let dir::Expression::Member {
            left: receiver,
            is_optional,
            ..
        } = *self.source().tree().get(callee)
        else {
            return Err(self.internal("a method call without a member callee"));
        };

        Ok((receiver, is_optional))
    }

    /// Lower one function target over one explicit receiver expression.
    pub(in crate::lower) fn lower_function_target_call(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
        write: Option<dir::LocalNodeId<dir::Expression>>,
        is_optional: bool,
    ) -> CompilerResult<Option<mir::Value>> {
        // require the receiver the selection named
        let adjusted = function
            .receiver
            .as_ref()
            .ok_or_else(|| CompilerError::Internal {
                message: "a method call without a selected receiver".to_string(),
            })?;

        // split the declared receiver slot off the value parameters
        let mut selected = self.resolve_callee(&function.key)?;
        self.instantiate_symbol_callee(&mut selected, function.key.symbol, resolution)?;
        let parameters = selected.parameters.clone();
        let Some((_, parameters)) = parameters.split_first() else {
            return Err(CompilerError::Internal {
                message: "a method call without a declared receiver slot".to_string(),
            });
        };

        // evaluate the receiver, then the arguments, then borrow the receiver at the call
        let (source, borrow, rest) =
            self.receiver_source(receiver, adjusted, is_optional, ReceiverUse::Value)?;
        let supplied = write.map(Argument::Expression);
        let arguments =
            self.lower_call_arguments(&resolution.arguments, parameters, supplied.as_slice())?;
        let receiver = self.finish_receiver(source, borrow, rest)?;
        let mut values = vec![receiver];
        values.extend(arguments);
        let result = self.call(&selected, values);
        self.mark_park(resolution)?;

        Ok(result)
    }

    /// Lower one selected call over an already lowered receiver value.
    pub(in crate::lower) fn lower_value_target_call(
        &mut self,
        receiver: mir::Value,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        match &call.target {
            // adjust the value as the selection recorded, then call the declared instance
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                let receiver = match &function.receiver {
                    Some(adjusted) => {
                        self.lower_receiver_adjustments(receiver, &adjusted.adjustments)?
                    }
                    None => receiver,
                };

                self.lower_direct_call(Some(receiver), &function.key, call)
            }
            // dispatch through the erased receiver's constraint slot
            dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => {
                let receiver =
                    self.lower_receiver_adjustments(receiver, &dispatch.receiver.adjustments)?;

                self.lower_dynamic_symbol_call(receiver, dispatch, *symbol, &call.arguments)
            }
            _ => Err(self.unsupported("a virtual value call")),
        }
    }

    /// Call one constraint slot on an adjusted erased receiver, the slot named by its symbol.
    pub(in crate::lower) fn lower_dynamic_symbol_call(
        &mut self,
        receiver: mir::Value,
        dispatch: &dir::DynamicDispatch,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::ArgumentBinding],
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(name) = self.lower.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a dispatched member without a name".to_string(),
            });
        };

        self.lower_dynamic_slot_call(receiver, name, dispatch, arguments)
    }

    /// Mark the call inserted last when the selected signature parks the current fiber.
    pub(in crate::lower) fn mark_park(&mut self, resolution: &dir::Call) -> CompilerResult<()> {
        if self.lower.signature_parks(resolution.callable_type)? {
            self.builder.mark_park();
        }

        Ok(())
    }

    /// Resolve one recorded selection to the callee its receiver names.
    pub(in crate::lower) fn resolve_callee(
        &mut self,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Callee> {
        // a selection without a receiver names its instance directly
        let Some(receiver) = selection.receiver else {
            return self.resolve_callee_of(selection.symbol, selection);
        };

        // an open receiver resolves a requirement through its witness at instantiation
        let flags = self
            .lower
            .types(receiver.module_id)?
            .get_type_flags(receiver.local_id);
        if flags.has_parameter() || flags.has_this() {
            return match self.lower.requirement_owner(selection.symbol)? {
                Some(_) => self.selected_witness(receiver, selection),
                None => self.resolve_callee_of(selection.symbol, selection),
            };
        }

        // require a witness for the closed receiver
        let lowered = self.lower_type(receiver)?;
        let lowered = mir::erase_regions(self.builder.tree_mut(), lowered);
        let Some(witnesses) = self.lower.lowered_witnesses.get(&lowered).cloned() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a call to '{}' at {receiver:?} without a witness",
                    self.lower.symbol_path(selection.symbol)?
                ),
            });
        };

        // read the implementer that witness names for the member
        let implementer = witnesses
            .iter()
            .flat_map(|(_, witness)| witness.functions.iter())
            .find(|function| function.member == selection.symbol)
            .map(|function| function.function.clone());
        let Some(mut implementer) = implementer else {
            // a requirement the witness names no function for is answered by representation
            let owner = self.lower.requirement_owner(selection.symbol)?;
            let mut answered = false;
            if let Some(owner) = owner {
                for (interface, _) in witnesses.iter() {
                    answered |= self.lower.ty(*interface)?.symbol() == Some(owner);
                }
            }
            return match answered {
                true => self.selected_witness(receiver, selection),
                false => self.resolve_callee_of(selection.symbol, selection),
            };
        };

        // bind the implementer's own parameters as the call binds the requirement's
        let own = self.lower.own_parameters(selection.symbol)?;
        let arguments: Vec<_> = own
            .iter()
            .filter_map(|parameter| {
                selection
                    .arguments
                    .iter()
                    .find(|binding| binding.parameter == *parameter)
                    .map(|binding| binding.argument)
            })
            .collect();
        for (parameter, argument) in self
            .lower
            .own_parameters(implementer.symbol)?
            .into_iter()
            .zip(arguments)
        {
            implementer
                .arguments
                .push(dir::GenericArgumentBinding::new(parameter, argument));
        }

        self.resolve_callee_of(implementer.symbol, &implementer)
    }

    /// Return the witness call of one interface member at an open receiver.
    fn selected_witness(
        &mut self,
        receiver: dir::GlobalTypeId,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Callee> {
        // name the interface the member belongs to and the member itself
        let symbol = selection.symbol;
        let Some(owner) = self.lower.requirement_owner(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a witness call outside an interface member".to_string(),
            });
        };

        // lower the member's signature and interface over the member's own template
        let chain = self.lower.symbol_scope(symbol)?;
        let declared = self.lower.symbol_type(symbol)?;
        let tree = self.builder.tree_mut();
        let (mut parameters, result) = self.lower.lower_signature(tree, declared, &chain)?;
        if !self.lower.is_static_member(symbol)? {
            let this = self.lower.declared_receiver_type(tree, declared, &chain)?;
            parameters.insert(0, this);
        }

        // close the template at the selection's arguments and the receiver
        let arguments = self.selection_arguments(receiver, selection, &chain)?;
        let tree = self.builder.tree_mut();
        let parameters: Vec<_> = parameters
            .into_iter()
            .map(|parameter| substitute_type(tree, parameter, &arguments))
            .collect();
        let result = substitute_type(tree, result, &arguments);
        let (receiver, interface) =
            self.lower_witness_types(owner, receiver, &chain, &arguments)?;
        let requirement = self
            .lower
            .template_function(self.builder.tree_mut(), selection.symbol)?;

        // build the signature the witness call takes under the requirement's region binders
        let lifetimes = chain.declarations(self.lower.strings);
        let signature = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::FunctionSignature {
                lifetimes,
                parameters: parameters
                    .iter()
                    .map(|parameter| mir::SignatureParameter::new(*parameter))
                    .collect(),
                result,
            });

        Ok(Callee {
            parameters,
            result,
            target: mir::Callee::Witness {
                receiver,
                interface,
                requirement,
            },
            signature: mir::TypeId::from(signature),
        })
    }

    /// Lower the receiver and the applied interface one requirement is answered through.
    pub(in crate::lower) fn lower_witness_types(
        &mut self,
        owner: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
        chain: &GenericScope,
        arguments: &[mir::GenericArgument],
    ) -> CompilerResult<(mir::TypeId, mir::TypeId)> {
        let interface = self.lower.symbol_type(owner)?;
        let tree = self.builder.tree_mut();
        let interface = self
            .lower
            .type_lowerer(tree, chain)
            .lower_nominal(interface)?
            .storage;
        let tree = self.builder.tree_mut();
        let interface = substitute_type(tree, mir::TypeId::from(interface), arguments);
        let receiver = self.lower_type(receiver)?;

        Ok((mir::TypeId::from(receiver), interface))
    }

    /// Return the argument bound to each parameter of one member's template, in index order.
    pub(in crate::lower) fn selection_arguments(
        &mut self,
        receiver: dir::GlobalTypeId,
        selection: &dir::InstanceKey,
        chain: &GenericScope,
    ) -> CompilerResult<Vec<mir::GenericArgument>> {
        // place each parameter's argument at its index, the receiver binding the receiver parameter
        let mut arguments = vec![None; chain.count() as usize];
        for (parameter, index) in chain.parameters.clone() {
            let is_receiver = chain.receiver == Some(index);
            let bound = selection
                .arguments
                .iter()
                .find(|binding| binding.parameter == parameter)
                .map(|binding| binding.argument)
                .or(is_receiver.then_some(receiver));
            let Some(argument) = bound else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a witness call to '{}' without a binding for one parameter",
                        self.lower.symbol_path(selection.symbol)?
                    ),
                });
            };
            arguments[index as usize] = Some(self.lower_generic_argument(argument)?);
        }

        // place each dependent's evaluated value, the owners' first as sema records them
        let mut values = self.lower.selection_dependents(selection)?.into_iter();
        for (_, index) in chain.dependents.clone() {
            let Some(value) = values.next() else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a witness call to '{}' without a value for one dependent",
                        self.lower.symbol_path(selection.symbol)?
                    ),
                });
            };
            arguments[index as usize] = Some(self.lower_generic_argument(value)?);
        }

        arguments
            .into_iter()
            .map(|argument| {
                argument.ok_or_else(|| CompilerError::Internal {
                    message: "a template parameter without its argument".to_string(),
                })
            })
            .collect()
    }

    /// Return the instance one symbol names at a selection's receiver and arguments.
    pub(in crate::lower) fn instance_of(
        &mut self,
        symbol: dir::GlobalSymbolId,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Instance> {
        // a key without a receiver or arguments names a declared function
        let bindings = self.lower.selection_bindings(selection)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(symbol, selection.receiver, &arguments)?;
        if key.receiver.is_none() && key.arguments.is_empty() {
            return self.function(&key).map(Instance::Declared);
        }

        // declare the instance the key names
        let tree = self.builder.tree_mut();
        self.lower.declare_instance(
            tree,
            &key,
            symbol,
            selection.receiver,
            &bindings,
            &self.scope,
        )
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
            Some(FunctionDeclaration::Failed) => {
                Err(self.internal("a call into an undeclared callable"))
            }
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

    /// Declare one callable first reached from a body, queueing the body this module defines.
    fn declare_callable(&mut self, key: &GenericInstanceKey) -> CompilerResult<()> {
        // leave every instantiated key to its own declaration
        if key.receiver.is_some() || !key.arguments.is_empty() {
            return Ok(());
        }
        let enclosing = self.scope.clone();
        let tree = self.builder.tree_mut();
        if let Some(definition) = self.lower.declare_callable(tree, key, &enclosing)? {
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
        // read the callee expression out of the call
        let (dir::Expression::Call { left, .. } | dir::Expression::New { left, .. }) =
            *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "a non-call expression in an indirect call".to_string(),
            });
        };

        // take the callable at the receiver mode its type declares, a borrowed one from its place
        let callable = self.node_type_id(left)?;
        let declared = self.lower_type(callable)?;
        let borrowed = match self.callable_receiver_access(left)? {
            Some(access) => Some((self.borrowed_place(left)?, access)),
            None => None,
        };
        let callee = match borrowed {
            Some(_) => None,
            None => {
                let operand = self.lower_operand(left)?;
                let ty = self.node_type_id(left)?;

                Some(self.lower_anchored(expression, |lower| lower.as_value(operand, ty))?)
            }
        };

        // call through the value's declared signature
        let represented = self.resolved_type(declared);
        let signature = match self.builder.tree().get(represented) {
            mir::Type::Function { signature, .. } | mir::Type::FunctionPointer { signature } => {
                *signature
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "a call through a non-callable value".to_string(),
                });
            }
        };
        // instantiate the value's binders at the regions the call binds
        let scope = match self.lower.callable_template(callable)? {
            Some(template) => GenericScope::for_signature(self.lower, Some(template), None)?,
            None => GenericScope::default(),
        };
        let mut parameters = self.signature_parameters(mir::TypeId::from(signature))?;
        let mut result = self.builder.signature_result(mir::TypeId::from(signature));
        self.instantiate_signature(
            &mut parameters,
            &mut result,
            &scope,
            &resolution.regions,
            None,
        )?;
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, &[])?;

        // borrow the callable after its arguments, for the call alone
        let callee = match (callee, borrowed) {
            (Some(callee), _) => callee,
            (None, Some((place, access))) => {
                let target = self.borrowed_callable_type(declared, access);

                self.borrow_place(&place, target, mir::AddressKind::Borrow)?
            }
            (None, None) => unreachable!("a callable is a value or a borrowed place"),
        };

        Ok(self.builder.call(
            mir::Callee::Indirect { value: callee },
            signature,
            values,
            result,
        ))
    }

    /// Return the access a call borrows one callable expression at, absent for a consumed one.
    fn callable_receiver_access(
        &mut self,
        callee: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<mir::Access>> {
        // read the callable type beneath its memory forms
        let mut ty = self.node_type_id(callee)?;
        while let dir::Type::Form(form) = self.lower.ty(ty)? {
            ty = form.value;
        }
        let dir::Type::Function(function) = self.lower.ty(ty)? else {
            return Ok(None);
        };

        // read the receiver mode the callable declares
        let mode = match self.lower.ty(function.receiver)? {
            dir::Type::Literal(dir::Literal::String(text)) => dir::ReceiverMode::from_text(text),
            _ => None,
        };

        Ok(match mode {
            Some(dir::ReceiverMode::Borrowed(access)) => Some(ModuleLowerer::mir_access(access)),
            Some(dir::ReceiverMode::Owned) | None => None,
        })
    }

    /// Return one callable type re-qualified as a borrow at one access.
    fn borrowed_callable_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        access: mir::Access,
    ) -> mir::LocalNodeId<mir::Type> {
        // only a function type declares a receiver mode
        let mut borrowed = self.builder.tree().get(ty).clone();
        let mir::Type::Function {
            kind,
            lifetime,
            access: declared,
            ..
        } = &mut borrowed
        else {
            return ty;
        };

        // re-qualify it as a borrow for the call alone, a handle borrowed while the call holds it
        if *kind != mir::ReferenceKind::Borrowed {
            *lifetime = match kind {
                mir::ReferenceKind::Managed => mir::Lifetime::managed(),
                _ => mir::Lifetime::frame(),
            };
        }
        *kind = mir::ReferenceKind::Borrowed;
        *declared = access;

        self.builder.tree_mut().intern_type(borrowed)
    }

    /// Home the incoming receiver in a local.
    pub(in crate::lower) fn bind_receiver(&mut self, value: mir::Value) -> CompilerResult<Binding> {
        Ok(Binding::Local(self.home(value)))
    }

    /// Lower one receiver expression through its selected adjustments.
    pub(in crate::lower) fn lower_adjusted_receiver(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        adjusted: &dir::AdjustedReceiver,
        is_optional: bool,
        use_: ReceiverUse,
    ) -> CompilerResult<mir::Value> {
        let (source, borrow, rest) = self.receiver_source(receiver, adjusted, is_optional, use_)?;

        self.finish_receiver(source, borrow, rest)
    }

    /// Read one receiver expression, a member access on this reading the receiver's storage
    /// itself and a value use reading the constructed object.
    fn receiver_value(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        use_: ReceiverUse,
    ) -> CompilerResult<mir::Value> {
        match (use_, self.source().tree().get(receiver)) {
            (ReceiverUse::Storage, dir::Expression::This) => self.lower_this_storage(receiver),
            _ => self.lower_value(receiver),
        }
    }

    /// Return whether the enclosing receiver binding holds a constructor's uninitialized storage.
    pub(in crate::lower) fn this_fills_uninitialized_storage(&self) -> bool {
        let Some(binding) = self.this else {
            return false;
        };
        let held = self.binding_representation(binding);
        let tree = self.builder.tree();
        match tree.get(held) {
            mir::Type::Reference { pointee, .. } => {
                matches!(tree.get(*pointee), mir::Type::Uninit { .. })
            }
            _ => false,
        }
    }

    /// Read the storage the enclosing receiver binding holds, narrowed at one this expression.
    pub(in crate::lower) fn lower_this_storage(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let Some(binding) = self.this else {
            return Err(CompilerError::Internal {
                message: "a this outside a method body".to_string(),
            });
        };
        let value = self.read_binding(binding);

        self.lower_narrowing(expression, value)
    }

    /// Evaluate one receiver expression up to the borrow its adjustments lead with, the place
    /// left to borrow at the reference type the borrow produces.
    fn receiver_source<'adjust>(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        adjusted: &'adjust dir::AdjustedReceiver,
        is_optional: bool,
        use_: ReceiverUse,
    ) -> CompilerResult<(
        Operand,
        Option<mir::LocalNodeId<mir::Type>>,
        &'adjust [dir::ReceiverAdjustment],
    )> {
        // guard the receiver when an optional chain encloses it
        let is_guarded = is_optional && !self.chains.is_empty();

        // take the place behind a view strip over a value
        let mut adjustments = adjusted.adjustments.as_slice();
        while let [dir::ReceiverAdjustment::Dereference(dereference), rest @ ..] = adjustments
            && self.is_view_strip(dereference)?
        {
            adjustments = rest;
        }

        // lower the receiver at the adjustment leading its chain
        match adjustments {
            // address the receiver place when a borrow leads the adjustments
            [dir::ReceiverAdjustment::Borrow { ty }, rest @ ..] if !is_guarded => {
                let target = self.lower_type(*ty)?;
                let place = match (use_, self.source().tree().get(receiver)) {
                    (ReceiverUse::Storage, dir::Expression::This) => {
                        let storage = self.lower_this_storage(receiver)?;

                        self.place_behind(storage)?
                    }
                    _ => self.borrowed_place(receiver)?,
                };

                Ok((Operand::Place(place), Some(target), rest))
            }
            // a borrow behind a reference dereference reborrows the reference itself
            [
                dir::ReceiverAdjustment::Dereference(dereference),
                dir::ReceiverAdjustment::Borrow { ty },
                rest @ ..,
            ] if !is_guarded && self.is_reference_dereference(dereference)? => {
                let target = self.lower_type(*ty)?;
                let value = self.receiver_value(receiver, use_)?;
                let place = self.place_behind(value)?;

                Ok((Operand::Place(place), Some(target), rest))
            }
            // otherwise lower the receiver as a value
            rest => {
                let value = self.receiver_value(receiver, use_)?;
                let value = match is_guarded {
                    true => self.lower_chain_guard(value)?,
                    false => value,
                };

                Ok((Operand::Value(value), None, rest))
            }
        }
    }

    /// Borrow one evaluated receiver at the call itself and apply its remaining adjustments.
    pub(in crate::lower) fn finish_receiver(
        &mut self,
        receiver: Operand,
        borrow: Option<mir::LocalNodeId<mir::Type>>,
        rest: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<mir::Value> {
        let value = match (receiver, borrow) {
            (Operand::Place(place), Some(target)) => {
                self.borrow_place(&place, target, mir::AddressKind::Borrow)?
            }
            (Operand::Value(value), None) => value,
            _ => {
                return Err(CompilerError::Internal {
                    message: "a receiver evaluated apart from its borrow".to_string(),
                });
            }
        };

        self.lower_receiver_adjustments(value, rest)
    }

    /// Apply receiver adjustments to one lowered value in order.
    pub(in crate::lower) fn lower_receiver_adjustments(
        &mut self,
        mut value: mir::Value,
        mut adjustments: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<mir::Value> {
        while let [adjustment, rest @ ..] = adjustments {
            adjustments = rest;

            // a borrow behind a reference dereference reborrows the reference itself
            if let dir::ReceiverAdjustment::Dereference(dereference) = adjustment
                && let [dir::ReceiverAdjustment::Borrow { ty }, rest @ ..] = rest
                && self.is_reference_dereference(dereference)?
            {
                let target = self.lower_type(*ty)?;
                value = self.builder.cast(mir::CastOperator::Bitcast, value, target);
                adjustments = rest;
                continue;
            }

            // apply the adjustment the selection recorded
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

                            self.builder
                                .field_addr(value, 0, target, mir::AddressKind::Projection)
                        }
                        _ => self.builder.field_get(value, 0),
                    }
                }
                // project a narrowed union value or stored union place
                dir::ReceiverAdjustment::UnionPayload { union, arm, .. } => {
                    let index = self.case(*union, *arm)?;

                    // retain the address form of stored tagged receivers
                    let value_type = self.value_representation(value)?;
                    match self.builder.tree().get(value_type).clone() {
                        mir::Type::Reference { pointee, .. }
                        | mir::Type::Pointer { pointee, .. }
                            if matches!(
                                self.builder.tree().get(pointee),
                                mir::Type::Variant { .. }
                            ) =>
                        {
                            let target = self.lower_type(adjustment.ty())?;

                            self.builder.variant_payload_addr(
                                value,
                                index,
                                target,
                                mir::AddressKind::Projection,
                            )
                        }
                        // extract tagged payloads by their case
                        mir::Type::Variant { .. } => self.builder.variant_payload(value, index),
                        // niched references narrow to their arm in place
                        _ => {
                            let target = self.lower_type(adjustment.ty())?;

                            self.builder.cast(mir::CastOperator::Bitcast, value, target)
                        }
                    }
                }
            };
        }

        Ok(value)
    }

    /// Borrow one value through spilled local storage.
    pub(in crate::lower) fn spill_borrow(
        &mut self,
        value: mir::Value,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let ty = self.value_representation(value)?;
        let local = self.builder.local(ty, mir::Mutability::Immutable);
        self.builder.local_set(local, value);

        Ok(self
            .builder
            .local_addr(local, target, mir::AddressKind::Borrow))
    }

    /// Return whether a direct dereference strips a view over a value, keeping its representation.
    pub(in crate::lower) fn is_view_strip(
        &mut self,
        dereference: &dir::Dereference,
    ) -> CompilerResult<bool> {
        let is_direct = matches!(dereference.target, dir::DereferenceTarget::Direct);

        Ok(is_direct
            && self
                .lower
                .indirection(dereference.receiver, &self.scope)?
                .is_none())
    }

    /// Return whether one dereference reads through a physical reference.
    fn is_reference_dereference(&mut self, dereference: &dir::Dereference) -> CompilerResult<bool> {
        let is_direct = matches!(dereference.target, dir::DereferenceTarget::Direct);

        Ok(is_direct
            && self
                .lower
                .indirection(dereference.receiver, &self.scope)?
                .is_some())
    }

    /// Dereference one receiver value through its selected target.
    fn lower_dereference(
        &mut self,
        value: mir::Value,
        dereference: &dir::Dereference,
    ) -> CompilerResult<mir::Value> {
        match &dereference.target {
            // keep the value a view over it reads
            dir::DereferenceTarget::Direct if self.is_view_strip(dereference)? => Ok(value),
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
                let mut target = self.resolve_callee(&function.key)?;
                self.instantiate_symbol_callee(&mut target, function.key.symbol, call)?;
                let result = self.call(&target, vec![value]);

                result.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a protocol dereference".to_string(),
                })
            }
        }
    }
}
