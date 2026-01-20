use destack_dir::{Expression, LocalNodeId, Resolution};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::emit::value::dispatch::DispatchTarget;

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

        // resolve call resolution (lower requires static resolution)
        let resolution =
            self.get_resolution(expression_id)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "call expression missing Resolution (Analyze issue)".to_string(),
                })?;
        let Resolution::Static {
            receiver: resolution_receiver,
            candidate,
        } = resolution
        else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "call resolution must be static before Lower".to_string(),
            });
        };
        let resolution_receiver = *resolution_receiver;
        let target_symbol = candidate.target_symbol;

        // resolve target function and receiver
        let is_static = self.is_static_method_symbol(target_symbol);
        let (function_id, receiver_value, receiver_type_id) = {
            // check if this is a method call (left is Member)
            let left_expr = self.env.dir_tree.get(*left);
            let mut receiver_type_id = if is_static { None } else { resolution_receiver };
            let receiver_value = if !is_static
                && resolution_receiver.is_some()
                && let Expression::Member {
                    left: receiver_id, ..
                } = left_expr
            {
                // skip evaluation for namespace receivers, they are compile-time only
                if self.receiver_is_namespace_reference(*receiver_id) {
                    None
                } else {
                    let (receiver_value, _) = self.lower_value_expression(*receiver_id)?;
                    receiver_type_id = self.type_for_expression(*receiver_id).or(receiver_type_id);
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

        // emit call when we have a static resolution
        let result_type = self.lower_type_for_expression(expression_id)?;
        let signature = self.signature_type_for_function(expression_id, function_id)?;
        let value = if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, dispatch_receiver)
        {
            // resolve dispatch target when the receiver supports it
            let dispatch_target = self.dispatch_target_for_symbol(
                expression_id,
                receiver_type_id,
                target_symbol,
                function_id,
            )?;
            if let Some(dispatch_target) = dispatch_target {
                match dispatch_target {
                    DispatchTarget::Interface {
                        declaring_type,
                        slot_id,
                        function_id,
                    } => {
                        let value = self.state.builder.call_interface(
                            receiver_value,
                            declaring_type,
                            slot_id,
                            Some(function_id),
                            signature,
                            arguments,
                        );
                        value.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "call returned no value".to_string(),
                        })?
                    }
                    DispatchTarget::Virtual {
                        declaring_type,
                        slot_id,
                        function_id,
                    } => {
                        let value = self.state.builder.call_virtual(
                            receiver_value,
                            declaring_type,
                            slot_id,
                            Some(function_id),
                            signature,
                            arguments,
                        );
                        value.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "call returned no value".to_string(),
                        })?
                    }
                }
            } else {
                let value = self.state.builder.call(function_id, signature, arguments);
                value.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "call returned no value".to_string(),
                })?
            }
        }
        // direct call when no receiver dispatch is needed
        else {
            let value = self.state.builder.call(function_id, signature, arguments);
            value.ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "call returned no value".to_string(),
            })?
        };

        Ok((value, result_type))
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
}
