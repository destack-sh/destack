use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, Implementer};
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Erase one concrete value behind an interface constraint.
    pub(in crate::lower) fn lower_existential(
        &mut self,
        value: mir::Value,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // read the constraint from the erased target
        let dynamic = self.lower_type(target)?;
        let mir::Type::Dynamic { constraint, .. } = *self.builder.tree().get(dynamic) else {
            return Err(CompilerError::Internal {
                message: "a value erased outside a dynamic target".to_string(),
            });
        };

        // register the concrete class's constraint entries
        let source = self.lowerer.reduced_type(source)?;
        if let dir::Type::Object(shape) = self.lowerer.ty(source)? {
            let reference = self.lower_type(source)?;
            let concrete = match self.builder.tree().get(reference) {
                mir::Type::Reference { pointee, .. } => *pointee,
                _ => {
                    return Err(CompilerError::Internal {
                        message: "an object class without a reference representation".to_string(),
                    });
                }
            };
            // record the written property names backing field slots
            let written = self
                .lowerer
                .types(source.module_id)?
                .properties(shape.properties)
                .iter()
                .filter_map(|property| match property.key {
                    dir::StaticKey::Name(name) => Some(name),
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.lowerer
                .erasures
                .entry((concrete, constraint))
                .or_insert(Implementer::Object { written });

            return Ok(self.builder.dynamic_bind(dynamic, value, concrete));
        }

        // register the declaring class's constraint entries
        let dir::Type::Application(instance) = self.lowerer.ty(source)? else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a structural existential source".to_string(),
            }
            .into());
        };
        let arguments = self
            .lowerer
            .types(source.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        let concrete = self
            .type_lowerer()
            .lower_nominal(instance.symbol, &arguments)?
            .storage;
        self.lowerer
            .erasures
            .entry((concrete, constraint))
            .or_insert(Implementer::Class(instance.symbol));

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
        let constraint = self
            .type_lowerer()
            .lower_dynamic_constraint(dispatch.constraint)?;
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

    /// Read one structural member through the receiver's dynamic entries.
    pub(in crate::lower) fn lower_dynamic_field_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        contract: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<mir::Value> {
        // find the constraint slot the key declares
        let dir::Type::Shape(shape) = self.lowerer.ty(contract)? else {
            return Err(CompilerError::Internal {
                message: "dynamic field read outside a structural contract".to_string(),
            });
        };
        let slot = self
            .lowerer
            .types(contract.module_id)?
            .properties(shape.properties)
            .iter()
            .position(|property| property.key == key)
            .ok_or_else(|| CompilerError::Internal {
                message: "a read of an undeclared structural member".to_string(),
            })?;

        // read the slot off the erased receiver
        let result_type = self.lower_type(self.node_type_id(expression)?)?;
        let receiver = self.lower_expression(left)?;

        Ok(self
            .builder
            .dynamic_read(receiver, slot as u32, result_type))
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
