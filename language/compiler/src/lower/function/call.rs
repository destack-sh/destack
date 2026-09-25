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

/// The use one lowered receiver serves.
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

                // route intrinsic callables before declared ones, a method form over its receiver
                if let Some(CallableImplementation::Intrinsic { name }) =
                    self.lower.callable_implementation(function.key.symbol)?
                {
                    let receiver = match self.lower.callable_header(function.key.symbol)?.receiver {
                        Receiver::None => None,
                        Receiver::This(_) | Receiver::Erased | Receiver::Constructs(_) => {
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

        self.callee_of(instance)
    }

    /// Return one selection with the type definition lifetimes its call binds among its arguments.
    pub(in crate::lower) fn with_type_lifetimes(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: &dir::InstanceKey,
        regions: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<dir::InstanceKey> {
        let mut key = key.clone();
        let scope = self.lower.symbol_scope(symbol)?;
        for binding in regions {
            if scope.parameters.contains_key(&binding.parameter)
                && self
                    .lower
                    .is_declaration_region_parameter(binding.parameter)?
                && !key
                    .arguments
                    .iter()
                    .any(|bound| bound.parameter == binding.parameter)
            {
                key.arguments.push(*binding);
            }
        }

        Ok(key)
    }

    /// Return the callee of one instance, an applied template calling at its arguments.
    pub(in crate::lower) fn callee_of(&mut self, instance: Instance) -> CompilerResult<Callee> {
        let (function, arguments) = match instance {
            Instance::Declared(function) => (function, Vec::new()),
            Instance::Applied {
                template,
                arguments,
            } => (template, arguments),
        };
        let (signature, parameters, result) = self.function_signature(function, &arguments)?;

        Ok(Callee {
            target: mir::Callee::Direct {
                function,
                arguments,
            },
            signature,
            parameters,
            result,
        })
    }

    /// Instantiate one signature's late-bound regions.
    pub(in crate::lower) fn instantiate_signature(
        &mut self,
        parameters: &mut [mir::TypeId],
        result: &mut mir::TypeId,
        scope: &GenericScope,
        bindings: &[dir::GenericArgumentBinding],
        positions: &[(dir::GlobalGenericParameterId, u32)],
        receiver: Option<mir::Lifetime>,
    ) -> CompilerResult<()> {
        // lower the region bound at each slot in the caller's scope and at each erased position
        let slots = &scope.slots;
        let count = positions
            .iter()
            .map(|(_, position)| *position as usize + 1)
            .max()
            .unwrap_or(0)
            .max(scope.names.len());
        let mut regions = vec![None; count];
        if regions.is_empty() {
            return Ok(());
        }

        // bind the receiver's region at the slots its borrow names
        if let Some(receiver) = receiver {
            let first = parameters
                .first()
                .map(|first| self.builder.tree().get(*first));
            let Some(mir::Type::Reference { lifetime, .. }) = first else {
                return Err(CompilerError::Internal {
                    message: "a receiver region bound outside a leading reference".to_string(),
                });
            };
            let lifetime = lifetime.clone();
            for index in lifetime.bound_indices() {
                regions[index as usize] = Some(receiver.clone());
            }
        }
        for binding in bindings {
            let name = self.lower.format_parameter_name(binding.parameter)?;
            let position = positions
                .iter()
                .find(|(parameter, _)| *parameter == binding.parameter)
                .map(|(_, position)| *position);
            let Some(index) = slots
                .get(&binding.parameter)
                .map(|slot| slot.index)
                .or(position)
            else {
                // skip a binding the signature's binder leaves out, the slot check below stays loud
                continue;
            };
            let tree = self.builder.tree_mut();
            let region = self
                .lower
                .type_lowerer(tree, &self.scope)
                .lower_region_argument(binding.argument)
                .map_err(|error| match error {
                    CompilerError::Internal { message } => CompilerError::Internal {
                        message: format!("{message} bound at '{name}'"),
                    },
                    error => error,
                })?;
            let mir::GenericArgument::Region(lifetime) = region else {
                return Err(CompilerError::Internal {
                    message: format!("a region bound at '{name}' outside a region argument"),
                });
            };
            regions[index as usize] = Some(lifetime);
        }
        let mut instantiation = Vec::with_capacity(regions.len());
        for (slot, region) in regions.into_iter().enumerate() {
            let Some(region) = region else {
                let parameter = slots.get_index(slot).map(|(parameter, _)| *parameter);
                let name = match parameter {
                    Some(parameter) => self.lower.format_parameter_name(parameter)?,
                    None => "?".to_string(),
                };
                return Err(CompilerError::Internal {
                    message: format!("a region slot {slot} ({name}) unbound at a call"),
                });
            };
            instantiation.push(region);
        }

        // instantiate the signature's parameters and result
        let tree = self.builder.tree_mut();
        for parameter in parameters {
            *parameter = mir::instantiate_regions(tree, *parameter, &instantiation);
        }
        *result = mir::instantiate_regions(tree, *result, &instantiation);

        Ok(())
    }

    /// Instantiate one symbol callee at the regions a call binds.
    pub(in crate::lower) fn instantiate_symbol_callee(
        &mut self,
        callee: &mut Callee,
        selection: &dir::InstanceKey,
        call: &dir::Call,
    ) -> CompilerResult<()> {
        let symbol = selection.symbol;
        let scope = self.lower.symbol_scope(symbol)?;
        let path = self.lower.symbol_path(symbol)?;
        let positions = self.erased_region_positions(selection)?;
        let Callee {
            parameters,
            result,
            signature,
            ..
        } = callee;
        self.instantiate_signature(parameters, result, &scope, &call.regions, &positions, None)
            .map_err(|error| match error {
                CompilerError::Internal { message } => CompilerError::Internal {
                    message: format!("{message} calling '{path}'"),
                },
                error => error,
            })?;

        // intern the signature the call binds at its instantiated regions
        let parameters = parameters
            .iter()
            .map(|ty| mir::SignatureParameter::new(*ty))
            .collect();
        *signature = self
            .builder
            .tree_mut()
            .intern_type(mir::Type::FunctionSignature {
                lifetimes: Vec::new(),
                parameters,
                result: *result,
            });

        Ok(())
    }

    /// Return the position each region parameter of one selection's instance erased to.
    pub(in crate::lower) fn erased_region_positions(
        &mut self,
        selection: &dir::InstanceKey,
    ) -> CompilerResult<Vec<(dir::GlobalGenericParameterId, u32)>> {
        let mut positions = Vec::new();
        for binding in self.lower.selection_bindings(selection)? {
            let extent = match self.lower.ty(binding.argument)? {
                dir::Type::Region(pair) => pair.extent,
                _ => binding.argument,
            };
            let Some(text) = self.lower.memory_text(extent)? else {
                continue;
            };
            if let Some(dir::Lifetime::Bound(position)) =
                dir::Lifetime::parse(self.lower.strings.get(text))
            {
                positions.push((binding.parameter, position));
            }
        }

        Ok(positions)
    }

    /// Return the signature one function takes at generic arguments.
    fn function_signature(
        &mut self,
        function: mir::FunctionId,
        arguments: &[mir::GenericArgument],
    ) -> CompilerResult<(mir::TypeId, Vec<mir::TypeId>, mir::TypeId)> {
        let tree = self.builder.tree_mut();
        let declared = tree.get(function).clone();

        // require one argument per template parameter when applying a template
        if !arguments.is_empty() && arguments.len() != declared.generics.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "a call to '{}' with {} arguments for {} slots",
                    self.lower.strings.get(declared.name),
                    arguments.len(),
                    declared.generics.len()
                ),
            });
        }

        // substitute the arguments under the signature's region binder
        let signature = tree.intern_type(mir::Type::FunctionSignature {
            lifetimes: declared.lifetimes,
            parameters: declared
                .parameters
                .iter()
                .map(|parameter| mir::SignatureParameter::new(parameter.ty))
                .collect(),
            result: declared.return_type,
        });
        let signature = substitute_type(tree, signature, arguments);
        let parameters = self.signature_parameters(signature)?;
        let result = self.builder.signature_result(signature);

        Ok((signature, parameters, result))
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
        let key = self.with_type_lifetimes(key.symbol, key, &call.regions)?;
        let mut callee = self.resolve_callee(&key)?;
        self.instantiate_symbol_callee(&mut callee, &key, call)?;
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

        // split the declared receiver off the value parameters
        let key =
            self.with_type_lifetimes(function.key.symbol, &function.key, &resolution.regions)?;
        let mut selected = self.resolve_callee(&key)?;
        self.instantiate_symbol_callee(&mut selected, &key, resolution)?;
        let parameters = selected.parameters.clone();
        let Some((_, parameters)) = parameters.split_first() else {
            return Err(CompilerError::Internal {
                message: "a method call without a declared receiver".to_string(),
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

    /// Lower one selected call over a receiver value or place.
    pub(in crate::lower) fn lower_target_call(
        &mut self,
        receiver: Operand,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        match &call.target {
            // adjust the value as the selection recorded, then call the declared instance
            dir::CallableTarget::Symbol {
                function,
                dispatch: dir::FunctionDispatch::Direct,
            } => {
                let receiver = match &function.receiver {
                    Some(adjusted) => self.adjust_receiver(receiver, adjusted)?,
                    None => match receiver {
                        Operand::Value(value) => value,
                        _ => {
                            return Err(self.internal("a receiver place without its adjustments"));
                        }
                    },
                };

                self.lower_direct_call(Some(receiver), &function.key, call)
            }
            // dispatch through the erased receiver's constraint slot
            dir::CallableTarget::Dynamic {
                dispatch,
                function: dir::DynamicFunction::Symbol(symbol),
                ..
            } => {
                let receiver = self.adjust_receiver(receiver, &dispatch.receiver)?;

                self.lower_dynamic_symbol_call(receiver, dispatch, *symbol, call)
            }
            _ => Err(self.unsupported("a virtual value call")),
        }
    }

    /// Apply receiver adjustments to a value or its storage.
    pub(in crate::lower) fn adjust_receiver(
        &mut self,
        receiver: Operand,
        adjusted: &dir::AdjustedReceiver,
    ) -> CompilerResult<mir::Value> {
        let value = match receiver {
            Operand::Value(value) => value,
            Operand::Place(place) => {
                // project storage up to the selected borrow
                if let Some((index, ty)) =
                    adjusted
                        .adjustments
                        .iter()
                        .enumerate()
                        .find_map(|(index, step)| match step {
                            dir::ReceiverAdjustment::Borrow { ty } => Some((index, *ty)),
                            _ => None,
                        })
                {
                    let place =
                        self.project_place_adjustments(place, &adjusted.adjustments[..index])?;
                    let target = self.lower_type(ty)?;

                    return self.finish_receiver(
                        Operand::Place(place),
                        Some(target),
                        &adjusted.adjustments[index + 1..],
                    );
                }
                self.read_place(&place)?
            }
            Operand::Constant(_) => {
                return Err(self.internal("an unmaterialized constant receiver"));
            }
        };

        self.lower_receiver_adjustments(value, &adjusted.adjustments)
    }

    /// Call one constraint slot on an adjusted erased receiver, the slot named by its symbol.
    pub(in crate::lower) fn lower_dynamic_symbol_call(
        &mut self,
        receiver: mir::Value,
        dispatch: &dir::DynamicDispatch,
        symbol: dir::GlobalSymbolId,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        let Some(name) = self.lower.symbol_name(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a dispatched member without a name".to_string(),
            });
        };

        self.lower_dynamic_slot_call(receiver, name, dispatch, call)
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

        // resolve a member outside an interface at the receiver directly
        if self.lower.requirement_owner(selection.symbol)?.is_none() {
            return self.resolve_callee_of(selection.symbol, selection);
        }

        // dispatch every other receiver through its witness at instantiation
        self.selected_witness(receiver, selection)
    }

    /// Return the witness call of one interface member at its receiver.
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

        // close the template at the selection's arguments and the receiver, under the binders
        let arguments = self.selection_arguments(receiver, selection, &chain)?;
        let signature = substitute_type(self.builder.tree_mut(), signature, &arguments);
        let parameters = self.signature_parameters(signature)?;
        let result = self.builder.signature_result(signature);
        let (receiver, interface) =
            self.lower_witness_types(owner, receiver, &chain, &arguments)?;
        let requirement =
            self.lower
                .template_function(self.builder.tree_mut(), selection.symbol, None)?;

        // pass the requirement's arguments, filling the implementer's open places
        let requirement_arguments = (chain.owner_count..chain.count())
            .map(|index| arguments[index as usize].clone())
            .collect();

        Ok(Callee {
            parameters,
            result,
            target: mir::Callee::Witness {
                receiver,
                interface,
                requirement,
                arguments: requirement_arguments,
            },
            signature,
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
        let interface = substitute_type(tree, interface, arguments);
        let receiver = self.lower_type(receiver)?;

        // construct the dispatch key independently of signature lifetimes
        let tree = self.builder.tree_mut();
        let receiver = mir::erase_lifetimes(tree, receiver);
        let interface = mir::erase_lifetimes(tree, interface);

        Ok((receiver, interface))
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

        // place each dependent's evaluated value, the owners' first
        let mut values = self.lower.selection_dependents(selection)?.into_iter();
        for dependent in chain.dependents.clone().into_values() {
            let Some(value) = values.next() else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a witness call to '{}' without a value for one dependent",
                        self.lower.symbol_path(selection.symbol)?
                    ),
                });
            };
            arguments[dependent.index as usize] = Some(self.lower_generic_argument(value)?);
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
        // call a symbol without template parameters as its declared function
        let bindings = self.lower.selection_bindings(selection)?;
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();
        let key = self.generic_instance_key(symbol, selection.receiver, &arguments)?;
        let slots = self.lower.symbol_scope(symbol)?.count();
        if slots == 0 {
            return self.function(&key).map(Instance::Declared);
        }
        if key.receiver.is_none() && key.arguments.is_empty() {
            return Err(CompilerError::Internal {
                message: format!(
                    "a call to '{}' without arguments for its {slots} slots",
                    self.lower.symbol_path(symbol)?
                ),
            });
        }

        // declare the instance the key names
        let dependents = self.lower.selection_dependents(selection)?;
        let tree = self.builder.tree_mut();
        self.lower.declare_instance(
            tree,
            &key,
            symbol,
            selection.receiver,
            &bindings,
            &dependents,
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
            // report a call into a callable that failed to declare
            Some(FunctionDeclaration::Failed) => {
                Err(self.unsupported("a call into a callable that failed to declare"))
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
        let borrowed = match self.callable_borrow(left)? {
            Some(access) => {
                let target = self.borrowed_callable_type(declared, access);

                Some((self.borrowed_place(left, target)?, target))
            }
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
        let mut parameters = self.signature_parameters(signature)?;
        let mut result = self.builder.signature_result(signature);
        self.instantiate_signature(
            &mut parameters,
            &mut result,
            &scope,
            &resolution.regions,
            &[],
            None,
        )?;
        let values = self.lower_call_arguments(&resolution.arguments, &parameters, &[])?;

        // borrow the callable after its arguments, for the call alone
        let callee = match (callee, borrowed) {
            (Some(callee), _) => callee,
            (None, Some((place, target))) => self.borrow_place(&place, target)?,
            (None, None) => unreachable!("a callable is a value or a borrowed place"),
        };

        Ok(self.builder.call(
            mir::Callee::Indirect { value: callee },
            signature,
            values,
            result,
        ))
    }

    /// Return the qualifiers used to borrow a callable, absent when consumed.
    fn callable_borrow(
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
            dir::Type::Literal(dir::Literal::String(text)) => {
                dir::ReceiverMode::from_text(self.lower.strings.get(text))
            }
            _ => None,
        };

        Ok(match mode {
            Some(dir::ReceiverMode::Borrowed { access }) => Some(ModuleLowerer::mir_access(access)),
            Some(dir::ReceiverMode::Owned) | None => None,
        })
    }

    /// Return a callable type with the requested borrow qualifiers.
    fn borrowed_callable_type(&mut self, ty: mir::TypeId, access: mir::Access) -> mir::TypeId {
        // re-qualify a function type, every other representation staying as it is
        let mut borrowed = self.builder.tree().type_definition(ty).clone();
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
        if !matches!(kind, mir::Reference::Borrowed) {
            *lifetime = match kind {
                mir::Reference::Managed(_) => mir::Lifetime::managed(),
                _ => mir::Lifetime::frame(),
            };
        }
        *kind = mir::Reference::Borrowed;
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

    /// Read one receiver expression at its member access or value use.
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
        let value = self.read_binding(binding)?;

        self.lower_narrowing(expression, value)
    }

    /// Evaluate one receiver expression up to the borrow its adjustments lead with.
    fn receiver_source<'adjust>(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        adjusted: &'adjust dir::AdjustedReceiver,
        is_optional: bool,
        use_: ReceiverUse,
    ) -> CompilerResult<(
        Operand,
        Option<mir::TypeId>,
        &'adjust [dir::ReceiverAdjustment],
    )> {
        // guard the receiver when an optional chain encloses it
        let is_guarded = is_optional && !self.chains.is_empty();

        // take the place behind a view over a value
        let mut adjustments = adjusted.adjustments.as_slice();
        while let [dir::ReceiverAdjustment::Dereference(dereference), rest @ ..] = adjustments
            && self.dereference_is_view(dereference)?
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
                    _ => self.borrowed_place(receiver, target)?,
                };

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
        borrow: Option<mir::TypeId>,
        rest: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<mir::Value> {
        let value = match (receiver, borrow) {
            (Operand::Place(place), Some(target)) => self.borrow_place(&place, target)?,
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

            // apply the adjustment the selection recorded
            value = match adjustment {
                // borrow the receiver value at the selected form
                dir::ReceiverAdjustment::Borrow { ty } => {
                    let target = self.lower_type(*ty)?;

                    self.borrow_value(value, target)?
                }
                // read through one reference or pointer receiver
                dir::ReceiverAdjustment::Dereference(dereference) => {
                    self.lower_dereference(value, dereference)?
                }
                // unwrap a newtype value or stored newtype place
                dir::ReceiverAdjustment::NewtypePayload { ty, .. } => {
                    let target = self.lower_type(*ty)?;
                    let stored = self.stored_projection(value, |definition| match definition {
                        mir::Type::Newtype { value, .. } => Some(*value),
                        _ => None,
                    })?;
                    match stored {
                        Some((stored, access)) => {
                            let projection = mir::Projection::Field { index: 0 };

                            self.stored_payload(value, projection, stored, access, target)?
                        }
                        None => self.builder.field_get(value, 0),
                    }
                }
                // project a narrowed union value or stored union place
                dir::ReceiverAdjustment::UnionPayload { union, arm, ty } => {
                    let members = self.lower.union_members(*union)?;
                    let index = self.case(&members, *arm)?;
                    let target = self.lower_type(*ty)?;
                    let stored = self.stored_projection(value, |definition| match definition {
                        mir::Type::Variant { cases, .. } => {
                            cases.get(index as usize).map(|case| case.ty)
                        }
                        _ => None,
                    })?;
                    match stored {
                        Some((payload, access)) => {
                            let projection = mir::Projection::Variant { case: index };

                            self.stored_payload(value, projection, payload, access, target)?
                        }
                        // extract tagged payloads by their case
                        None if matches!(
                            self.builder.tree().get(self.value_representation(value)?),
                            mir::Type::Variant { .. }
                        ) =>
                        {
                            self.builder.variant_payload(value, index)
                        }
                        // niched references narrow to their arm in place
                        None => self.builder.cast(mir::CastOperator::Bitcast, value, target),
                    }
                }
                // reinterpret the receiver at the base class declaring the member
                dir::ReceiverAdjustment::Upcast { ty } => {
                    let target = self.lower_type(*ty)?;

                    self.builder.cast(mir::CastOperator::Bitcast, value, target)
                }
            };
        }

        Ok(value)
    }

    /// Return the payload one stored receiver projects, with the access its storage exposes.
    fn stored_projection(
        &mut self,
        value: mir::Value,
        select: impl Fn(&mir::Type) -> Option<mir::TypeId>,
    ) -> CompilerResult<Option<(mir::TypeId, mir::Access)>> {
        let held = self.value_representation(value)?;
        let Some(access) = self.rooted_access(held) else {
            return Ok(None);
        };
        let pointee = self.reference_pointee(value)?;
        let pointee = self.resolved_type(pointee);

        Ok(select(self.builder.tree().type_definition(pointee)).map(|payload| (payload, access)))
    }

    /// Address one projected payload of a stored receiver, borrowing the object past a handle.
    fn stored_payload(
        &mut self,
        value: mir::Value,
        projection: mir::Projection,
        stored: mir::TypeId,
        access: mir::Access,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        // reinterpret a handle at the object newtype's leading payload
        if matches!(projection, mir::Projection::Field { index: 0 })
            && matches!(
                self.builder.tree().type_definition(target),
                mir::Type::Reference {
                    kind: mir::Reference::Managed(_),
                    ..
                }
            )
        {
            return Ok(self.builder.cast(mir::CastOperator::Bitcast, value, target));
        }
        let place = mir::Place::value(value)
            .with_projection(mir::Projection::Deref)
            .with_projection(projection);
        if self.addresses_value(target, stored) || self.rooted_access(stored).is_none() {
            return Ok(self.builder.address(place, target));
        }

        // borrow the object the stored handle names for as long as the receiver lives
        let lifetime = self.reborrow_lifetime(value);
        let slot = self.builder.tree_mut().intern_type(mir::Type::Reference {
            kind: mir::Reference::Borrowed,
            lifetime,
            access,
            pointee: stored,
        });
        let slot = self.builder.address(place, slot);
        let handle = self.dereference(slot)?;

        self.reborrow_or_reinterpret(handle, target)
    }

    /// Borrow one value at the target type: a fresh temporary, else the reference's referent.
    pub(in crate::lower) fn borrow_value(
        &mut self,
        value: mir::Value,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let held = self.value_representation(value)?;
        if !self.addresses_value(target, held) && self.rooted_access(held).is_some() {
            let place = self.place_behind(value)?;

            return self.borrow_place(&place, target);
        }

        self.borrow_temporary(value, target)
    }

    /// Store one value in a fresh frame local and borrow it at the target type.
    pub(in crate::lower) fn borrow_temporary(
        &mut self,
        value: mir::Value,
        target: mir::TypeId,
    ) -> CompilerResult<mir::Value> {
        let ty = self.value_representation(value)?;
        let local = self.builder.local(ty, mir::Mutability::Mutable);
        self.builder.local_set(local, value);

        Ok(self.builder.address(mir::Place::local(local), target))
    }

    /// Return whether one dereference strips a view over the value itself.
    pub(in crate::lower) fn dereference_is_view(
        &mut self,
        dereference: &dir::Dereference,
    ) -> CompilerResult<bool> {
        Ok(matches!(
            self.lower.ty(dereference.receiver)?,
            dir::Type::Form(form) if matches!(form.form, dir::Form::Readonly | dir::Form::Owned)
        ))
    }

    /// Return whether one built-in dereference yields a managed object.
    fn dereferences_object(&mut self, dereference: &dir::Dereference) -> CompilerResult<bool> {
        Ok(self.lower.ownership(dereference.ty)? == dir::Ownership::Managed)
    }

    /// Dereference one receiver value: a view reads through.
    fn lower_dereference(
        &mut self,
        value: mir::Value,
        dereference: &dir::Dereference,
    ) -> CompilerResult<mir::Value> {
        match &dereference.protocol {
            None if self.dereference_is_view(dereference)? => Ok(value),
            None if self.dereferences_object(dereference)? => {
                let ty = self.lower_type(dereference.ty)?;

                Ok(self.builder.cast(mir::CastOperator::Bitcast, value, ty))
            }
            None => {
                let ty = self.lower_type(dereference.ty)?;
                let place = mir::Place::value(value).with_projection(mir::Projection::Deref);

                Ok(self.load_place(place, ty))
            }
            Some(call) => {
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
                self.instantiate_symbol_callee(&mut target, &function.key, call)?;
                let result = self.call(&target, vec![value]);

                result.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a protocol dereference".to_string(),
                })
            }
        }
    }
}
