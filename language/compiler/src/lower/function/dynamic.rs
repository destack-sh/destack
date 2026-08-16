use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Erase one concrete value behind an interface constraint.
    pub(in crate::lower) fn lower_existential(
        &mut self,
        value: mir::Value,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // require the erased target's dynamic carrier
        let dynamic = self.lower_type(target)?;
        let mir::Type::Dynamic { .. } = *self.builder.tree().get(dynamic) else {
            return Err(CompilerError::Internal {
                message: "a value erased outside a dynamic target".to_string(),
            });
        };

        // bind object types behind their managed reference representation
        if let dir::Type::Object(_) = self.lowerer.ty(source)? {
            let reference = self.lower_type(source)?;
            let concrete = match self.builder.tree().get(reference) {
                mir::Type::Reference { pointee, .. } => *pointee,
                _ => {
                    return Err(CompilerError::Internal {
                        message: "an object class without a reference representation".to_string(),
                    });
                }
            };

            return Ok(self.builder.dynamic_bind(dynamic, value, concrete));
        }

        // bind classes at their declared nominal storage
        let dir::Type::Application(_) = self.lowerer.ty(source)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural existential source".to_string(),
            }
            .into());
        };
        let concrete = self.lower_nominal(source)?.storage;

        Ok(self.builder.dynamic_bind(dynamic, value, concrete))
    }

    /// Lower one method call dispatched through an erased receiver.
    pub(in crate::lower) fn lower_dynamic_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        dispatch: &dir::DynamicDispatch,
    ) -> CompilerResult<Option<mir::Value>> {
        // read the receiver and dispatch slot from the callee member
        let dir::Expression::Call { left: callee, .. } = *self.source().tree().get(expression)
        else {
            return Err(CompilerError::Internal {
                message: "a dispatched call outside a call expression".to_string(),
            });
        };
        let dir::Expression::Member {
            left: receiver,
            name,
            ..
        } = *self.source().tree().get(callee)
        else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a dynamic call without a member callee".to_string(),
            }
            .into());
        };
        let Some(name) = name else {
            return Err(CompilerError::Internal {
                message: "a dispatched call without a member name".to_string(),
            });
        };

        // select the constraint's declared slot and signature
        let receiver = self.lower_adjusted_receiver(receiver, &dispatch.receiver)?;
        let constraint = self.lower_constraint(dispatch.constraint)?;
        let Some(shape) = self.lowerer.dynamic_shapes.get(&constraint) else {
            return Err(CompilerError::Internal {
                message: "a dynamic call reached an unregistered constraint shape".to_string(),
            });
        };
        let Some((slot, signature)) =
            shape
                .slots
                .iter()
                .enumerate()
                .find_map(|(index, slot)| match slot {
                    mir::DynamicSlot::Function {
                        name: Some(slot_name),
                        signature,
                    } if *slot_name == name => Some((index, *signature)),
                    _ => None,
                })
        else {
            return Err(CompilerError::Internal {
                message: "a dispatch to an undeclared constraint member".to_string(),
            });
        };

        // call the selected slot at its declared signature
        let values = self.lower_provided_arguments(resolution)?;

        Ok(self.builder.call(
            mir::Callee::Dynamic {
                receiver,
                constraint: mir::TypeId::from(constraint),
                slot: mir::DispatchSlot(slot as u32),
            },
            mir::TypeId::from(signature),
            values,
        ))
    }

    /// Read one member through an erased receiver's dispatch table.
    pub(in crate::lower) fn lower_dynamic_member_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        field: &dir::FieldResolution,
        dispatch: &dir::DynamicDispatch,
    ) -> CompilerResult<mir::Value> {
        // read the member name the constraint slot declares
        let dir::StaticKey::Name(name) = field.target.key() else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a computed member read through dynamic dispatch".to_string(),
            }
            .into());
        };

        // select the slot behind the name in the constraint shape
        let receiver = self.lower_adjusted_receiver(left, &dispatch.receiver)?;
        let constraint = self.lower_constraint(dispatch.constraint)?;
        let Some(shape) = self.lowerer.dynamic_shapes.get(&constraint) else {
            return Err(CompilerError::Internal {
                message: "a dynamic read reached an unregistered constraint shape".to_string(),
            });
        };
        let Some((slot, signature)) =
            shape
                .slots
                .iter()
                .enumerate()
                .find_map(|(index, slot)| match slot {
                    mir::DynamicSlot::Field {
                        name: slot_name, ..
                    } if *slot_name == name => Some((index, None)),
                    mir::DynamicSlot::Function {
                        name: Some(slot_name),
                        signature,
                    } if *slot_name == name => Some((index, Some(*signature))),
                    _ => None,
                })
        else {
            return Err(CompilerError::Internal {
                message: "a dynamic read of an undeclared constraint member".to_string(),
            });
        };
        let result_type = self.lower_type(self.node_type_id(expression)?)?;

        // call a getter slot at its declared signature
        match signature {
            Some(signature) => {
                let value = self.builder.call(
                    mir::Callee::Dynamic {
                        receiver,
                        constraint: mir::TypeId::from(constraint),
                        slot: mir::DispatchSlot(slot as u32),
                    },
                    mir::TypeId::from(signature),
                    Vec::new(),
                );

                value.ok_or_else(|| CompilerError::Internal {
                    message: "the dispatched getter returned no value".to_string(),
                })
            }
            // read a field slot through the dispatch table
            None => {
                Ok(self
                    .builder
                    .dynamic_read(receiver, mir::DispatchSlot(slot as u32), result_type))
            }
        }
    }

    /// Find one computed key through the receiver's dynamic table.
    pub(in crate::lower) fn lower_dynamic_signature_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        key: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let result_type = self.lower_type(self.node_type_id(expression)?)?;
        let receiver = self.lower_expression(left)?;
        let key = self.lower_expression(key)?;

        Ok(self.builder.dynamic_find(receiver, key, result_type))
    }
}
