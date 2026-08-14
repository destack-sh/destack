use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionLowerer, GenericInstanceKey};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one receiver expression through its selected adjustments.
    pub(in crate::lower) fn lower_adjusted_receiver(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        adjusted: &dir::AdjustedReceiver,
    ) -> CompilerResult<mir::Value> {
        // take the receiver place directly for a leading borrow
        let (value, rest) = match adjusted.adjustments.as_slice() {
            [dir::ReceiverAdjustment::Borrow { ty }, rest @ ..] => {
                let target = self.lower_type(*ty)?;

                (self.lower_borrowed_place(receiver, target)?, rest)
            }
            rest => (self.lower_expression(receiver)?, rest),
        };

        self.lower_receiver_adjustments(value, rest)
    }

    /// Apply receiver adjustments to one lowered value in order.
    pub(in crate::lower) fn lower_receiver_adjustments(
        &mut self,
        mut value: mir::Value,
        adjustments: &[dir::ReceiverAdjustment],
    ) -> CompilerResult<mir::Value> {
        for adjustment in adjustments {
            value = match adjustment {
                // borrow spilled storage when no source place exists
                dir::ReceiverAdjustment::Borrow { ty } => {
                    let target = self.lower_type(*ty)?;

                    self.spill_borrow(value, target)?
                }
                // read through one reference or pointer receiver
                dir::ReceiverAdjustment::Dereference(dereference) => {
                    self.lower_dereference(value, dereference)?
                }
                // unwrap a newtype value or stored newtype place
                dir::ReceiverAdjustment::NewtypePayload { ty, .. } => {
                    let Some(value_type) = self.builder.value_type(value) else {
                        return Err(CompilerError::Internal {
                            message: "a newtype payload receiver has no lowered type".to_string(),
                        });
                    };

                    // retain the address form of stored receivers
                    match self.builder.tree().get(value_type) {
                        mir::Type::Reference { .. } | mir::Type::Pointer { .. } => {
                            let target = self.lower_type(*ty)?;

                            self.builder.field_addr(value, 0, target)
                        }
                        _ => self.builder.field_get(value, 0),
                    }
                }
                // project a narrowed union value or stored union place
                dir::ReceiverAdjustment::UnionPayload { union, arm, .. } => {
                    let members = self.union_members(*union)?;
                    let Some(index) = members.iter().position(|member| member == arm) else {
                        return Err(CompilerError::Internal {
                            message: "a union payload adjustment selecting an absent arm"
                                .to_string(),
                        });
                    };
                    let Some(value_type) = self.builder.value_type(value) else {
                        return Err(CompilerError::Internal {
                            message: "a union payload receiver has no lowered type".to_string(),
                        });
                    };

                    // retain the address form of stored receivers
                    match self.builder.tree().get(value_type) {
                        mir::Type::Reference { .. } | mir::Type::Pointer { .. } => {
                            let target = self.lower_type(adjustment.ty())?;

                            self.builder
                                .variant_payload_addr(value, index as u32, target)
                        }
                        _ => self.builder.variant_payload(value, index as u32),
                    }
                }
            };
        }

        Ok(value)
    }

    /// Borrow one value through spilled local storage.
    fn spill_borrow(
        &mut self,
        value: mir::Value,
        target: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::Value> {
        let Some(ty) = self.builder.value_type(value) else {
            return Err(CompilerError::Internal {
                message: "a borrowed receiver value has no type".to_string(),
            });
        };
        let local = self.builder.local(ty, mir::Mutability::Immutable);
        self.builder.local_set(local, value);

        Ok(self.builder.local_addr(local, target))
    }

    /// Dereference one receiver value through its selected target.
    fn lower_dereference(
        &mut self,
        value: mir::Value,
        dereference: &dir::Dereference,
    ) -> CompilerResult<mir::Value> {
        match &dereference.target {
            // load through the physical reference
            dir::DereferenceTarget::Direct => {
                let ty = self.lower_type(dereference.ty)?;

                Ok(self.builder.load(value, ty))
            }
            // call the selected method for a protocol dereference
            dir::DereferenceTarget::Call(call) => {
                let dir::CallTarget::Symbol {
                    function,
                    dispatch: dir::FunctionDispatch::Direct,
                } = &call.target
                else {
                    return Err(CompilerError::Internal {
                        message: "a protocol dereference selected no direct method".to_string(),
                    });
                };
                let key = match function.generic_arguments.is_empty() {
                    true => GenericInstanceKey::non_generic(function.symbol),
                    false => {
                        let bindings = self.lowerer.instance_bindings(
                            &function.generic_arguments,
                            &self.type_substitution,
                        )?;
                        let arguments: Vec<_> =
                            bindings.iter().map(|binding| binding.argument).collect();

                        self.generic_instance_key(function.symbol, &arguments)?
                    }
                };
                let target = self.function(&key)?;
                let result = self.builder.call_function(target, vec![value]);

                result.ok_or_else(|| CompilerError::Internal {
                    message: "a protocol dereference returned no value".to_string(),
                })
            }
        }
    }
}
