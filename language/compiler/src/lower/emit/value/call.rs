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
    /// v1: i32 = iconst 1
    /// v2: i32 = iconst 2
    /// v3: i32 = call @add(v1, v2) -> fn(i32, i32) -> i32
    /// ```
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

        // lower intrinsic bindings directly
        if let Some(result) = self.lower_intrinsic_binding_call(
            expression_id,
            target_symbol,
            resolution_receiver,
            dynamic_arguments,
        )? {
            return Ok(result);
        }

        // lower closure calls when the resolution target is not a function symbol
        if !self.env.functions_by_symbol.contains_key(&target_symbol)
            && let Some(type_id) = self.type_for_expression(*left)
            && matches!(self.env.types.get_type(type_id), dir::Type::Function { .. })
        {
            let (closure_value, closure_type) = self.lower_value_expression(*left)?;
            return self.lower_closure_call_from_value(
                expression_id,
                closure_value,
                closure_type,
                dynamic_arguments,
            );
        }

        // lower calls to captured functions via closure values
        let has_captures = self
            .env
            .captures
            .capture_set(target_symbol)
            .is_some_and(|set| !set.captures.is_empty());
        if self.env.symbols.get_symbol(target_symbol.local_id).ty == dir::SymbolType::Function
            && has_captures
        {
            let (closure_value, closure_type) = self.lower_value_expression(*left)?;
            return self.lower_closure_call_from_value(
                expression_id,
                closure_value,
                closure_type,
                dynamic_arguments,
            );
        }

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
                }
                | Expression::PrivateMember {
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

    /// Lower an intrinsic binding call when requested by decorators.
    fn lower_intrinsic_binding_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: dir::GlobalSymbolId,
        resolution_receiver: Option<dir::LocalTypeId>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<Option<(mir::Value, mir::LocalNodeId<mir::Type>)>> {
        // check for intrinsic binding on the target
        let symbol = self.env.symbols.get_symbol(target_symbol.local_id);
        let Some(binding) = symbol.decorators.intrinsic_binding.as_ref() else {
            return Ok(None);
        };

        // intrinsics are free functions for now
        if resolution_receiver.is_some() {
            return Err(self.error(expression_id, "intrinsic calls cannot use a receiver"));
        }

        // resolve the intrinsic name
        let name_id = binding
            .name
            .or_else(|| symbol.name())
            .ok_or_else(|| self.error(expression_id, "intrinsic binding missing symbol name"))?;
        let name = self.env.strings.get(name_id);

        // resolve the result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        // lower intrinsic casts that require the result type
        match name.as_ref() {
            "fcvt_to_sint.sat" => {
                let arguments =
                    self.lower_positional_arguments(expression_id, dynamic_arguments)?;
                let argument = match arguments.as_slice() {
                    [argument] => *argument,
                    _ => {
                        return Err(
                            self.error(expression_id, "fcvt_to_sint.sat expects a single argument")
                        );
                    }
                };
                let value = self.state.builder.cast(
                    mir::CastOperator::FloatToSignedIntSaturating,
                    argument,
                    result_type,
                );
                return Ok(Some((value, result_type)));
            }
            "fcvt_to_uint.sat" => {
                let arguments =
                    self.lower_positional_arguments(expression_id, dynamic_arguments)?;
                let argument = match arguments.as_slice() {
                    [argument] => *argument,
                    _ => {
                        return Err(
                            self.error(expression_id, "fcvt_to_uint.sat expects a single argument")
                        );
                    }
                };
                let value = self.state.builder.cast(
                    mir::CastOperator::FloatToUnsignedIntSaturating,
                    argument,
                    result_type,
                );
                return Ok(Some((value, result_type)));
            }
            _ => {}
        }

        // lower SIMD intrinsics that map to MIR vector instructions
        if name.as_ref() == "splat" {
            let (argument, argument_type) =
                self.lower_single_positional_argument(expression_id, dynamic_arguments, "splat")?;
            let element_type = self.vector_element_type(expression_id, result_type, "splat")?;
            if argument_type != element_type {
                return Err(self.error(
                    expression_id,
                    "splat argument type must match vector element type",
                ));
            }

            let value = self.state.builder.vector_splat(result_type, argument);
            return Ok(Some((value, result_type)));
        }

        if name.as_ref() == "select" {
            let argument_ids = match dynamic_arguments {
                [mask_id, then_id, else_id] => (*mask_id, *then_id, *else_id),
                _ => {
                    return Err(self.error(expression_id, "select expects a mask and two values"));
                }
            };

            let mask_expr = self.argument_expression(expression_id, argument_ids.0)?;
            let (mask, mask_type) = self.lower_value_expression(mask_expr)?;
            let then_expr = self.argument_expression(expression_id, argument_ids.1)?;
            let (then_value, then_type) = self.lower_value_expression(then_expr)?;
            let else_expr = self.argument_expression(expression_id, argument_ids.2)?;
            let (else_value, else_type) = self.lower_value_expression(else_expr)?;

            if then_type != else_type {
                return Err(self.error(
                    expression_id,
                    "select values must have matching vector types",
                ));
            }

            if result_type != then_type {
                return Err(self.error(
                    expression_id,
                    "select result type must match vector operand types",
                ));
            }

            let (mask_element, mask_lanes) =
                self.vector_type_info(expression_id, mask_type, "select")?;
            let (_, value_lanes) = self.vector_type_info(expression_id, then_type, "select")?;
            if mask_lanes != value_lanes {
                return Err(self.error(
                    expression_id,
                    "select mask lane count must match value lane count",
                ));
            }

            if !matches!(
                self.state.builder.tree().get(mask_element),
                mir::Type::Boolean
            ) {
                return Err(self.error(expression_id, "select mask element type must be boolean"));
            }

            let value = self
                .state
                .builder
                .vector_select(mask, then_value, else_value);
            return Ok(Some((value, result_type)));
        }

        if let Some(operator) = vector_reduce_operator_for_name(name.as_ref()) {
            let (argument, argument_type) = self.lower_single_positional_argument(
                expression_id,
                dynamic_arguments,
                name.as_ref(),
            )?;
            let element_type =
                self.vector_element_type(expression_id, argument_type, name.as_ref())?;
            if result_type != element_type {
                return Err(self.error(
                    expression_id,
                    "reduce intrinsic result type must match vector element type",
                ));
            }

            let value = self.state.builder.vector_reduce(operator, argument);
            return Ok(Some((value, result_type)));
        }

        // lower direct MIR intrinsics when possible
        let intrinsic: mir::Intrinsic = name.as_ref().parse().map_err(|_| {
            self.error(
                expression_id,
                format!("unsupported intrinsic binding '{}'", name.as_ref()),
            )
        })?;

        if intrinsic.is_comptime_only() {
            return Err(self.error(
                expression_id,
                "comptime-only intrinsics cannot be lowered here",
            ));
        }

        if intrinsic.requires_ordering() {
            let result = self.lower_atomic_intrinsic_binding_call(
                expression_id,
                intrinsic,
                dynamic_arguments,
                result_type,
            )?;
            return Ok(Some(result));
        }

        let arguments = self.lower_positional_arguments(expression_id, dynamic_arguments)?;
        let value = self
            .state
            .builder
            .intrinsic(intrinsic, result_type, arguments);

        Ok(Some((value, result_type)))
    }

    /// Lower a single positional argument for an intrinsic call.
    fn lower_single_positional_argument(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
        intrinsic_name: &str,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let argument_id = match dynamic_arguments {
            [argument_id] => *argument_id,
            _ => {
                return Err(self.error(
                    expression_id,
                    format!("{intrinsic_name} expects a single argument"),
                ));
            }
        };

        let argument_expr = self.argument_expression(expression_id, argument_id)?;
        let (value, value_type) = self.lower_value_expression(argument_expr)?;

        Ok((value, value_type))
    }

    /// Resolve the element type for a vector value.
    fn vector_element_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        vector_type: mir::LocalNodeId<mir::Type>,
        intrinsic_name: &str,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        match self.state.builder.tree().get(vector_type) {
            mir::Type::Vector { element, .. } => Ok(*element),
            _ => Err(self.error(
                expression_id,
                format!("{intrinsic_name} expects a vector type"),
            )),
        }
    }

    /// Resolve the element type and lane count for a vector value.
    fn vector_type_info(
        &self,
        expression_id: LocalNodeId<Expression>,
        vector_type: mir::LocalNodeId<mir::Type>,
        intrinsic_name: &str,
    ) -> LowerResult<(mir::LocalNodeId<mir::Type>, u32)> {
        match self.state.builder.tree().get(vector_type) {
            mir::Type::Vector { element, lanes, .. } => Ok((*element, *lanes)),
            _ => Err(self.error(
                expression_id,
                format!("{intrinsic_name} expects a vector type"),
            )),
        }
    }

    /// Lower positional call arguments into MIR values.
    fn lower_positional_arguments(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<Vec<mir::Value>> {
        let mut arguments = Vec::with_capacity(dynamic_arguments.len());
        for argument_id in dynamic_arguments {
            let argument = self.env.dir_tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return Err(self.error(expression_id, "unsupported non-positional argument"));
            }
            let (value, _) = self.lower_value_expression(argument.value())?;
            arguments.push(value);
        }

        Ok(arguments)
    }

    /// Lower atomic intrinsics with explicit metadata arguments.
    fn lower_atomic_intrinsic_binding_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        intrinsic: mir::Intrinsic,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
        result_type: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let base_args = usize::from(intrinsic.expected_arg_count());
        let metadata_args = atomic_metadata_arg_count();

        if dynamic_arguments.len() != base_args + metadata_args {
            return Err(self.error(
                expression_id,
                "atomic intrinsic arguments must include explicit ordering and semantics",
            ));
        }

        let (value_args, metadata_args) = dynamic_arguments.split_at(base_args);
        let arguments = self.lower_positional_arguments(expression_id, value_args)?;

        let ordering_expr = self.argument_expression(expression_id, metadata_args[0])?;
        let scope_expr = self.argument_expression(expression_id, metadata_args[1])?;
        let memory_scope_expr = self.argument_expression(expression_id, metadata_args[2])?;
        let locations_expr = self.argument_expression(expression_id, metadata_args[3])?;
        let is_volatile_expr = self.argument_expression(expression_id, metadata_args[4])?;
        let is_make_available_expr = self.argument_expression(expression_id, metadata_args[5])?;
        let is_make_visible_expr = self.argument_expression(expression_id, metadata_args[6])?;

        let ordering = self.parse_memory_ordering(expression_id, ordering_expr)?;
        let scope = self.parse_atomic_scope(expression_id, scope_expr)?;
        let memory_scope = self.parse_memory_scope(expression_id, memory_scope_expr)?;
        let locations = self.parse_memory_location_set(expression_id, locations_expr)?;
        let is_volatile = self.parse_boolean_literal(expression_id, is_volatile_expr)?;
        let is_make_available =
            self.parse_boolean_literal(expression_id, is_make_available_expr)?;
        let is_make_visible = self.parse_boolean_literal(expression_id, is_make_visible_expr)?;

        let semantics = mir::MemorySemantics::with_flags(
            locations,
            is_volatile,
            is_make_available,
            is_make_visible,
        );

        let value = self.state.builder.atomic_intrinsic(
            intrinsic,
            arguments,
            ordering,
            scope,
            memory_scope,
            semantics,
            result_type,
        );

        Ok((value, result_type))
    }

    /// Extract the expression id for a positional argument.
    fn argument_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<dir::Argument>,
    ) -> LowerResult<LocalNodeId<Expression>> {
        let argument = self.env.dir_tree.get(argument_id);
        if !matches!(argument, dir::Argument::Positional { .. }) {
            return Err(self.error(expression_id, "unsupported non-positional argument"));
        }

        Ok(argument.value())
    }

    /// Parse a MemoryOrdering constant from an expression.
    fn parse_memory_ordering(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::MemoryOrdering> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        match name.as_ref() {
            "Relaxed" => Ok(mir::MemoryOrdering::Relaxed),
            "Acquire" => Ok(mir::MemoryOrdering::Acquire),
            "Release" => Ok(mir::MemoryOrdering::Release),
            "AcqRel" => Ok(mir::MemoryOrdering::AcqRel),
            "SeqCst" => Ok(mir::MemoryOrdering::SeqCst),
            _ => Err(self.error(
                expression_id,
                "unsupported memory ordering for atomic intrinsic",
            )),
        }
    }

    /// Parse an AtomicScope constant from an expression.
    fn parse_atomic_scope(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::AtomicScope> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        match name.as_ref() {
            "Invocation" => Ok(mir::AtomicScope::Invocation),
            "Subgroup" => Ok(mir::AtomicScope::Subgroup),
            "Workgroup" => Ok(mir::AtomicScope::Workgroup),
            "Device" => Ok(mir::AtomicScope::Device),
            "CrossDevice" => Ok(mir::AtomicScope::CrossDevice),
            "QueueFamily" => Ok(mir::AtomicScope::QueueFamily),
            "ShaderCallGroup" => Ok(mir::AtomicScope::ShaderCallGroup),
            "System" => Ok(mir::AtomicScope::System),
            _ => Err(self.error(
                expression_id,
                "unsupported atomic scope for atomic intrinsic",
            )),
        }
    }

    /// Parse a MemoryScope constant from an expression.
    fn parse_memory_scope(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::MemoryScope> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        match name.as_ref() {
            "Invocation" => Ok(mir::MemoryScope::Invocation),
            "Subgroup" => Ok(mir::MemoryScope::Subgroup),
            "Workgroup" => Ok(mir::MemoryScope::Workgroup),
            "Device" => Ok(mir::MemoryScope::Device),
            "CrossDevice" => Ok(mir::MemoryScope::CrossDevice),
            "QueueFamily" => Ok(mir::MemoryScope::QueueFamily),
            "ShaderCallGroup" => Ok(mir::MemoryScope::ShaderCallGroup),
            "System" => Ok(mir::MemoryScope::System),
            _ => Err(self.error(
                expression_id,
                "unsupported memory scope for atomic intrinsic",
            )),
        }
    }

    /// Parse a MemoryLocationSet constant from an expression.
    fn parse_memory_location_set(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::MemoryLocationSet> {
        let name = self.enum_member_name(expression_id, argument_id)?;
        match name.as_ref() {
            "None" => Ok(mir::MemoryLocationSet::NONE),
            "Arguments" => Ok(mir::MemoryLocationSet::ARGUMENTS),
            "Heap" => Ok(mir::MemoryLocationSet::HEAP),
            "Stack" => Ok(mir::MemoryLocationSet::STACK),
            "Global" => Ok(mir::MemoryLocationSet::GLOBAL),
            "Shared" => Ok(mir::MemoryLocationSet::SHARED),
            "Local" => Ok(mir::MemoryLocationSet::LOCAL),
            "Constant" => Ok(mir::MemoryLocationSet::CONSTANT),
            "Inaccessible" => Ok(mir::MemoryLocationSet::INACCESSIBLE),
            "Io" => Ok(mir::MemoryLocationSet::IO),
            "Any" => Ok(mir::MemoryLocationSet::ANY),
            _ => Err(self.error(
                expression_id,
                "unsupported memory location set for atomic intrinsic",
            )),
        }
    }

    /// Parse a boolean literal for intrinsic metadata.
    fn parse_boolean_literal(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<bool> {
        let mut current = argument_id;
        loop {
            match self.env.dir_tree.get(current) {
                Expression::Parenthesized { expression } => {
                    current = *expression;
                }
                Expression::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(value),
                } => return Ok(*value),
                _ => {
                    return Err(self.error(
                        expression_id,
                        "atomic intrinsic metadata must be boolean literals",
                    ));
                }
            }
        }
    }

    /// Resolve the enum member name for an expression.
    fn enum_member_name(
        &self,
        expression_id: LocalNodeId<Expression>,
        argument_id: LocalNodeId<Expression>,
    ) -> LowerResult<destack_base::StringRef<'_>> {
        let mut current = argument_id;
        loop {
            match self.env.dir_tree.get(current) {
                Expression::Parenthesized { expression } => {
                    current = *expression;
                }
                _ => break,
            }
        }

        let Some(symbol) = self.resolved_member_symbol(current) else {
            return Err(self.error(
                expression_id,
                "atomic intrinsic metadata must be enum members",
            ));
        };
        let symbol = self.env.symbols.get_symbol(symbol.local_id);
        let name_id = symbol
            .name()
            .ok_or_else(|| self.error(expression_id, "enum member missing name"))?;
        Ok(self.env.strings.get(name_id))
    }

    /// Lower a call through a closure value.
    fn lower_closure_call_from_value(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        closure_value: mir::Value,
        closure_type: mir::LocalNodeId<mir::Type>,
        dynamic_arguments: &[LocalNodeId<dir::Argument>],
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let anchor = expression_id
            .into_global_any(self.env.module_id)
            .into_anchored(Some(self.env.profile));
        let layout = self
            .env
            .type_lowerer
            .layout_for_type_or_error(closure_type, anchor)?;
        let fn_index = layout
            .field_index_by_source(0)
            .ok_or_else(|| self.error(expression_id, "missing closure function field"))?;
        let env_index = layout
            .field_index_by_source(1)
            .ok_or_else(|| self.error(expression_id, "missing closure env field"))?;
        let fn_field = layout
            .field(fn_index)
            .ok_or_else(|| self.error(expression_id, "missing closure function field"))?;

        let fn_ptr = self.state.builder.field_get(closure_value, fn_index);
        let env_ptr = self.state.builder.field_get(closure_value, env_index);

        // build arguments for the indirect call
        let mut arguments = Vec::with_capacity(dynamic_arguments.len());
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

        // call indirect
        let result_type = self.lower_type_for_expression(expression_id)?;
        let value = self
            .state
            .builder
            .call_indirect(fn_ptr, Some(env_ptr), fn_field.ty, arguments);

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

/// Number of explicit metadata arguments required for atomic intrinsics.
fn atomic_metadata_arg_count() -> usize {
    7
}

/// Map SIMD reduce intrinsic names to MIR operators.
fn vector_reduce_operator_for_name(name: &str) -> Option<mir::VectorReduceOperator> {
    match name {
        "reduce.add" => Some(mir::VectorReduceOperator::Add),
        "reduce.mul" => Some(mir::VectorReduceOperator::Multiply),
        "reduce.min" => Some(mir::VectorReduceOperator::Min),
        "reduce.max" => Some(mir::VectorReduceOperator::Max),
        "reduce.and" => Some(mir::VectorReduceOperator::And),
        "reduce.or" => Some(mir::VectorReduceOperator::Or),
        "reduce.xor" => Some(mir::VectorReduceOperator::Xor),
        _ => None,
    }
}
