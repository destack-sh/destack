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
        // construct the case value for enum and payload-free Tagged members
        if let dir::Type::Variant(variant) = self.node_type(expression)? {
            return self.lower_variant_member(&variant);
        }

        let resolution = self.member_resolution(expression)?;
        let dir::OperationResolution::One(access) = &resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a member read on a union receiver".to_string(),
            }
            .into());
        };

        match &access.target {
            // project a compiler-defined member off the receiver
            dir::MemberTarget::Projection { projection, .. } => {
                let receiver = self.lower_expression(left)?;

                self.lower_member_projection(receiver, projection)
            }
            // call the resolved getter
            dir::MemberTarget::Call(call) => {
                let dir::Call {
                    target:
                        dir::CallTarget::Symbol {
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
        let resolution = self.subscript_resolution(expression)?;
        let dir::OperationResolution::One(subscript) = resolution else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a subscript read on a union receiver".to_string(),
            }
            .into());
        };

        match subscript.target {
            dir::SubscriptTarget::Member(access) => match access.target {
                // read the constant key straight off its field
                dir::MemberTarget::Field(field) => self.lower_field_read(expression, left, &field),
                // find the computed key through the receiver's dynamic table
                dir::MemberTarget::Index(read)
                    if matches!(read.target, dir::IndexTarget::Signature(_)) =>
                {
                    // keyed finds dispatch by name over string domains only
                    let domain = self.lowerer.reduced_type(read.key_type)?;
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
                        dir::CallTarget::Symbol {
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
                        dir::CallTarget::Symbol {
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
        // read structural receivers through their dynamic entries
        let reduced = self.lowerer.reduced_type(field.receiver.ty())?;
        if matches!(self.lowerer.ty(reduced)?, dir::Type::Shape(_)) {
            return self.lower_dynamic_field_read(expression, left, reduced, field.target.key());
        }

        // lower the receiver the resolution selected
        let index = self.member_field_index(field)?;
        let receiver = field.receiver.ty();
        let result_type = self.lower_type(self.node_type_id(expression)?)?;
        let value = self.lower_expression(left)?;
        let value = self.lower_member_receiver(value, &field.receiver)?;

        // load fields through addresses for reference receivers
        if let Some(layer) = self
            .lowerer
            .peel_indirection(receiver, &self.type_substitution)?
        {
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
            dir::Projection::VariantTag { .. } => Ok(self.builder.variant_tag(receiver)),
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("a projected member read through {other:?}"),
            }
            .into()),
        }
    }

    /// Apply the selected receiver adjustments before one member read.
    fn lower_member_receiver(
        &mut self,
        value: mir::Value,
        receiver: &dir::MemberReceiver,
    ) -> CompilerResult<mir::Value> {
        let dir::MemberReceiver::Direct(receiver) = receiver else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a member read through dynamic dispatch".to_string(),
            }
            .into());
        };

        self.lower_receiver_adjustments(value, &receiver.adjustments)
    }

    /// Return the storage index selected by one field resolution.
    pub(in crate::lower) fn member_field_index(
        &self,
        field: &dir::FieldResolution,
    ) -> CompilerResult<u32> {
        // locate storage in the selected receiver
        let stored = match self
            .lowerer
            .peel_indirection(field.receiver.ty(), &self.type_substitution)?
        {
            Some(layer) => layer.stored,
            None => self
                .lowerer
                .peel_owned(field.receiver.ty(), &self.type_substitution)?,
        };

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
