use destack_base::StringId;
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, Resolution, Type};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::emit::value::dispatch::DispatchTarget;

/// Kinds of static members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticMemberKind {
    /// Static field member.
    Field,
    /// Static method member.
    Method,
    /// Static associated type member.
    Type,
}

impl FunctionContext<'_> {
    /// Lower a member access expression to a field_get.
    pub(crate) fn lower_member_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        field_name: StringId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower getter access into function call
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && matches!(
                self.member_mode_for_symbol(target_symbol),
                Some(dir::FunctionMode::Getter)
            )
        {
            return self.lower_getter_call(expression_id, left_id, target_symbol);
        }

        // lower static field access into globals
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && self.is_static_field_symbol(target_symbol)
        {
            return self.lower_reference_expression(expression_id, target_symbol);
        }

        // lower enum member access into constants
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && let Some(result) = self.lower_enum_field_member(expression_id, target_symbol)?
        {
            return Ok(result);
        }

        // lower union discriminant field access into tag selection
        if let Some((union_type_id, layout, field)) =
            self.union_discriminant_field_for_member(left_id, field_name)
        {
            return self.lower_union_discriminant_member(
                expression_id,
                left_id,
                union_type_id,
                layout,
                field,
            );
        }

        // lower the aggregate value
        let (mut aggregate_value, aggregate_type) = self.lower_value_expression(left_id)?;

        // resolve field index through the type lowerer
        let field_index = self
            .env
            .type_lowerer
            .field_index_for_type(
                aggregate_type,
                field_name,
                self.env.strings,
                self.state.builder.tree(),
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "field not found in aggregate type".to_string(),
            })?;

        // ensure constructor fields are initialized before read
        if matches!(self.env.dir_tree.get(left_id), Expression::This) {
            let node = expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile));
            self.require_constructor_field_initialized(node, field_index as u32, field_name)?;
        }

        // load through references before field access
        let mut aggregate_type = aggregate_type;
        loop {
            let aggregate_mir_type = self.state.builder.tree().get(aggregate_type).clone();
            match aggregate_mir_type {
                mir::Type::Reference { pointee, .. } => {
                    aggregate_value = self.state.builder.load(aggregate_value, pointee);
                    aggregate_type = pointee;
                }
                _ => break,
            }
        }

        // get the result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        // emit field_get
        let value = self
            .state
            .builder
            .field_get(aggregate_value, field_index as u32);
        Ok((value, result_type))
    }

    /// Resolve a static member symbol for a member access expression.
    pub(crate) fn resolved_member_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<GlobalSymbolId> {
        // read the resolution from analyze
        let resolution = self.get_resolution(expression_id)?;

        match resolution {
            Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
            _ => None,
        }
    }

    /// Identify static members for a symbol when available.
    fn static_member_kind_for_symbol(&self, symbol: GlobalSymbolId) -> Option<StaticMemberKind> {
        // load the module for this symbol
        let module = self.env.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.env.profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // resolve the primary declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.primary_declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // inspect member declarations for static modifiers
        if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
            let member = tree.get(member_id);
            let (modifiers, kind) = match member {
                dir::Member::Field { modifiers, .. } => (modifiers, StaticMemberKind::Field),
                dir::Member::Method { modifiers, .. } => (modifiers, StaticMemberKind::Method),
                dir::Member::Type { modifiers, .. } => (modifiers, StaticMemberKind::Type),
                _ => return None,
            };

            if modifiers
                .is_some_and(|modifiers| modifiers.anchor == Some(dir::BindingAnchor::Static))
            {
                return Some(kind);
            }
        }

        // inspect property declarations for static modifiers
        if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
            let property = tree.get(property_id);
            let (modifiers, kind) = match property {
                dir::Property::Field { modifiers, .. } => (modifiers, StaticMemberKind::Field),
                dir::Property::Method { modifiers, .. } => (modifiers, StaticMemberKind::Method),
                dir::Property::Spread { .. } => return None,
            };

            if modifiers
                .is_some_and(|modifiers| modifiers.anchor == Some(dir::BindingAnchor::Static))
            {
                return Some(kind);
            }
        }

        None
    }

    /// Return true when a symbol resolves to a static field.
    pub(crate) fn is_static_field_symbol(&self, symbol: GlobalSymbolId) -> bool {
        matches!(
            self.static_member_kind_for_symbol(symbol),
            Some(StaticMemberKind::Field)
        )
    }

    /// Return true when a symbol resolves to a static method.
    pub(crate) fn is_static_method_symbol(&self, symbol: GlobalSymbolId) -> bool {
        matches!(
            self.static_member_kind_for_symbol(symbol),
            Some(StaticMemberKind::Method)
        )
    }

    /// Resolve the function mode for a member symbol when available.
    pub(crate) fn member_mode_for_symbol(
        &self,
        symbol: GlobalSymbolId,
    ) -> Option<dir::FunctionMode> {
        // load the module for this symbol
        let module = self.env.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.env.profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();

        // resolve the primary declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.primary_declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // handle member declarations
        if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
            let member = tree.get(member_id);
            if let dir::Member::Method { signature, .. } = member {
                return signature.mode;
            }
        }

        // handle property declarations
        if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
            let property = tree.get(property_id);
            if let dir::Property::Method { signature, .. } = property {
                return signature.mode;
            }
        }

        None
    }

    /// Resolve receiver values for member getter/setter calls.
    fn member_call_receivers(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
    ) -> LowerResult<(
        Option<mir::Value>,
        Option<mir::Value>,
        Option<dir::LocalTypeId>,
    )> {
        // resolve the receiver value when present
        let receiver_value = if self.receiver_is_namespace_reference(receiver_id) {
            None
        } else {
            let (value, _) = self.lower_value_expression(receiver_id)?;
            Some(value)
        };

        // resolve the receiver type id when available
        let receiver_type_id = self.type_for_expression(receiver_id);

        // resolve interface receivers for dispatch and argument passing
        let receivers = if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, receiver_value)
        {
            Some(self.interface_call_receivers(expression_id, receiver_type_id, receiver_value)?)
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

        Ok((call_receiver, dispatch_receiver, receiver_type_id))
    }

    /// Lower a getter call for a resolved member access.
    pub(crate) fn lower_getter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve receiver values for the call
        let (call_receiver, dispatch_receiver, receiver_type_id) =
            self.member_call_receivers(expression_id, receiver_id)?;

        // get the result type
        let result_type = self.lower_type_for_expression(expression_id)?;

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = call_receiver {
            arguments.push(receiver);
        }

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // resolve the call signature
        let signature = self.signature_type_for_function(expression_id, function_id)?;

        // use interface or virtual dispatch when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, dispatch_receiver)
        {
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
                        let value = value.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "getter call returned no value".to_string(),
                        })?;
                        return Ok((value, result_type));
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
                        let value = value.ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id
                                .into_global_any(self.env.module_id)
                                .into_anchored(Some(self.env.profile)),
                            message: "getter call returned no value".to_string(),
                        })?;
                        return Ok((value, result_type));
                    }
                }
            }
        }

        // emit the direct call
        let value = self.state.builder.call(function_id, signature, arguments);
        let value = value.ok_or_else(|| LowerError::UnsupportedConstruct {
            node: expression_id
                .into_global_any(self.env.module_id)
                .into_anchored(Some(self.env.profile)),
            message: "getter call returned no value".to_string(),
        })?;

        Ok((value, result_type))
    }

    /// Lower a setter call for a resolved member access.
    pub(crate) fn lower_setter_call(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        value: mir::Value,
    ) -> LowerResult<()> {
        // resolve receiver values for the call
        let (call_receiver, dispatch_receiver, receiver_type_id) =
            self.member_call_receivers(expression_id, receiver_id)?;

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = call_receiver {
            arguments.push(receiver);
        }
        arguments.push(value);

        // resolve the target function
        let function_id = *self
            .env
            .functions_by_symbol
            .get(&target_symbol)
            .ok_or_else(|| LowerError::MissingFunction {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                symbol: target_symbol,
            })?;

        // resolve the call signature
        let signature = self.signature_type_for_function(expression_id, function_id)?;

        // use interface or virtual dispatch when available
        if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, dispatch_receiver)
        {
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
                        self.state.builder.call_interface_void(
                            receiver_value,
                            declaring_type,
                            slot_id,
                            Some(function_id),
                            signature,
                            arguments,
                        );
                        return Ok(());
                    }
                    DispatchTarget::Virtual {
                        declaring_type,
                        slot_id,
                        function_id,
                    } => {
                        self.state.builder.call_virtual_void(
                            receiver_value,
                            declaring_type,
                            slot_id,
                            Some(function_id),
                            signature,
                            arguments,
                        );
                        return Ok(());
                    }
                }
            }
        }

        // emit the direct call
        self.state
            .builder
            .call_void(function_id, signature, arguments);

        Ok(())
    }

    /// Lower an index expression to an element_get.
    pub(crate) fn lower_index_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        left_id: LocalNodeId<Expression>,
        index_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // reject non array indices for native lowering
        let left_type_id = self.type_for_expression_or_error(left_id)?;
        match self.env.types.get_type(left_type_id) {
            Type::Array { .. } | Type::ArraySized { .. } => {}
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "index signatures are not supported for native lowering".to_string(),
                });
            }
        }

        // lower the array value and index
        let (array_value, _array_type) = self.lower_value_expression(left_id)?;
        let (index_value, _index_type) = self.lower_value_expression(index_id)?;

        // get the result type (element type)
        let result_type = self.lower_type_for_expression(expression_id)?;

        // emit element_get
        let value = self.state.builder.element_get(array_value, index_value);
        Ok((value, result_type))
    }
}
