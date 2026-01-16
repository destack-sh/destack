use destack_base::StringId;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::table::interface::InterfaceSlot;

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
        let (function_id, receiver_value) = match static_target {
            // static resolution: use target_symbol from candidate
            Some(target_symbol) => {
                // check if this is a method call (left is Member)
                let left_expr = self.env.dir_tree.get(*left);
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

                (function_id, receiver_value)
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
                        (function_id, None)
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

        // build arguments: receiver (if method call) + declared arguments
        let mut arguments =
            Vec::with_capacity(dynamic_arguments.len() + receiver_value.is_some() as usize);
        if let Some(receiver) = receiver_value {
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

        // emit call with metadata when we have a static resolution
        let value = if let (Some(receiver_type_id), Some(receiver_value), Some(target_symbol)) =
            (resolution_receiver, receiver_value, static_target)
        {
            // resolve interface dispatch metadata when needed
            let interface_target = self.interface_dispatch_target(
                expression_id,
                receiver_type_id,
                receiver_value,
                target_symbol,
                result_type,
            )?;

            if let Some((function_id, metadata)) = interface_target {
                self.state
                    .builder
                    .call_with_metadata(function_id, arguments, metadata)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "call returned no value".to_string(),
                    })?
            } else {
                // emit direct call metadata
                let signature = result_type;
                let metadata = mir::CallMetadata::direct(function_id, signature);
                self.state
                    .builder
                    .call_with_metadata(function_id, arguments, metadata)
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
                .call(function_id, arguments)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "call returned no value".to_string(),
                })?
        };

        Ok((value, result_type))
    }

    /// Build interface dispatch metadata when applicable.
    pub(crate) fn interface_dispatch_target(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
        receiver_value: mir::Value,
        target_symbol: GlobalSymbolId,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<Option<(mir::LocalNodeId<mir::Function>, mir::CallMetadata)>> {
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

        // build interface call metadata
        let signature = result_type;
        let metadata = mir::CallMetadata::interface_call(
            receiver_value,
            declaring_type,
            slot_id,
            signature,
            Some(function_id),
        );

        Ok(Some((function_id, metadata)))
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

        // resolve the member node
        let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "member symbol is not a method".to_string(),
            });
        };

        // load the member declaration
        let member = self.env.dir_tree.get(member_id);
        let dir::Member::Method { key, .. } = member else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "member symbol is not a method".to_string(),
            });
        };

        // resolve the method name
        let method_name = match key {
            Some(dir::DynamicKey::Name(name)) => *name,
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
        let node_id = member_id.into_global_any(self.env.module_id);
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
    fn interface_symbol_for_type(
        &self,
        receiver_type_id: dir::LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // resolve the receiver type
        let dir_type = self.env.types.get_type(receiver_type_id);
        match dir_type {
            dir::Type::Reference { symbol, .. } if symbol.ty() == dir::SymbolType::Interface => {
                Some(*symbol)
            }
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
}
