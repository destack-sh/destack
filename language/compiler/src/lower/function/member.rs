use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one member read through its resolution.
    pub(in crate::lower) fn lower_member(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // read the access's own optionality for its chain guard
        let is_optional = matches!(
            self.source().tree().get(expression),
            dir::Expression::Member {
                is_optional: true,
                ..
            }
        );

        // construct the case value for enum members
        if let dir::Type::Variant(variant) = self.node_type(expression)? {
            return self.lower_variant_member(&variant);
        }

        // read a decided reference as its resolved symbol
        let node = expression.into_global_any(self.source);
        let named = self.source().decisions.function_symbol(node).or_else(|| {
            self.source()
                .resolutions
                .name_resolution(node)
                .and_then(|resolution| resolution.single_symbol())
        });
        if let Some(symbol) = named {
            return self.lower_resolved_value(expression, symbol);
        }

        // read the single access selected for this member
        let resolution = self.member_decision(expression)?;
        let dir::OperationResolution::One(access) = &resolution else {
            return self.lower_union_member_read(expression, left, resolution.arms());
        };

        // lower the read at the selected target
        match &access.target {
            // project a compiler-defined member off the receiver
            dir::MemberTarget::Projection {
                receiver,
                projection,
                ..
            } => {
                let receiver = self.lower_adjusted_receiver(left, receiver, is_optional)?;

                self.lower_member_projection(receiver, projection)
            }
            // call the resolved getter
            dir::MemberTarget::Call(call) => {
                let dir::Call {
                    target:
                        dir::CallableTarget::Symbol {
                            function,
                            dispatch: dir::FunctionDispatch::Direct,
                        },
                    ..
                } = &**call
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a dynamic property read".to_string(),
                    }
                    .into());
                };
                let value =
                    self.lower_function_target_call(left, call, function, None, is_optional)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a getter".to_string(),
                })
            }
            // read the resolved field
            dir::MemberTarget::Field(field) => {
                self.lower_field_read(expression, left, field, is_optional)
            }
            // reject every other member read
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("a '{}' member read", other.name()),
            }
            .into()),
        }
    }

    /// Lower one subscript read through its resolution.
    pub(in crate::lower) fn lower_subscript(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        index: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<mir::Value> {
        // read the access's own optionality for its chain guard
        let is_optional = matches!(
            self.source().tree().get(expression),
            dir::Expression::Index {
                is_optional: true,
                ..
            }
        );

        // read the single access selected for this subscript
        let resolution = self.subscript_decision(expression)?;
        let dir::OperationResolution::One(subscript) = resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a subscript read on a union receiver".to_string(),
            }
            .into());
        };

        // lower the read at the selected target
        match subscript.target {
            // read the member the constant key names
            dir::SubscriptTarget::Member(access) => match access.target {
                // project a compiler-defined member off the receiver
                dir::MemberTarget::Projection {
                    receiver,
                    projection,
                    ..
                } => {
                    let receiver = self.lower_adjusted_receiver(left, &receiver, is_optional)?;

                    // evaluate the computed singleton key
                    if let Some(index) = index {
                        self.lower_const_expression(index)?;
                    }

                    self.lower_member_projection(receiver, &projection)
                }
                // read the constant key straight off its field
                dir::MemberTarget::Field(field) => {
                    let value = self.lower_member_receiver(left, &field.receiver, is_optional)?;

                    // evaluate the computed singleton key
                    if let Some(index) = index {
                        self.lower_const_expression(index)?;
                    }

                    self.lower_field_value(expression, value, &field)
                }
                // find the computed key through the receiver's dynamic table
                dir::MemberTarget::Index(read)
                    if matches!(read.target, dir::IndexTarget::Signature(_)) =>
                {
                    // dispatch a keyed find by name over string domains only
                    let domain = read.key_type;
                    if !matches!(
                        self.lower.ty(domain)?,
                        dir::Type::Primitive(dir::PrimitiveType::String)
                    ) {
                        return Err(LowerError::Unsupported {
                            anchor: self.lower.module.into(),
                            construct: "a non-string signature key domain".to_string(),
                        }
                        .into());
                    }
                    let key = index.ok_or_else(|| CompilerError::Internal {
                        message: "a signature subscript read without a key expression".to_string(),
                    })?;

                    self.lower_dynamic_signature_read(expression, left, key)
                }
                // reject every other subscript read
                other => Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("a '{}' subscript read", other.name()),
                }
                .into()),
            },
            // call the resolved subscript getter
            dir::SubscriptTarget::Call(call) => {
                let dir::Call {
                    target:
                        dir::CallableTarget::Symbol {
                            function,
                            dispatch: dir::FunctionDispatch::Direct,
                        },
                    ..
                } = &call
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a dynamic subscript read".to_string(),
                    }
                    .into());
                };
                let value =
                    self.lower_function_target_call(left, &call, function, None, is_optional)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a subscript read".to_string(),
                })
            }
            // load the value behind the address the Index protocol returns
            dir::SubscriptTarget::Index(read) => {
                if read.missing.is_some() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "an Index read with a missing result".to_string(),
                    }
                    .into());
                }

                // require a directly dispatched Index call
                let call = read.call;
                let dir::Call {
                    target:
                        dir::CallableTarget::Symbol {
                            function,
                            dispatch: dir::FunctionDispatch::Direct,
                        },
                    ..
                } = &call
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a dynamic subscript read".to_string(),
                    }
                    .into());
                };
                let value =
                    self.lower_function_target_call(left, &call, function, None, is_optional)?;
                let value = value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a subscript read".to_string(),
                })?;
                let result_type = self.lower_type(read.dereference.ty)?;

                Ok(self.builder.load(value, result_type))
            }
        }
    }

    /// Lower one field read.
    fn lower_field_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        field: &dir::FieldResolution,
        is_optional: bool,
    ) -> CompilerResult<mir::Value> {
        // read erased receivers through their dispatch table
        if let dir::MemberReceiver::Dynamic(dispatch) = &field.receiver {
            let dispatch = dispatch.clone();

            return self.lower_dynamic_member_read(expression, left, field, &dispatch);
        }

        // lower the receiver the resolution selected
        let value = self.lower_member_receiver(left, &field.receiver, is_optional)?;

        self.lower_field_value(expression, value, field)
    }

    /// Lower one field read from its already adjusted receiver value.
    fn lower_field_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        field: &dir::FieldResolution,
    ) -> CompilerResult<mir::Value> {
        // dispatch a field the arms of a union receiver share
        let receiver = field.receiver.ty();
        let indirect = self
            .lower
            .peel_indirection(receiver)?
            .map(|layer| layer.stored);
        let stored = self.union_stored(indirect.unwrap_or(receiver))?;
        if let dir::Type::Union(_) = self.lower.ty(stored)? {
            return self.lower_union_field_read(expression, value, field, stored);
        }

        // resolve the selected storage field and result type
        let index = self.member_field_index(field)?;
        let result_type = self.lower_type(self.node_type_id(expression)?)?;

        // load fields through addresses for reference receivers
        if let Some(layer) = self.lower.peel_indirection(receiver)? {
            let address = self.emit_field_address(value, index, result_type, layer.access);

            return Ok(self.builder.load(address, result_type));
        }

        Ok(self.builder.field_get(value, index))
    }

    /// Lower one payload-free variant member to its case construction.
    fn lower_variant_member(&mut self, variant: &dir::VariantType) -> CompilerResult<mir::Value> {
        // materialize the owner representation and select the declared case
        let dir::Type::Application(owner) = self.lower.ty(variant.owner)? else {
            return Err(CompilerError::Internal {
                message: "a variant without its owner instance".to_string(),
            });
        };
        let ty = self.lower_type(variant.owner)?;
        let case = self.lower.variant_position(owner.symbol, variant.variant)?;

        Ok(self.builder.variant_new(ty, case, None))
    }

    /// Lower one compiler-defined member projection.
    fn lower_member_projection(
        &mut self,
        receiver: mir::Value,
        projection: &dir::Projection,
    ) -> CompilerResult<mir::Value> {
        // lower the projection the member names
        match projection {
            // read the structural discriminant
            dir::Projection::Discriminant {
                union, cases, ty, ..
            } => self.lower_discriminant_value(receiver, *union, cases, *ty),
            // reject every other projection
            other => Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: format!("a '{}' member projection", other.name()),
            }
            .into()),
        }
    }

    /// Lower one structural discriminant to its source property value.
    fn lower_discriminant_value(
        &mut self,
        receiver: mir::Value,
        union: dir::GlobalTypeId,
        cases: &[dir::DiscriminantCase],
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // require at least one checked runtime case
        if cases.is_empty() {
            return Err(CompilerError::Internal {
                message: "a discriminant projection without a reachable case".to_string(),
            });
        }

        // read the arms of the union the receiver carries
        let source_members = self.union_members(union)?;

        // read the result's literal arms while it stays an indexed union
        let result = self.lower.ty(ty)?;
        let is_singleton = result.singleton_literal().is_some();
        let result_members = match result {
            dir::Type::Union(_) if self.lower.scalar_literal_union(ty)?.is_some() => None,
            dir::Type::Union(result) => Some(
                self.lower
                    .types(ty.module_id)?
                    .type_ids(result.elements)
                    .to_vec(),
            ),
            _ => None,
        };

        // map physical union cases to the projected source values
        let mut mappings = Vec::with_capacity(cases.len());
        for case in cases {
            let Some(source_index) = source_members.iter().position(|arm| *arm == case.arm) else {
                return Err(CompilerError::Internal {
                    message: "a discriminant projection over an absent union arm".to_string(),
                });
            };

            // select the result arm carrying this case's literal
            let mut result_index = None;
            if let Some(result_members) = &result_members {
                for (index, member) in result_members.iter().enumerate() {
                    let value = self.lower.ty(*member)?.singleton_literal();
                    if value == Some(case.value) {
                        result_index = Some(index as u32);
                        break;
                    }
                }

                if result_index.is_none() {
                    return Err(CompilerError::Internal {
                        message: "a discriminant projection value absent from its result type"
                            .to_string(),
                    });
                }
            }

            mappings.push((source_index as i128, result_index, case.value));
        }

        // dispatch on the physical tag and construct the corresponding source value
        let tag = self.lower_discriminant_tag(receiver, union)?;
        let result_type = self.lower_type(ty)?;
        let result = self.builder.local(result_type, mir::Mutability::Immutable);
        let exit = self.builder.block();
        let unreachable = self.builder.block();
        let blocks = mappings
            .iter()
            .map(|(source, _, _)| (*source, self.builder.block()))
            .collect::<Vec<_>>();
        self.builder.switch(tag, unreachable, blocks.clone());

        // materialize the source-level property value in every reachable case
        for ((_, result_index, literal), (_, block)) in mappings.into_iter().zip(blocks) {
            self.builder.switch_to_block(block);
            let value = match result_index {
                Some(result_index) => self.builder.variant_new(result_type, result_index, None),
                None if is_singleton => {
                    self.builder.constant(mir::Constant::Undefined, result_type)
                }
                None => {
                    let result_type = self.builder.tree().get(result_type).clone();

                    self.lower_constant(literal, result_type)?
                }
            };
            self.builder.local_set(result, value);
            self.builder.jump(exit);
        }

        // reject any physical case excluded by the checked flow type
        self.builder.switch_to_block(unreachable);
        self.builder.unreachable();
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(result))
    }

    /// Lower the physical tag carried by one union receiver.
    pub(in crate::lower) fn lower_discriminant_tag(
        &mut self,
        receiver: mir::Value,
        union: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // read the receiver's own representation
        let receiver_type = self.value_representation(receiver)?;

        // stored unions expose their tag through their address
        if matches!(
            self.builder.tree().get(receiver_type),
            mir::Type::Reference { .. } | mir::Type::Pointer { .. }
        ) {
            let union = self.lower_type(union)?;

            return Ok(self.builder.variant_tag_load(receiver, union));
        }

        Ok(self.builder.variant_tag(receiver))
    }

    /// Apply the selected receiver adjustments before one member read.
    fn lower_member_receiver(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        receiver: &dir::MemberReceiver,
        is_optional: bool,
    ) -> CompilerResult<mir::Value> {
        // require a direct receiver
        let dir::MemberReceiver::Direct(receiver) = receiver else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a member read through dynamic dispatch".to_string(),
            }
            .into());
        };

        self.lower_adjusted_receiver(expression, receiver, is_optional)
    }

    /// Return the storage index selected by one field resolution.
    pub(in crate::lower) fn member_field_index(
        &self,
        field: &dir::FieldResolution,
    ) -> CompilerResult<u32> {
        // locate storage beneath every reference layer of the selected receiver
        let mut stored = field.receiver.ty();
        while let Some(layer) = self.lower.peel_indirection(stored)? {
            if layer.stored == stored {
                break;
            }
            stored = layer.stored;
        }
        let mut stored = self.lower.peel_owned(stored)?;

        // follow transparent alias and newtype definitions, re-peeling their owners
        while let dir::Type::Application(application) = self.lower.ty(stored)? {
            let defined = match self.lower.definition(application.symbol)? {
                Some(dir::Definition::TypeAlias(alias)) => alias.value,
                Some(dir::Definition::Newtype(newtype)) => newtype.backing,
                _ => break,
            };

            // read the body through the application's own instance
            let specialization = self
                .lower
                .application_specialization(stored, &application)?;
            let defined = self.lower.instance_type(specialization, defined)?;

            stored = self.lower.peel_owned(defined)?;
        }

        // find the field's position in the storage the receiver declares
        let index = match self.lower.ty(stored)? {
            dir::Type::Tuple(_) => match field.target.key() {
                dir::StaticKey::Index(index) => Some(index),
                _ => None,
            },
            dir::Type::Application(instance) => {
                let fields = self.lower.nominal_fields(instance.symbol)?;

                fields.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { symbol, .. } => stored.symbol == symbol,
                })
            }
            dir::Type::Object(shape) => {
                let properties = self
                    .lower
                    .types(stored.module_id)?
                    .properties(shape.properties);

                properties.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { .. } => false,
                })
            }
            stored_head => {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!(
                        "a member read on a '{}' receiver",
                        stored_head.variant_name()
                    ),
                }
                .into());
            }
        };

        index
            .map(|index| index as u32)
            .ok_or_else(|| CompilerError::Internal {
                message: "a field absent from its lowered receiver".to_string(),
            })
    }

    /// Dispatch one member read over the union arms its resolution recorded.
    fn lower_union_member_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arms: &[dir::MemberAccess],
    ) -> CompilerResult<mir::Value> {
        // read each arm's adjustment chain off its adjusted receiver
        let mut chains = Vec::with_capacity(arms.len());
        for arm in arms {
            chains.push(self.union_arm_receiver(arm)?.adjustments.clone());
        }

        // take the shared prefix reaching the dispatched union
        let prefix = shared_adjustment_prefix(&chains);

        // reject an arm re-projecting the payload the dispatch selects
        for chain in &chains {
            if let Some(dir::ReceiverAdjustment::UnionPayload { .. }) = chain.get(prefix) {
                return Err(CompilerError::Internal {
                    message: "a union member arm re-projecting its dispatched payload".to_string(),
                });
            }
        }

        // evaluate the receiver once and walk the shared prefix to the union
        let receiver = self.lower_expression(left)?;
        let first = &chains[0];
        let dispatch = self.lower_receiver_adjustments(receiver, &first[..prefix])?;
        let union = match prefix {
            0 => self.node_type_id(left)?,
            _ => first[prefix - 1].ty(),
        };
        let members = self.union_members(union)?;

        // route each arm by the case its narrowed receiver selects
        let mut targets = Vec::with_capacity(arms.len());
        let mut blocks = Vec::with_capacity(arms.len());
        for arm in arms {
            let position = self.union_case_position(&members, arm.receiver)?;
            let block = self.builder.block();
            targets.push((position, block));
            blocks.push(block);
        }

        // dispatch on the value's own discriminant, loading it behind stored receivers
        let received = self.value_representation(dispatch)?;
        let stored = match self.builder.tree().get(received).clone() {
            // switch a bare variant receiver on its own case
            mir::Type::Variant { .. } => {
                self.builder.variant_switch(dispatch, None, targets.clone());

                false
            }
            // load the encoded tag behind a stored union receiver
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
                let tag = self.builder.variant_tag_load(dispatch, pointee);
                let cases = targets
                    .iter()
                    .map(|(position, block)| (i128::from(*position), *block))
                    .collect();
                let unreached = self.builder.block();
                self.builder.switch(tag, unreached, cases);
                self.builder.switch_to_block(unreached);
                self.builder.unreachable();

                true
            }
            // reject every other receiver representation
            _ => {
                return Err(CompilerError::Internal {
                    message: "a union member read outside a variant representation".to_string(),
                });
            }
        };

        // run each arm's narrowed read and join the member values
        let ty = self.node_type_id(expression)?;
        let representation = self.lower_type(ty)?;
        let slot = self
            .builder
            .local(representation, mir::Mutability::Immutable);
        let exit = self.builder.block();
        for ((arm, chain), ((position, _), block)) in
            arms.iter().zip(&chains).zip(targets.iter().zip(blocks))
        {
            self.builder.switch_to_block(block);

            // extract the dispatched arm payload, keeping stored receivers addressed
            let payload = match stored {
                true => {
                    let target = self.lower_type(arm.receiver)?;

                    self.builder
                        .variant_payload_addr(dispatch, *position, target)
                }
                false => self.builder.variant_payload(dispatch, *position),
            };

            // narrow the payload through the arm's remaining adjustments
            let narrowed = self.lower_receiver_adjustments(payload, &chain[prefix..])?;

            // read the member at the arm's selected target
            let value = match &arm.target {
                dir::MemberTarget::Field(field) => {
                    self.lower_field_value(expression, narrowed, field)?
                }
                dir::MemberTarget::Projection { projection, .. } => {
                    self.lower_member_projection(narrowed, projection)?
                }
                other => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: format!("a '{}' member read on a union receiver", other.name()),
                    }
                    .into());
                }
            };

            // join the arm value at the member's committed representation
            let value = self.adapt_to_representation(value, representation)?;
            self.builder.local_set(slot, value);
            self.builder.jump(exit);
        }

        // continue lowering at the exit block
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }

    /// Return one union arm's adjusted receiver.
    fn union_arm_receiver<'access>(
        &self,
        arm: &'access dir::MemberAccess,
    ) -> CompilerResult<&'access dir::AdjustedReceiver> {
        let adjusted = match &arm.target {
            dir::MemberTarget::Field(field) => match &field.receiver {
                dir::MemberReceiver::Direct(adjusted) => Some(adjusted),
                _ => None,
            },
            dir::MemberTarget::Projection { receiver, .. } => Some(receiver),
            _ => None,
        };

        adjusted.ok_or_else(|| CompilerError::Internal {
            message: "a union member arm without an adjusted receiver".to_string(),
        })
    }

    /// Dispatch one shared field read over its union receiver's cases.
    fn lower_union_field_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        field: &dir::FieldResolution,
        union: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // settle the receiver to the bare dispatched variant value
        let mut value = value;
        loop {
            let received = self.value_representation(value)?;
            match self.builder.tree().get(received).clone() {
                // load the union out of stored receivers holding a tagged payload
                mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. }
                    if matches!(
                        self.builder.tree().get(pointee),
                        mir::Type::Variant { .. }
                            | mir::Type::Newtype { .. }
                            | mir::Type::Reference { .. }
                            | mir::Type::Pointer { .. }
                    ) =>
                {
                    value = self.builder.load(value, pointee);
                }
                // unwrap enclosing newtype layers
                mir::Type::Newtype { .. } => value = self.builder.field_get(value, 0),
                // stop at the bare variant
                mir::Type::Variant { .. } => break,
                // reject an erased union layout until descriptor dispatch exists
                _ => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "a member read on an erased union layout".to_string(),
                    }
                    .into());
                }
            }
        }

        // route each case to its own block
        let members = self.union_members(union)?;
        let mut targets = Vec::with_capacity(members.len());
        for index in 0..members.len() {
            targets.push((index as u32, self.builder.block()));
        }
        self.builder.variant_switch(value, None, targets.clone());

        // read the shared key off each case and join the values
        let result_type = self.lower_type(self.node_type_id(expression)?)?;
        let slot = self.builder.local(result_type, mir::Mutability::Immutable);
        let exit = self.builder.block();
        for (member, (position, block)) in members.iter().zip(targets) {
            self.builder.switch_to_block(block);

            // find the key's position on this case's nominal
            let arm = self.lower.peel_owned(*member)?;
            let dir::Type::Application(application) = self.lower.ty(arm)? else {
                return Err(CompilerError::Internal {
                    message: "a union field read on a structural arm".to_string(),
                });
            };
            let fields = self.lower.nominal_fields(application.symbol)?;
            let key = field.target.key();
            let Some(index) = fields.iter().position(|stored| stored.key == key) else {
                return Err(CompilerError::Internal {
                    message: "a shared field absent from one union arm".to_string(),
                });
            };

            // read the field off the payload, loading through reference arms
            let payload = self.builder.variant_payload(value, position);
            let received = self.value_representation(payload)?;
            let read = match self.builder.tree().get(received).clone() {
                // read a stored arm through its field address
                mir::Type::Reference { .. } | mir::Type::Pointer { .. } => {
                    let address = self.emit_field_address(
                        payload,
                        index as u32,
                        result_type,
                        mir::Access::Readonly,
                    );

                    self.builder.load(address, result_type)
                }
                // read a value arm's field directly
                _ => self.builder.field_get(payload, index as u32),
            };

            // join the case value at the field's committed representation
            let read = self.adapt_to_representation(read, result_type)?;
            self.builder.local_set(slot, read);
            self.builder.jump(exit);
        }

        // continue lowering at the exit block
        self.builder.switch_to_block(exit);

        Ok(self.builder.local_get(slot))
    }
}

/// Return the length of the adjustment prefix every chain shares.
fn shared_adjustment_prefix(chains: &[Vec<dir::ReceiverAdjustment>]) -> usize {
    let Some(first) = chains.first() else {
        return 0;
    };

    (0..first.len())
        .take_while(|index| {
            chains
                .iter()
                .all(|chain| chain.get(*index) == Some(&first[*index]))
        })
        .count()
}
