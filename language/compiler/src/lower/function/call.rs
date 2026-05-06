use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::{DispatchTarget, FunctionLowerer, is_void_type, resolve_result_union};

/// Interface receiver values for lowering calls.
pub(super) struct InterfaceCallReceivers {
    /// Receiver used for argument passing.
    pub(super) argument_receiver: mir::Value,
    /// Receiver used for interface dispatch.
    pub(super) dispatch_receiver: mir::Value,
}

/// Kind of call to lower.
enum CallKind {
    /// Lower a call expression to its result value.
    Expression,
    /// Lower a call expression to its result statement.
    Statement,
}

impl FunctionLowerer<'_> {
    /// Lower a call expression to its result value and type.
    ///
    /// ```ds
    /// function add(a: int32, b: int32): int32 {
    ///     return a + b;
    /// }
    ///
    /// function use(): int32 {
    ///     return add(1, 2);
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: int32 = const 2
    /// v3: int32 = call add(v1, v2)
    /// ```
    pub(crate) fn lower_call_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: &dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let (value, result_type) = self.lower_call(
            expression_id,
            left,
            arguments,
            generic_arguments,
            CallKind::Expression,
        )?;
        let value = value.ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile)),
            message: "call returned no value".to_string(),
        })?;

        Ok((value, result_type))
    }

    /// Lower a call as a statement (no result value).
    pub(crate) fn lower_call_statement(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: &dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> LowerResult<()> {
        self.lower_call(
            expression_id,
            left,
            arguments,
            generic_arguments,
            CallKind::Statement,
        )?;

        Ok(())
    }

    /// Lower a call expression to its result value and type.
    fn lower_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: &dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        kind: CallKind,
    ) -> LowerResult<(Option<mir::Value>, mir::LocalNodeId<mir::Type>)> {
        // resolve call resolution (lower requires static resolution)
        let resolution =
            self.get_resolution(expression_id)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "call expression missing dir::Resolution (Analyze issue)".to_string(),
                })?;
        let dir::Resolution::Static {
            receiver: resolution_receiver,
            candidate,
        } = resolution
        else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "call resolution must be static before Lower".to_string(),
            });
        };
        let resolution_receiver = *resolution_receiver;
        let target_symbol = candidate.target_symbol;

        // lower intrinsic bindings directly
        if let Some(result) = self.lower_intrinsic_binding_call(
            expression_id,
            target_symbol,
            resolution_receiver,
            arguments,
        )? {
            return Ok(result);
        }

        // TODO #Incomplete: lower/monomorphize generic functions
        if !generic_arguments.is_empty() {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "generic arguments are only supported for intrinsic calls".to_string(),
            })?;
        }

        // lower closure calls when the resolution target is not a function symbol
        if target_symbol.ty() != dir::SymbolType::Function
            && self.function_for_symbol(target_symbol).is_none()
            && let Some(type_id) = self.type_for_expression(*left)
            && matches!(
                self.context.types.get_type(type_id),
                dir::Type::Function { .. }
            )
        {
            let (closure_value, closure_type) = self.lower_value_expression(*left)?;
            return self.lower_closure_call(
                expression_id,
                closure_value,
                closure_type,
                arguments,
                kind,
            );
        }

        // lower calls to captured functions via closure values
        let has_captures = self
            .context
            .captures
            .capture_set(target_symbol)
            .is_some_and(|set| !set.captures.is_empty());
        if self.context.symbols.get_symbol(target_symbol.local_id).ty == dir::SymbolType::Function
            && has_captures
        {
            let (closure_value, closure_type) = self.lower_value_expression(*left)?;
            return self.lower_closure_call(
                expression_id,
                closure_value,
                closure_type,
                arguments,
                kind,
            );
        }

        // resolve target function and receiver
        let is_static = self.is_static_method_symbol(target_symbol);
        let (function_id, receiver_value, receiver_type_id) = {
            // check if this is a method call (left is Member)
            let left_expr = self.context.dir_tree.get(*left);
            let mut receiver_type_id = if is_static { None } else { resolution_receiver };
            let receiver_value = if !is_static
                && resolution_receiver.is_some()
                && let dir::Expression::Member {
                    left: receiver_id, ..
                }
                | dir::Expression::PrivateMember {
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
            let function_id = self.function_for_symbol(target_symbol).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "missing function for resolved target symbol".to_string(),
                }
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
        let mut argument_values =
            Vec::with_capacity(arguments.len() + call_receiver.is_some() as usize);
        if let Some(receiver) = call_receiver {
            argument_values.push(receiver);
        }
        for argument_id in arguments {
            let argument = self.context.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported non-positional argument".to_string(),
                })?;
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            argument_values.push(value);
        }

        // emit call when we have a static resolution
        let result_type = self.lower_type_for_expression(expression_id)?;
        let returns_void = result_type == self.context.type_lowerer.ty_void;
        let signature = self.signature_type_for_function(expression_id, function_id)?;
        let is_binding_call = self.context.binding_abi_lowering
            && self.context.binding_symbols.contains(&target_symbol);
        if is_binding_call {
            if dispatch_receiver.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "binding calls cannot be method calls".to_string(),
                });
            }
            let (value, lowered_type) = self.lower_binding_call_expression(
                expression_id,
                function_id,
                signature,
                argument_values,
                result_type,
            )?;
            return Ok((Some(value), lowered_type));
        }

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
                        slot,
                        function_id,
                    } => {
                        if returns_void {
                            self.state.builder.call_interface_void(
                                receiver_value,
                                declaring_type,
                                slot,
                                Some(function_id),
                                signature,
                                argument_values,
                            );
                            None
                        } else {
                            self.state.builder.call_interface(
                                receiver_value,
                                declaring_type,
                                slot,
                                Some(function_id),
                                signature,
                                argument_values,
                            )
                        }
                    }
                    DispatchTarget::Virtual {
                        declaring_type,
                        slot,
                        function_id,
                    } => {
                        if returns_void {
                            self.state.builder.call_virtual_void(
                                receiver_value,
                                declaring_type,
                                slot,
                                Some(function_id),
                                signature,
                                argument_values,
                            );
                            None
                        } else {
                            self.state.builder.call_virtual(
                                receiver_value,
                                declaring_type,
                                slot,
                                Some(function_id),
                                signature,
                                argument_values,
                            )
                        }
                    }
                }
            } else if returns_void {
                self.state
                    .builder
                    .call_void(function_id, signature, argument_values);
                None
            } else {
                self.state
                    .builder
                    .call(function_id, signature, argument_values)
            }
        }
        // direct call when no receiver dispatch is needed
        else if returns_void {
            self.state
                .builder
                .call_void(function_id, signature, argument_values);
            None
        } else {
            self.state
                .builder
                .call(function_id, signature, argument_values)
        };

        // reject void calls for expression results (void is not a value)
        if returns_void && matches!(kind, CallKind::Expression) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "call returned no value".to_string(),
            });
        }

        Ok((value, result_type))
    }

    fn lower_binding_call_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        function_id: mir::LocalNodeId<mir::Function>,
        signature: mir::LocalNodeId<mir::Type>,
        arguments: Vec<mir::Value>,
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let result_type_id = self.type_for_expression_or_error(expression_id)?;
        let result_info =
            resolve_result_union(self.context.types, self.context.strings, result_type_id)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "binding return type must be Result<T, PlatformError>".to_string(),
                })?;

        let union_layout = self
            .context
            .type_lowerer
            .union_layout(result_info.union_type)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        let ok_is_void = is_void_type(self.context.types, result_info.ok_value_type);
        let ok_value_mir_type = if ok_is_void {
            self.context.type_lowerer.ty_void
        } else {
            self.cached_type_for_id(expression_id, result_info.ok_value_type)?
        };
        let ok_struct_mir_type = self.cached_type_for_id(expression_id, result_info.ok_type)?;
        let err_struct_mir_type = self.cached_type_for_id(expression_id, result_info.err_type)?;
        let err_value_mir_type =
            self.cached_type_for_id(expression_id, result_info.err_value_type)?;

        let status_layout =
            self.context
                .runtime_status_layout
                .ok_or_else(|| LowerError::Internal {
                    module: self.context.module_id,
                    message: "binding ABI call is missing RuntimeStatus layout".to_string(),
                })?;

        let mut call_args = Vec::with_capacity(arguments.len() + (!ok_is_void as usize));
        let ok_out_ptr = if ok_is_void {
            None
        } else {
            let out_ptr_type = self.state.builder.type_reference(
                mir::ReferenceKind::Raw,
                ok_value_mir_type,
                mir::Mutability::Mutable,
                mir::AddressSpace::Stack,
                false,
            );
            let out_ptr = self
                .state
                .builder
                .stack_alloc(ok_value_mir_type, out_ptr_type);
            call_args.push(out_ptr);
            Some(out_ptr)
        };
        call_args.extend(arguments);

        let status_value = self
            .state
            .builder
            .call(function_id, signature, call_args)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "binding call returned no status value".to_string(),
            })?;

        let code_value = self
            .state
            .builder
            .field_get(status_value, status_layout.code_field_index);
        let zero_code = self.state.builder.iconst(0, 32, false);
        let is_ok = self.state.builder.icmp_eq(code_value, zero_code);

        let ok_block = self.state.builder.block();
        let err_block = self.state.builder.block();
        let merge_block = self.state.builder.block();
        let result_variable = self.state.builder.variable(result_type);

        self.state.builder.branch(is_ok, ok_block, err_block);

        let anchor = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));

        // ok branch
        self.state.builder.switch_to_block(ok_block);
        let ok_value = if ok_is_void {
            None
        } else {
            let out_ptr = ok_out_ptr.ok_or_else(|| LowerError::Internal {
                module: self.context.module_id,
                message: "binding ok value missing out pointer".to_string(),
            })?;
            Some(self.state.builder.load(out_ptr, ok_value_mir_type))
        };
        let ok_struct_value = self.build_result_struct_value(
            expression_id,
            ok_struct_mir_type,
            "ok",
            "value",
            ok_value,
        )?;
        let ok_union_value = self.union_value_from_variant(
            union_layout,
            result_type,
            result_info.ok_type,
            ok_struct_value,
            ok_struct_mir_type,
            anchor,
        )?;
        self.state
            .builder
            .define_variable(result_variable, ok_union_value);
        self.state.builder.jump(merge_block);

        // err branch
        self.state.builder.switch_to_block(err_block);
        let error_id_value = self
            .state
            .builder
            .field_get(status_value, status_layout.error_id_field_index);
        let take_function =
            self.context
                .take_platform_error_function
                .ok_or_else(|| LowerError::Internal {
                    module: self.context.module_id,
                    message: "binding ABI call is missing takePlatformError".to_string(),
                })?;
        let take_signature = self.signature_type_for_function(expression_id, take_function)?;
        let error_out_ptr_type = self.state.builder.type_reference(
            mir::ReferenceKind::Raw,
            err_value_mir_type,
            mir::Mutability::Mutable,
            mir::AddressSpace::Stack,
            false,
        );
        let error_out_ptr = self
            .state
            .builder
            .stack_alloc(err_value_mir_type, error_out_ptr_type);
        let _ = self
            .state
            .builder
            .call(
                take_function,
                take_signature,
                vec![error_out_ptr, error_id_value],
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "takePlatformError returned no status value".to_string(),
            })?;
        let error_value = self.state.builder.load(error_out_ptr, err_value_mir_type);
        let err_struct_value = self.build_result_struct_value(
            expression_id,
            err_struct_mir_type,
            "err",
            "error",
            Some(error_value),
        )?;
        let err_union_value = self.union_value_from_variant(
            union_layout,
            result_type,
            result_info.err_type,
            err_struct_value,
            err_struct_mir_type,
            anchor,
        )?;
        self.state
            .builder
            .define_variable(result_variable, err_union_value);
        self.state.builder.jump(merge_block);

        self.state.builder.switch_to_block(merge_block);
        let result_value = self.state.builder.use_variable(result_variable);

        Ok((result_value, result_type))
    }

    fn cached_type_for_id(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        type_id: dir::LocalTypeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        self.context
            .type_lowerer
            .cached_type(type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
    }

    fn build_result_struct_value(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        struct_mir_type: mir::LocalNodeId<mir::Type>,
        kind_literal: &str,
        value_field: &str,
        value: Option<mir::Value>,
    ) -> LowerResult<mir::Value> {
        let anchor = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));
        let layout = self
            .context
            .type_lowerer
            .layout_for_type_or_error(struct_mir_type, anchor)?;

        let kind_name = self.context.strings.intern("kind");
        let value_name = self.context.strings.intern(value_field);
        let (kind_value, _) = self.string_literal_value(kind_literal)?;

        let mut fields = Vec::with_capacity(layout.fields.len());
        let mut saw_kind = false;
        let mut saw_value = value.is_none();
        for field in &layout.fields {
            if field.name == kind_name {
                fields.push(kind_value);
                saw_kind = true;
                continue;
            }
            if field.name == value_name {
                let value = value.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "result value field is missing a payload".to_string(),
                })?;
                fields.push(value);
                saw_value = true;
                continue;
            }

            let zero_value = self.zero_value_for_type(field.ty, anchor)?;
            fields.push(zero_value);
        }

        if !saw_kind {
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "result variant missing kind field".to_string(),
            });
        }
        if !saw_value {
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "result variant missing value field".to_string(),
            });
        }

        Ok(self.state.builder.struct_(struct_mir_type, fields))
    }

    /// Lower a call through a closure value.
    fn lower_closure_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        closure_value: mir::Value,
        closure_type: mir::LocalNodeId<mir::Type>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        kind: CallKind,
    ) -> LowerResult<(Option<mir::Value>, mir::LocalNodeId<mir::Type>)> {
        let anchor = expression_id
            .into_global_any(self.context.module_id)
            .into_anchored(Some(self.context.profile));

        self.context
            .type_lowerer
            .layout_for_type_or_error(closure_type, anchor)?;

        // build arguments for the indirect call
        let mut argument_values = Vec::with_capacity(arguments.len());
        for argument_id in arguments {
            let argument = self.context.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                    message: "unsupported non-positional argument".to_string(),
                })?;
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            argument_values.push(value);
        }

        // call indirect
        let result_type = self.lower_type_for_expression(expression_id)?;
        let returns_void = result_type == self.context.type_lowerer.ty_void;
        let value = if returns_void {
            self.state
                .builder
                .call_indirect_void(closure_value, closure_type, argument_values);
            None
        } else {
            Some(
                self.state
                    .builder
                    .call_indirect(closure_value, closure_type, argument_values),
            )
        };

        // reject void calls for expression results (void is not a value)
        if returns_void && matches!(kind, CallKind::Expression) {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
                message: "call returned no value".to_string(),
            });
        }

        Ok((value, result_type))
    }

    /// Resolve a MIR signature type for a lowered function.
    pub(crate) fn signature_type_for_function(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let signature = self
            .context
            .function_signature_types
            .get(&function_id)
            .copied()
            .ok_or_else(|| self.missing_type_error(expression_id))?;
        Ok(signature)
    }

    /// Resolve receiver values for interface call lowering.
    pub(super) fn interface_call_receivers(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
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
            .context
            .type_lowerer
            .interface_ref_layout(receiver_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

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
