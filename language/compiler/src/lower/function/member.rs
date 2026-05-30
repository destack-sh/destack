use destack_core::StringId;
use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError};

use crate::lower::{DispatchTarget, FunctionLowerer};

/// Kinds of static members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticMemberKind {
    /// Static field member.
    Field,
    /// Static method member.
    Method,
}

impl FunctionLowerer<'_> {
    /// Resolve a static integer literal for tuple indexing.
    fn static_index_literal(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<usize> {
        // require a compile time integer literal
        let index_expression = self.unwrap_expression(expression_id);
        match self.context.dir_tree.get(index_expression) {
            dir::Expression::ScalarLiteral(
                dir::ScalarLiteral::Integer(value) | dir::ScalarLiteral::Bigint(value),
            ) if *value >= 0 => Ok(*value as usize),
            _ => Err(self
                .error(expression_id, "tuple index must be a constant integer")
                .into()),
        }
    }

    /// Lower a member access expression to a field_get.
    ///
    /// ```ds
    /// struct Vec2 { x: int32; y: int32; }
    ///
    /// function read(v: Vec2): int32 {
    ///     return v.x;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: i32 = field.get v0, 0
    /// ```
    pub(crate) fn lower_member_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        field_name: StringId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // lower getter access into function call
        if let Some(target_symbol) = self.resolved_member_symbol(expression_id)
            && matches!(
                self.member_role_for_symbol(target_symbol),
                Some(dir::FunctionRole::Getter)
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
            .context
            .type_lowerer
            .field_index_for_type(
                aggregate_type,
                field_name,
                self.context.strings,
                self.state.builder.tree(),
            )
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "field not found in aggregate type".to_string(),
            })
            .map_err(CompilerError::from)?;

        // ensure constructor fields are initialized before read
        if matches!(self.context.dir_tree.get(left_id), dir::Expression::This) {
            let node = expression_id
                .into_global_any(self.context.module_id)
                .into_anchored(Some(self.context.profile));
            self.require_constructor_field_initialized(node, field_index as u32, field_name)?;
        }

        // load through references before field access
        let mut aggregate_type = aggregate_type;
        loop {
            let aggregate_mir_type = self.state.builder.tree().get(aggregate_type).clone();
            match aggregate_mir_type {
                mir::Type::Reference { pointee, .. } => {
                    let pointee = pointee.ty().ok_or_else(|| {
                        self.error(expression_id, "aggregate pointee type is not concrete")
                    })?;

                    self.emit_null_check(expression_id, aggregate_value, aggregate_type)?;
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
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let resolution = self.get_member_resolution(expression_id)?;

        match &resolution.target {
            dir::MemberTarget::Symbol(candidate) => Some(candidate.symbol),
            dir::MemberTarget::Builtin(_)
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Select(_) => None,
        }
    }

    /// Identify static members for a symbol when available.
    fn static_member_kind_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<StaticMemberKind> {
        // load checked dir data for this symbol
        let dir = self.dir_bound_if_present(symbol.module_id)?;
        let parsed = self.dir_parsed_if_present(symbol.module_id)?;
        let tree = &parsed.tree;
        let symbols = dir.binding_table();

        // resolve the declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // inspect member declarations for static modifiers
        if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
            let member = tree.get(member_id);
            let (is_static, kind) = match member {
                dir::Member::Field { is_static, .. } => (is_static, StaticMemberKind::Field),
                dir::Member::Method { is_static, .. } => (is_static, StaticMemberKind::Method),
                _ => return None,
            };

            if *is_static {
                return Some(kind);
            }
        }

        // only member declarations can be static
        None
    }

    /// Return true when a symbol resolves to a static field.
    pub(crate) fn is_static_field_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        matches!(
            self.static_member_kind_for_symbol(symbol),
            Some(StaticMemberKind::Field)
        )
    }

    /// Return true when a symbol resolves to a static method.
    pub(crate) fn is_static_method_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        matches!(
            self.static_member_kind_for_symbol(symbol),
            Some(StaticMemberKind::Method)
        )
    }

    /// Resolve the function role for a member symbol when available.
    pub(crate) fn member_role_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::FunctionRole> {
        // load checked dir data for this symbol
        let dir = self.dir_bound_if_present(symbol.module_id)?;
        let parsed = self.dir_parsed_if_present(symbol.module_id)?;
        let tree = &parsed.tree;
        let symbols = dir.binding_table();

        // resolve the declaration node
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary = symbol_entry.declaration?;
        if primary.module_id != symbol.module_id {
            return None;
        }

        // handle member declarations
        if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
            let member = tree.get(member_id);
            if let dir::Member::Method { signature, .. } = member {
                return signature.role;
            }
        }

        // handle property declarations
        if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
            let property = tree.get(property_id);
            if let dir::Property::Method { signature, .. } = property {
                return signature.role;
            }
        }

        None
    }

    /// Resolve receiver values for member getter/setter calls.
    fn member_call_receivers(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(
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

        // resolve dynamic receivers for dispatch and argument passing
        let receivers = if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, receiver_value)
        {
            Some(self.dynamic_call_receivers(expression_id, receiver_type_id, receiver_value)?)
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
    ///
    /// ```ds
    /// class Box {
    ///     get size(): int32 { return 1; }
    /// }
    ///
    /// function read(value: Box): int32 {
    ///     return value.size;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = call Box.size(v0)
    /// ```
    pub(crate) fn lower_getter_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
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

        let function_id = self.function_for_symbol(target_symbol);
        let dispatch_target = if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, dispatch_receiver)
        {
            self.dispatch_target_for_symbol(
                expression_id,
                receiver_type_id,
                target_symbol,
                function_id,
            )?
            .map(|target| (target, receiver_value))
        } else {
            None
        };
        let signature = match &dispatch_target {
            Some((DispatchTarget::Dynamic { signature, .. }, _)) => *signature,
            _ => {
                let function_id = function_id
                    .ok_or_else(|| LowerError::MissingFunction {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        symbol: target_symbol,
                    })
                    .map_err(CompilerError::from)?;
                self.signature_type_for_function(expression_id, function_id)?
            }
        };

        if let Some((dispatch_target, receiver_value)) = dispatch_target {
            let value = match dispatch_target {
                DispatchTarget::Dynamic {
                    constraint,
                    slot,
                    signature: _,
                } => self.state.builder.call_dynamic(
                    receiver_value,
                    constraint,
                    slot,
                    signature,
                    arguments,
                ),
                DispatchTarget::Virtual {
                    class,
                    slot,
                    function_id,
                } => self.state.builder.call_virtual(
                    receiver_value,
                    class,
                    slot,
                    Some(function_id),
                    signature,
                    arguments,
                ),
            };
            let value = value
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "getter call returned no value".to_string(),
                })
                .map_err(CompilerError::from)?;
            return Ok((value, result_type));
        };

        // emit the direct call
        let function_id = function_id
            .ok_or_else(|| LowerError::MissingFunction {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                symbol: target_symbol,
            })
            .map_err(CompilerError::from)?;
        let value = self.state.builder.call(function_id, signature, arguments);
        let value = value
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "getter call returned no value".to_string(),
            })
            .map_err(CompilerError::from)?;

        Ok((value, result_type))
    }

    /// Lower a setter call for a resolved member access.
    ///
    /// ```ds
    /// class Box {
    ///     set size(value: int32) { }
    /// }
    ///
    /// function write(value: Box): void {
    ///     value.size = 3;
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 3
    /// call Box.size(v0, v1)
    /// ```
    pub(crate) fn lower_setter_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
        value: mir::Value,
    ) -> CompilerResult<()> {
        // resolve receiver values for the call
        let (call_receiver, dispatch_receiver, receiver_type_id) =
            self.member_call_receivers(expression_id, receiver_id)?;

        // build argument list
        let mut arguments = Vec::new();
        if let Some(receiver) = call_receiver {
            arguments.push(receiver);
        }
        arguments.push(value);

        let function_id = self.function_for_symbol(target_symbol);
        let dispatch_target = if let (Some(receiver_type_id), Some(receiver_value)) =
            (receiver_type_id, dispatch_receiver)
        {
            self.dispatch_target_for_symbol(
                expression_id,
                receiver_type_id,
                target_symbol,
                function_id,
            )?
            .map(|target| (target, receiver_value))
        } else {
            None
        };
        let signature = match &dispatch_target {
            Some((DispatchTarget::Dynamic { signature, .. }, _)) => *signature,
            _ => {
                let function_id = function_id
                    .ok_or_else(|| LowerError::MissingFunction {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        symbol: target_symbol,
                    })
                    .map_err(CompilerError::from)?;
                self.signature_type_for_function(expression_id, function_id)?
            }
        };

        if let Some((dispatch_target, receiver_value)) = dispatch_target {
            match dispatch_target {
                DispatchTarget::Dynamic {
                    constraint,
                    slot,
                    signature: _,
                } => self.state.builder.call_dynamic_void(
                    receiver_value,
                    constraint,
                    slot,
                    signature,
                    arguments,
                ),
                DispatchTarget::Virtual {
                    class,
                    slot,
                    function_id,
                } => self.state.builder.call_virtual_void(
                    receiver_value,
                    class,
                    slot,
                    Some(function_id),
                    signature,
                    arguments,
                ),
            }
            return Ok(());
        }

        // emit the direct call
        let function_id = function_id
            .ok_or_else(|| LowerError::MissingFunction {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                symbol: target_symbol,
            })
            .map_err(CompilerError::from)?;
        self.state
            .builder
            .call_void(function_id, signature, arguments);

        Ok(())
    }

    /// Lower an index expression to an element address and load.
    ///
    /// ```ds
    /// function read(values: [int32; 3]): int32 {
    ///     return values[1];
    /// }
    /// ```
    /// ->
    /// ```mir
    /// v1: int32 = const 1
    /// v2: ref<int32, raw, readonly, space(frame)> = element.address v0, v1
    /// v3: int32 = load v2
    /// ```
    pub(crate) fn lower_index_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left_id: dir::LocalNodeId<dir::Expression>,
        index_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        // resolve the left-hand type
        let left_type_id = self.type_for_expression_or_error(left_id)?;
        let left_type_id = self.unwrap_form_payload_type_id(left_type_id);
        let left_mir_type = self.lower_type_for_expression(left_id)?;

        // handle array indexing
        if matches!(
            self.context.types.get_type(left_type_id),
            dir::Type::Slice(_) | dir::Type::FixedArray(_)
        ) {
            // lower the array value and index
            let (array_value, _array_type) = self.lower_value_expression(left_id)?;
            let (index_value, _index_type) = self.lower_value_expression(index_id)?;

            // emit bounds checks when enabled
            let array_type = self.lower_type_for_expression(left_id)?;
            self.emit_bounds_check(
                expression_id,
                array_value,
                array_type,
                index_value,
                index_id,
            )?;

            // get the result type for the element
            let result_type = self.lower_type_for_expression(expression_id)?;

            // dynamic indexing projects an address first
            let reference_type = self.state.builder.type_reference(
                mir::ReferenceKind::Raw,
                result_type,
                mir::Access::Readonly,
                mir::Space::Frame,
                mir::Nullability::None,
            );
            let pointer = self
                .state
                .builder
                .element_addr(array_value, index_value, reference_type);
            let value = self.state.builder.load(pointer, result_type);
            return Ok((value, result_type));
        }

        // handle tuple and newtype indexing
        let (element_types, newtype_inner, is_tuple_payload) = {
            let mir_type = self.state.builder.tree().get(left_mir_type);
            match mir_type {
                mir::Type::Tuple { elements, .. } => (
                    elements
                        .iter()
                        .cloned()
                        .map(|element| element.ty())
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| {
                            self.error(expression_id, "tuple element type is not concrete")
                        })?,
                    None,
                    true,
                ),
                mir::Type::Newtype { inner, .. } => {
                    let inner_type = inner.ty().ok_or_else(|| {
                        self.error(expression_id, "newtype inner type is not concrete")
                    })?;
                    let inner_payload = self.state.builder.tree().get(inner_type);
                    match inner_payload {
                        mir::Type::Tuple { elements, .. } => (
                            elements
                                .iter()
                                .copied()
                                .map(|element| element.ty())
                                .collect::<Option<Vec<_>>>()
                                .ok_or_else(|| {
                                    self.error(expression_id, "tuple element type is not concrete")
                                })?,
                            Some(inner_type),
                            true,
                        ),
                        _ => (vec![inner_type], Some(inner_type), false),
                    }
                }
                _ => {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "index signatures are not supported for native lowering"
                            .to_string(),
                    }
                    .into());
                }
            }
        };

        // require a constant integer index for tuples and newtypes
        let index = self.static_index_literal(index_id)?;

        // lower the payload value after resolving the type
        let (payload_value, _) = self.lower_value_expression(left_id)?;
        let payload_value = match newtype_inner {
            Some(inner_type) => self.state.builder.bitcast(payload_value, inner_type),
            None => payload_value,
        };

        // ensure the index is in bounds
        if index >= element_types.len() {
            return Err(self
                .error(expression_id, "tuple index is out of bounds")
                .into());
        }

        // extract the payload element
        let element_type = element_types[index];
        let value = if is_tuple_payload {
            self.state.builder.field_get(payload_value, index as u32)
        } else {
            payload_value
        };

        Ok((value, element_type))
    }
}
