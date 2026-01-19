use destack_base::StringId;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::table::interface::InterfaceSlot;

/// Interface dispatch target details for lowering.
pub(super) struct InterfaceDispatchTarget {
    /// The declared target function id for the interface method.
    pub(super) function_id: mir::LocalNodeId<mir::Function>,
    /// The declaring interface type id.
    pub(super) declaring_type: mir::LocalNodeId<mir::Type>,
    /// The itab slot id for the method.
    pub(super) slot_id: u32,
}

/// Interface receiver values for lowering calls.
pub(super) struct InterfaceCallReceivers {
    /// Receiver used for argument passing.
    pub(super) argument_receiver: mir::Value,
    /// Receiver used for interface dispatch.
    pub(super) dispatch_receiver: mir::Value,
}

impl FunctionContext<'_> {
    /// Lower a call expression to its result value and type.
    pub(crate) fn lower_call_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left: &LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
        static_arguments: &Option<Vec<LocalNodeId<dir::Argument>>>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        if static_arguments.is_some() {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "static arguments are not supported".to_string(),
            })?;
        }

        // check for resolution on the call expression
        // extract target symbol if static resolution (to avoid borrow issues)
        let resolution = self.get_resolution(expression_id);
        let static_target = match resolution {
            Some(Resolution::Static { candidate, .. }) => Some(candidate.target_symbol),
            _ => None,
        };
        let resolution_receiver = resolution.and_then(|resolution| resolution.receiver());

        // resolve target function and receiver based on resolution or syntax
        let (function_id, receiver_value, receiver_type_id) = match static_target {
            // static resolution: use target_symbol from candidate
            Some(target_symbol) => {
                // check if this is a method call (left is Member)
                let left_expr = self.env.dir_tree.get(*left);
                let mut receiver_type_id = resolution_receiver;
                let receiver_value = if resolution_receiver.is_some()
                    && let Expression::Member {
                        left: receiver_id,
                        static_arguments: member_static_args,
                        ..
                    } = left_expr
                {
                    if member_static_args.is_some() {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "static arguments on method calls are not supported"
                                .to_string(),
                        })?;
                    }
                    // skip evaluation for namespace receivers, they are compile-time only
                    if self.receiver_is_namespace_reference(*receiver_id) {
                        None
                    } else {
                        let (receiver_value, _) = self.lower_value_expression(*receiver_id)?;
                        receiver_type_id = self
                            .dir_type_for_expression(*receiver_id)
                            .or(receiver_type_id);
                        Some(receiver_value)
                    }
                } else {
                    None
                };

                // look up function by target symbol
                let function_id = *self
                    .env
                    .functions_by_symbol
                    .get(&target_symbol)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "missing function for resolved target symbol".to_string(),
                    })?;

                (function_id, receiver_value, receiver_type_id)
            }

            // no resolution: fall back to syntax-based dispatch for direct calls
            _ => {
                let left_expr = self.env.dir_tree.get(*left);
                match left_expr {
                    // method call requires Resolution from Analyze
                    Expression::Member { name, .. } => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: format!(
                                "method call '{}' missing Resolution (Analyze issue)",
                                self.env.strings.get(*name).as_str()
                            ),
                        })?;
                    }

                    // direct function call
                    Expression::LocalReference { target_symbol, .. }
                    | Expression::ModuleReference { target_symbol, .. }
                    | Expression::GlobalReference { target_symbol, .. } => {
                        let function_id = *self
                            .env
                            .functions_by_symbol
                            .get(target_symbol)
                            .ok_or_else(|| LowerError::UnsupportedConstruct {
                                node: expression_id
                                    .into_global_any(self.env.module_id)
                                    .into_anchored(Some(self.env.profile)),
                                message: "missing function symbol".to_string(),
                            })?;
                        (function_id, None, None)
                    }

                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "unsupported call expression target".to_string(),
                        })?;
                    }
                }
            }
        };

        // resolve interface receivers for dispatch and argument passing
        let receivers =
            if let (Some(receiver_type_id), Some(receiver)) = (receiver_type_id, receiver_value) {
                Some(self.interface_call_receivers(expression_id, receiver_type_id, receiver)?)
            } else {
                None
            };
        let (call_receiver, dispatch_receiver) = match receivers {
            Some(receivers) => (
                Some(receivers.argument_receiver),
                Some(receivers.dispatch_receiver),
            ),
            None => (receiver_value, receiver_value),
        };

        // build arguments: receiver (if method call) + declared arguments
        let mut arguments =
            Vec::with_capacity(dynamic_arguments.len() + call_receiver.is_some() as usize);
        if let Some(receiver) = call_receiver {
            arguments.push(receiver);
        }
        for argument_id in dynamic_arguments {
            let argument = self.env.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "unsupported non-positional argument".to_string(),
                })?;
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            arguments.push(value);
        }

        // get result type
        let result_type = self.mir_type_for_expression(expression_id)?;
        let signature = self.signature_type_for_function(expression_id, function_id)?;

        // emit call when we have a static resolution
        let value = if let (Some(receiver_type_id), Some(receiver_value), Some(target_symbol)) =
            (receiver_type_id, dispatch_receiver, static_target)
        {
            // resolve interface dispatch when needed
            let interface_target =
                self.interface_dispatch_target(expression_id, receiver_type_id, target_symbol)?;

            if let Some(interface_target) = interface_target {
                self.state
                    .builder
                    .call_interface(
                        receiver_value,
                        interface_target.declaring_type,
                        interface_target.slot_id,
                        Some(interface_target.function_id),
                        signature,
                        arguments,
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "call returned no value".to_string(),
                    })?
            }
            // emit virtual call
            else if let Some(slot_id) = self.virtual_method_slot_id(target_symbol)
                && self.class_symbol_for_type(receiver_type_id).is_some()
            {
                let declaring_type =
                    self.declaring_type_for_virtual_call(expression_id, receiver_type_id)?;
                self.state
                    .builder
                    .call_virtual(
                        receiver_value,
                        declaring_type,
                        slot_id,
                        Some(function_id),
                        signature,
                        arguments,
                    )
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "call returned no value".to_string(),
                    })?
            }
            // emit direct call
            else {
                self.state
                    .builder
                    .call(function_id, signature, arguments)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "call returned no value".to_string(),
                    })?
            }
        }
        // fall back to plain call
        else {
            self.state
                .builder
                .call(function_id, signature, arguments)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "call returned no value".to_string(),
                })?
        };

        Ok((value, result_type))
    }

    /// Build interface dispatch targets when applicable.
    pub(super) fn interface_dispatch_target(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<Option<InterfaceDispatchTarget>> {
        // resolve the interface symbol from the receiver type
        let Some(interface_symbol) = self.interface_symbol_for_type(receiver_type_id) else {
            return Ok(None);
        };

        // resolve method info for the target symbol
        let (method_name, signature_type_id) =
            self.method_key_for_symbol(expression_id, target_symbol)?;

        // resolve the interface slot id
        let slot_id = self.interface_method_slot_id(
            expression_id,
            interface_symbol,
            method_name,
            signature_type_id,
        )?;

        // resolve the declaring type
        let interface_type_id = self
            .env
            .types
            .get_instance_type_id(interface_symbol)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // resolve the declaring mir type
        let declaring_type = *self
            .env
            .type_lowerer
            .type_cache
            .get(&interface_type_id)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // resolve the target function id
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "missing function for resolved target symbol".to_string(),
            })?;

        Ok(Some(InterfaceDispatchTarget {
            function_id,
            declaring_type,
            slot_id,
        }))
    }

    /// Resolve a MIR signature type for a lowered function.
    pub(crate) fn signature_type_for_function(
        &self,
        expression_id: LocalNodeId<Expression>,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // lookup the signature type for this function
        let signature = self
            .env
            .function_signature_types
            .get(&function_id)
            .copied()
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        Ok(signature)
    }

    /// Resolve receiver values for interface call lowering.
    pub(super) fn interface_call_receivers(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
        receiver_value: mir::Value,
    ) -> LowerResult<InterfaceCallReceivers> {
        // skip non-interface receivers
        if self.interface_symbol_for_type(receiver_type_id).is_none() {
            return Ok(InterfaceCallReceivers {
                argument_receiver: receiver_value,
                dispatch_receiver: receiver_value,
            });
        }

        // resolve interface reference layout
        let layout = self
            .env
            .type_lowerer
            .interface_ref_layout(receiver_type_id)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        // extract the object pointer for argument passing
        let object_value = self
            .state
            .builder
            .field_get(receiver_value, layout.object_field_index);

        Ok(InterfaceCallReceivers {
            argument_receiver: object_value,
            dispatch_receiver: receiver_value,
        })
    }

    /// Resolve a member name and signature type for a symbol.
    fn method_key_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
    ) -> LowerResult<(StringId, dir::LocalTypeId)> {
        // require a local symbol for now
        if symbol.module_id != self.env.module_id {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface dispatch across modules is not supported".to_string(),
            });
        }

        // resolve the primary declaration node
        let symbol_entry = self.env.symbols.get_symbol(symbol.local_id);
        let Some(primary) = symbol_entry.primary_declaration else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "member symbol missing declaration".to_string(),
            });
        };

        // resolve the member or property node
        let (method_key, signature, node_id) =
            if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
                let member = self.env.dir_tree.get(member_id);
                let dir::Member::Method { key, signature, .. } = member else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "member symbol is not a method".to_string(),
                    });
                };
                (
                    key,
                    signature,
                    member_id.into_global_any(self.env.module_id),
                )
            } else if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
                let property = self.env.dir_tree.get(property_id);
                let dir::Property::Method { key, signature, .. } = property else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "property symbol is not a method".to_string(),
                    });
                };
                (
                    key,
                    signature,
                    property_id.into_global_any(self.env.module_id),
                )
            } else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "member symbol is not a method".to_string(),
                });
            };

        // resolve the method name
        let method_name = match (method_key, signature.mode) {
            (Some(dir::DynamicKey::Name(name)), _) => *name,
            (None, Some(dir::FunctionMode::Call)) => self.env.dispatch_call_name,
            (None, Some(dir::FunctionMode::Constructor | dir::FunctionMode::New)) => {
                self.env.dispatch_construct_name
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "method must have a static name".to_string(),
                });
            }
        };

        // resolve the signature type id
        let signature_type_id = self
            .env
            .types
            .get_signature_type_for_node(node_id)
            .ok_or_else(|| LowerError::MissingType {
                node: node_id.into_anchored(Some(self.env.profile)),
            })?;

        Ok((method_name, signature_type_id))
    }

    /// Resolve the interface symbol for a receiver type.
    pub(crate) fn interface_symbol_for_type(
        &self,
        receiver_type_id: dir::LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // resolve the receiver type
        let dir_type = self.env.types.get_type(receiver_type_id);
        match dir_type {
            dir::Type::Reference { symbol, .. } if symbol.ty() == dir::SymbolType::Interface => {
                Some(*symbol)
            }
            dir::Type::Value { value } => self.interface_symbol_for_type(*value),
            dir::Type::Intersection { elements } => elements
                .iter()
                .find_map(|element| self.interface_symbol_for_type(*element)),
            _ => None,
        }
    }

    /// Resolve the interface dispatch slot id for a method.
    fn interface_method_slot_id(
        &self,
        expression_id: LocalNodeId<Expression>,
        interface_symbol: GlobalSymbolId,
        method_name: StringId,
        signature_type_id: dir::LocalTypeId,
    ) -> LowerResult<u32> {
        // load interface slots for dispatch
        let slots = self
            .env
            .interface_dispatch
            .slots(interface_symbol)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface dispatch layout missing".to_string(),
            })?;

        // locate the matching method slot
        let slot_index = slots.iter().position(|slot| {
            let InterfaceSlot::Method {
                name, signature, ..
            } = slot
            else {
                return false;
            };
            *name == method_name
                && dir::are_types_equal(*signature, signature_type_id, self.env.types)
        });

        // require a matching slot
        let Some(slot_index) = slot_index else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface method slot missing".to_string(),
            });
        };

        Ok(slot_index as u32 + 1)
    }

    /// Resolve the vtable slot id for a virtual method symbol.
    pub(super) fn virtual_method_slot_id(&self, symbol: GlobalSymbolId) -> Option<u32> {
        self.env
            .virtual_method_slots_by_symbol
            .get(&symbol)
            .copied()
    }

    /// Resolve the declaring MIR type for a virtual call.
    pub(super) fn declaring_type_for_virtual_call(
        &self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let class_symbol = self
            .class_symbol_for_type(receiver_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "virtual dispatch requires a class receiver".to_string(),
            })?;

        let instance_type_id = self
            .env
            .types
            .get_instance_type_id(class_symbol)
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        let mir_type = self
            .env
            .type_lowerer
            .type_cache
            .get(&instance_type_id)
            .copied()
            .ok_or_else(|| LowerError::MissingType {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
            })?;

        Ok(mir_type)
    }
}
