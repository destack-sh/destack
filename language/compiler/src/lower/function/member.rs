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

        // read the single access the checker selected for this member
        let resolution = self.member_decision(expression)?;
        let dir::OperationResolution::One(access) = &resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a member read on a union receiver".to_string(),
            }
            .into());
        };

        match &access.target {
            // project a compiler-defined member off the receiver
            dir::MemberTarget::Projection {
                receiver,
                projection,
                ..
            } => {
                let receiver = self.lower_adjusted_receiver(left, receiver)?;

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
                        anchor: self.lowerer.module.into(),
                        construct: "a dynamic property read".to_string(),
                    }
                    .into());
                };
                let value = self.lower_function_target_call(left, call, function, None)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "the getter returned no value".to_string(),
                })
            }
            // read the resolved field
            dir::MemberTarget::Field(field) => self.lower_field_read(expression, left, field),
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a member read through {other:?}"),
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
        let resolution = self.subscript_decision(expression)?;
        let dir::OperationResolution::One(subscript) = resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a subscript read on a union receiver".to_string(),
            }
            .into());
        };

        match subscript.target {
            dir::SubscriptTarget::Member(access) => match access.target {
                // project a compiler-defined member off the receiver
                dir::MemberTarget::Projection {
                    receiver,
                    projection,
                    ..
                } => {
                    let receiver = self.lower_adjusted_receiver(left, &receiver)?;

                    // evaluate the computed singleton key
                    if let Some(index) = index {
                        self.lower_const_expression(index)?;
                    }

                    self.lower_member_projection(receiver, &projection)
                }
                // read the constant key straight off its field
                dir::MemberTarget::Field(field) => {
                    let value = self.lower_member_receiver(left, &field.receiver)?;

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
                        self.lowerer.ty(domain)?,
                        dir::Type::Primitive(dir::PrimitiveType::String)
                    ) {
                        return Err(LowerError::Unsupported {
                            anchor: self.lowerer.module.into(),
                            construct: "a non-string signature key domain".to_string(),
                        }
                        .into());
                    }
                    let key = index.ok_or_else(|| CompilerError::Internal {
                        message: "a signature subscript read has no key expression".to_string(),
                    })?;

                    self.lower_dynamic_signature_read(expression, left, key)
                }
                other => Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("a subscript read through {other:?}"),
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
                        anchor: self.lowerer.module.into(),
                        construct: "a dynamic subscript read".to_string(),
                    }
                    .into());
                };
                let value = self.lower_function_target_call(left, &call, function, None)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "the subscript read returned no value".to_string(),
                })
            }
            // load the value behind the address the Index protocol returns
            dir::SubscriptTarget::Index(read) => {
                if read.missing.is_some() {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "an Index read with a missing result".to_string(),
                    }
                    .into());
                }
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
                        anchor: self.lowerer.module.into(),
                        construct: "a dynamic subscript read".to_string(),
                    }
                    .into());
                };
                let value = self.lower_function_target_call(left, &call, function, None)?;
                let value = value.ok_or_else(|| CompilerError::Internal {
                    message: "the subscript read returned no value".to_string(),
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
    ) -> CompilerResult<mir::Value> {
        // read erased receivers through their dispatch table
        if let dir::MemberReceiver::Dynamic(dispatch) = &field.receiver {
            let dispatch = dispatch.clone();

            return self.lower_dynamic_member_read(expression, left, field, &dispatch);
        }

        // lower the receiver the resolution selected
        let value = self.lower_member_receiver(left, &field.receiver)?;

        self.lower_field_value(expression, value, field)
    }

    /// Lower one field read from its already adjusted receiver value.
    fn lower_field_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        value: mir::Value,
        field: &dir::FieldResolution,
    ) -> CompilerResult<mir::Value> {
        // resolve the selected storage field and result type
        let receiver = field.receiver.ty();
        let index = self.member_field_index(field)?;
        let result_type = self.lower_type(self.node_type_id(expression)?)?;

        // load fields through addresses for reference receivers
        if let Some(layer) = self.lowerer.peel_indirection(receiver)? {
            let address = self.emit_field_address(value, index, result_type, layer.access);

            return Ok(self.builder.load(address, result_type));
        }

        Ok(self.builder.field_get(value, index))
    }

    /// Lower one payload-free variant member to its case construction.
    fn lower_variant_member(&mut self, variant: &dir::VariantType) -> CompilerResult<mir::Value> {
        // materialize the owner carrier and select the declared case
        let dir::Type::Application(owner) = self.lowerer.ty(variant.owner)? else {
            return Err(CompilerError::Internal {
                message: "a variant without its owner instance".to_string(),
            });
        };
        let ty = self.lower_type(variant.owner)?;
        let case = self
            .lowerer
            .variant_position(owner.symbol, variant.variant)?;

        Ok(self.builder.variant_new(ty, case, None))
    }

    /// Lower one compiler-defined member projection.
    fn lower_member_projection(
        &mut self,
        receiver: mir::Value,
        projection: &dir::Projection,
    ) -> CompilerResult<mir::Value> {
        match projection {
            dir::Projection::Discriminant {
                union, cases, ty, ..
            } => self.lower_discriminant_value(receiver, *union, cases, *ty),
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a projected member read through {other:?}"),
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
                message: "a discriminant projection has no reachable cases".to_string(),
            });
        }
        let source_members = self.union_members(union)?;

        // read the result's literal arms when it remains an indexed union
        let result = self.lowerer.ty(ty)?;
        let is_singleton = result.singleton_literal().is_some();
        let result_members = match result {
            dir::Type::Union(result) => Some(
                self.lowerer
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
                    message: "a discriminant projection selects an absent union arm".to_string(),
                });
            };
            let mut result_index = None;
            if let Some(result_members) = &result_members {
                for (index, member) in result_members.iter().enumerate() {
                    let value = self.lowerer.ty(*member)?.singleton_literal();
                    if value == Some(case.value) {
                        result_index = Some(index as u32);
                        break;
                    }
                }
                if result_index.is_none() {
                    return Err(CompilerError::Internal {
                        message: "a discriminant projection value is absent from its result type"
                            .to_string(),
                    });
                }
            };

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
        let Some(receiver_type) = self.builder.value_type(receiver) else {
            return Err(CompilerError::Internal {
                message: "a discriminant receiver has no lowered type".to_string(),
            });
        };

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
    ) -> CompilerResult<mir::Value> {
        let dir::MemberReceiver::Direct(receiver) = receiver else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a member read through dynamic dispatch".to_string(),
            }
            .into());
        };

        self.lower_adjusted_receiver(expression, receiver)
    }

    /// Return the storage index selected by one field resolution.
    pub(in crate::lower) fn member_field_index(
        &self,
        field: &dir::FieldResolution,
    ) -> CompilerResult<u32> {
        // locate storage in the selected receiver
        let mut stored = match self.lowerer.peel_indirection(field.receiver.ty())? {
            Some(layer) => layer.stored,
            None => self.lowerer.peel_owned(field.receiver.ty())?,
        };

        // follow transparent alias and newtype definitions, re-peeling their owners
        while let dir::Type::Application(instance) = self.lowerer.ty(stored)? {
            let defined = match self.lowerer.definition(instance.symbol)? {
                Some(dir::Definition::TypeAlias(alias)) => alias.value,
                Some(dir::Definition::Newtype(newtype)) => newtype.backing,
                _ => break,
            };
            stored = self.lowerer.peel_owned(defined)?;
        }

        // find the field's position in the storage the receiver declares
        let index = match self.lowerer.ty(stored)? {
            dir::Type::Tuple(_) => match field.target.key() {
                dir::StaticKey::Index(index) => Some(index),
                _ => None,
            },
            dir::Type::Application(instance) => {
                let fields = self.lowerer.nominal_fields(instance.symbol)?;

                fields.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { symbol, .. } => stored.symbol == symbol.local_id,
                })
            }
            dir::Type::Object(shape) => {
                let properties = self
                    .lowerer
                    .types(stored.module_id)?
                    .properties(shape.properties);

                properties.iter().position(|stored| match field.target {
                    dir::FieldTarget::Structural { key, .. } => stored.key == key,
                    dir::FieldTarget::Member { .. } => false,
                })
            }
            _ => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a member read on a structural receiver".to_string(),
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
}
