use tspp_core::StringId;
use tspp_dir as dir;
use tspp_mir as mir;
use tspp_mir::substitute_type;

use crate::lower::function::call::ReceiverUse;
use crate::lower::{FunctionLowerer, GenericScope};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Erase one concrete value behind its constraint's dynamic representation.
    pub(in crate::lower) fn lower_erasure(
        &mut self,
        value: mir::Value,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Value> {
        // reject narrowing back out of an erased representation
        let dynamic = self.lower_type(target)?;
        let mir::Type::Dynamic { .. } = *self.builder.tree().get(dynamic) else {
            return Err(self.unsupported("narrowing an erased value to its payload"));
        };

        // register the implementer pair behind this erasure for the dispatch tables
        let scope = self.scope.erased();
        self.lower
            .check_erasure(self.builder.tree_mut(), source, target, &scope)?;

        // bind the payload at its own lowered type, the key every implementer table shares
        let concrete = self.lower_type(source)?;
        let concrete = mir::erase_lifetimes(self.builder.tree_mut(), concrete);

        Ok(self.builder.dynamic_bind(dynamic, value, concrete))
    }

    /// Lower one method call dispatched through an erased receiver.
    pub(in crate::lower) fn lower_dynamic_call(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        dispatch: &dir::DynamicDispatch,
    ) -> CompilerResult<Option<mir::Value>> {
        // read the receiver and member name from the callee
        let (dir::Expression::Call { left: callee, .. }
        | dir::Expression::New { left: callee, .. }) = *self.source().tree().get(expression)
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
            return Err(self.unsupported("a dynamic call without a member callee"));
        };
        let Some(name) = name else {
            return Err(CompilerError::Internal {
                message: "a dispatched call without a member name".to_string(),
            });
        };

        let receiver =
            self.lower_adjusted_receiver(receiver, &dispatch.receiver, false, ReceiverUse::Value)?;

        self.lower_dynamic_slot_call(receiver, name, dispatch, resolution)
    }

    /// Call one constraint slot by name on an adjusted erased receiver value.
    pub(in crate::lower) fn lower_dynamic_slot_call(
        &mut self,
        receiver: mir::Value,
        name: StringId,
        dispatch: &dir::DynamicDispatch,
        call: &dir::Call,
    ) -> CompilerResult<Option<mir::Value>> {
        // select the constraint's declared slot and signature
        let constraint = self.lower_constraint(dispatch.constraint)?;
        let Some((shape, applied)) = self.lower.dynamic_shape(self.builder.tree(), constraint)
        else {
            return Err(CompilerError::Internal {
                message: "a dynamic call without a registered constraint shape".to_string(),
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
                message: "a dynamic call of an undeclared constraint member".to_string(),
            });
        };

        // close the slot's signature at the interface's arguments and the member's parameters
        let (arguments, chain) = self.dynamic_slot_arguments(receiver, &applied, call)?;
        let signature = substitute_type(self.builder.tree_mut(), signature, &arguments);
        let mut parameters = self.signature_parameters(signature)?;
        let mut result = self.builder.signature_result(signature);
        if let Some(chain) = &chain {
            self.instantiate_signature(
                &mut parameters,
                &mut result,
                chain,
                &call.regions,
                &[],
                None,
            )?;
        }
        let values = self.lower_call_arguments(&call.arguments, &parameters, &[])?;

        Ok(self.builder.call(
            mir::Callee::Dynamic {
                receiver,
                constraint,
                slot: mir::DispatchSlot(slot as u32),
            },
            signature,
            values,
            result,
        ))
    }

    /// Return the scope the dispatching interface's shape indexes its slots under.
    fn constraint_scope(&mut self, constraint: dir::GlobalTypeId) -> CompilerResult<GenericScope> {
        let Some(symbol) = self.lower.nominal_symbol(constraint)? else {
            return Err(self.internal("a dynamic dispatch through a non-nominal constraint"));
        };

        match self
            .lower
            .definition(symbol)?
            .and_then(|definition| definition.template())
        {
            Some(template) => {
                GenericScope::for_declaration(self.lower, template.into_global(symbol.module_id))
            }
            None => Ok(GenericScope::default()),
        }
    }

    /// Return the arguments one dispatched slot closes at: interface, receiver, then member.
    fn dynamic_slot_arguments(
        &mut self,
        receiver: mir::Value,
        applied: &[mir::GenericArgument],
        call: &dir::Call,
    ) -> CompilerResult<(Vec<mir::GenericArgument>, Option<GenericScope>)> {
        // a symbol-free slot closes at the interface's arguments alone
        let dir::CallableTarget::Dynamic {
            dispatch,
            function: dir::DynamicFunction::Symbol(symbol),
            generic_arguments,
        } = &call.target
        else {
            return Ok((applied.to_vec(), None));
        };

        // place the interface's arguments first, its receiver bound to the erased receiver
        let interface = self.constraint_scope(dispatch.constraint)?;
        let chain = self
            .lower
            .dispatch_slot_scope(self.builder.tree(), &interface, *symbol)?;
        let erased = self.value_representation(receiver)?;
        let mut arguments = vec![None; chain.count() as usize];
        for (index, argument) in applied.iter().enumerate() {
            arguments[index] = Some(argument.clone());
        }
        if let Some(index) = chain.receiver {
            arguments[index as usize] = Some(mir::GenericArgument::Type(erased));
        }

        // bind the member's own parameters at the call's generic arguments and regions
        for binding in generic_arguments.iter().chain(call.regions.iter()) {
            let Some(index) = chain.parameters.get(&binding.parameter) else {
                continue;
            };
            let argument = &mut arguments[*index as usize];
            if argument.is_none() {
                *argument = Some(self.lower_generic_argument(binding.argument)?);
            }
        }

        // close the member's dependents at the erased receiver
        for dependent in chain.dependents.values() {
            let tree = self.builder.tree_mut();
            let mut types = self.lower.type_lowerer(tree, &self.scope);
            types.this_type = Some(erased);
            arguments[dependent.index as usize] = Some(types.lower_generic_argument(dependent.ty)?);
        }

        // require an argument at every index
        let mut closed = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.into_iter().enumerate() {
            let Some(argument) = argument else {
                let slot = chain
                    .parameters
                    .iter()
                    .find(|(_, slot)| **slot == index as u32)
                    .map(|(parameter, _)| self.lower.format_parameter_name(*parameter))
                    .transpose()?
                    .unwrap_or_else(|| format!("slot {index}"));

                return Err(CompilerError::Internal {
                    message: format!(
                        "a dynamic call to '{}' without an argument for '{slot}'",
                        self.lower.symbol_path(*symbol)?
                    ),
                });
            };
            closed.push(argument);
        }

        Ok((closed, Some(chain)))
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
            return Err(self.unsupported("a computed member read through dynamic dispatch"));
        };

        // select the slot behind the name in the constraint shape
        let receiver =
            self.lower_adjusted_receiver(left, &dispatch.receiver, false, ReceiverUse::Value)?;
        let constraint = self.lower_constraint(dispatch.constraint)?;
        let Some((shape, applied)) = self.lower.dynamic_shape(self.builder.tree(), constraint)
        else {
            return Err(CompilerError::Internal {
                message: "a dynamic read without a registered constraint shape".to_string(),
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

        // read the slot at its signature under the interface's arguments
        let signature = match (applied.is_empty(), signature) {
            (false, Some(signature)) => Some(substitute_type(
                self.builder.tree_mut(),
                signature,
                &applied,
            )),
            (_, signature) => signature,
        };
        let result_type = self.lower_type(self.node_type_id(expression)?)?;

        // call a getter slot at its declared signature
        match signature {
            Some(signature) => {
                let value = self.builder.call(
                    mir::Callee::Dynamic {
                        receiver,
                        constraint,
                        slot: mir::DispatchSlot(slot as u32),
                    },
                    signature,
                    Vec::new(),
                    result_type,
                );

                value.ok_or_else(|| CompilerError::Internal {
                    message: "a void result from a dispatched getter".to_string(),
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
        let receiver = self.lower_value(left)?;
        let key = self.lower_value(key)?;

        Ok(self.builder.dynamic_find(receiver, key, result_type))
    }
}
