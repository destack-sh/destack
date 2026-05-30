use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerError, CompilerResult, LowerError};

use crate::lower::{FunctionLowerer, DynamicMember, MethodKey};

/// Dispatch target details for lowering.
pub(crate) enum DispatchTarget {
    /// The dynamic dispatch target details.
    Dynamic {
        /// The dynamic constraint type declaring the dispatch slot.
        constraint: mir::LocalNodeId<mir::Type>,
        /// The dispatch slot for the method.
        slot: mir::DispatchSlot,
        /// The dynamic member signature.
        signature: mir::LocalNodeId<mir::Type>,
    },
    /// Virtual dispatch target details.
    Virtual {
        /// The class type declaring the dispatch slot.
        class: mir::LocalNodeId<mir::Type>,
        /// The dispatch slot for the method.
        slot: mir::DispatchSlot,
        /// The declared target function id for the method.
        function_id: mir::LocalNodeId<mir::Function>,
    },
}

impl FunctionLowerer<'_> {
    /// Resolve a dispatch target for a receiver and target symbol.
    pub(super) fn dispatch_target_for_symbol(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_type_id: dir::LocalTypeId,
        target_symbol: dir::GlobalSymbolId,
        function_id: Option<mir::LocalNodeId<mir::Function>>,
    ) -> CompilerResult<Option<DispatchTarget>> {
        // resolve dynamic constraint and class symbols for the receiver
        let constraint_symbol = self.dynamic_constraint_symbol_for_type(receiver_type_id);
        let class_symbol = self.class_symbol_for_type(receiver_type_id);
        if constraint_symbol.is_none() && class_symbol.is_none() {
            return Ok(None);
        }

        // resolve the dispatch key for this symbol
        let method_key = self.method_key_for_symbol(expression_id, target_symbol)?;

        // prefer dynamic dispatch when available
        if let Some(constraint_symbol) = constraint_symbol {
            let slot = self.dynamic_method_slot(expression_id, constraint_symbol, method_key)?;
            let signature =
                self.dynamic_method_signature(expression_id, constraint_symbol, method_key)?;
            let constraint = self.dynamic_constraint_type(expression_id, constraint_symbol)?;
            return Ok(Some(DispatchTarget::Dynamic {
                constraint,
                slot,
                signature,
            }));
        }

        // resolve virtual dispatch when a slot is present
        if let Some(class_symbol) = class_symbol
            && let Some(slot) = self.virtual_method_slot(class_symbol, method_key)
        {
            let Some(function_id) = function_id else {
                return Err(LowerError::MissingFunction {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    symbol: target_symbol,
                }
                .into());
            };
            let class = self.class_type_for_call(expression_id, receiver_type_id)?;
            return Ok(Some(DispatchTarget::Virtual {
                class,
                slot: mir::DispatchSlot::new(slot),
                function_id,
            }));
        }

        Ok(None)
    }

    /// Resolve the dynamic constraint symbol for a receiver type.
    pub(super) fn dynamic_constraint_symbol_for_type(
        &self,
        receiver_type_id: dir::LocalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        // resolve the receiver type
        let dir_type = self.context.types.get_type(receiver_type_id);
        match dir_type {
            dir::Type::Dynamic(dynamic) => {
                self.interface_constraint_symbol_for_type(dynamic.constraint)
            }
            dir::Type::Form(value) => self.dynamic_constraint_symbol_for_type(value.value),
            _ => None,
        }
    }

    /// Resolve the dynamic constraint symbol carried by one dynamic constraint type.
    pub(super) fn interface_constraint_symbol_for_type(
        &self,
        constraint_type_id: dir::LocalTypeId,
    ) -> Option<dir::GlobalSymbolId> {
        let dir_type = self.context.types.get_type(constraint_type_id);
        match dir_type {
            dir::Type::Named(reference)
                if self
                    .context
                    .symbol_kind_matches(reference.symbol, dir::SymbolKind::Interface) =>
            {
                Some(reference.symbol)
            }
            dir::Type::Form(value) => self.interface_constraint_symbol_for_type(value.value),
            dir::Type::Intersection(intersection) => intersection
                .elements
                .iter()
                .find_map(|element| self.interface_constraint_symbol_for_type(*element)),
            _ => None,
        }
    }

    /// Resolve the dynamic dispatch slot for a method key.
    fn dynamic_method_slot(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        constraint_symbol: dir::GlobalSymbolId,
        method_key: MethodKey,
    ) -> CompilerResult<mir::DispatchSlot> {
        // load dynamic members for dispatch
        let slots = self
            .context
            .dynamic_members_by_symbol
            .get(&constraint_symbol)
            .map(|slots| slots.as_slice())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "dynamic dispatch layout missing".to_string(),
            })
            .map_err(CompilerError::from)?;

        // locate the matching dynamic slot
        let slot_index = slots.iter().position(|slot| {
            let (name, signature, role) = match slot {
                DynamicMember::Getter {
                    name, signature, ..
                } => (*name, *signature, Some(dir::FunctionRole::Getter)),
                DynamicMember::Setter {
                    name, signature, ..
                } => (*name, *signature, Some(dir::FunctionRole::Setter)),
                DynamicMember::Method {
                    name, signature, ..
                } => (*name, *signature, None),
                DynamicMember::Call { signature, .. } => {
                    (self.context.dispatch_call_name, *signature, Some(dir::FunctionRole::Call))
                }
                DynamicMember::Field { .. } => return false,
            };

            name == method_key.name()
                && role == method_key.role()
                && signature == method_key.signature()
        });

        // require a matching slot
        let Some(slot_index) = slot_index else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "dynamic method slot missing".to_string(),
            }
            .into());
        };

        Ok(mir::DynamicTable::slot_for_index(slot_index))
    }

    /// Resolve the dynamic member signature for a method key.
    fn dynamic_method_signature(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        constraint_symbol: dir::GlobalSymbolId,
        method_key: MethodKey,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let slots = self
            .context
            .dynamic_members_by_symbol
            .get(&constraint_symbol)
            .map(|slots| slots.as_slice())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "dynamic dispatch layout missing".to_string(),
            })
            .map_err(CompilerError::from)?;

        for slot in slots {
            let (name, signature, role) = match slot {
                DynamicMember::Getter {
                    name, signature, ..
                } => (*name, *signature, Some(dir::FunctionRole::Getter)),
                DynamicMember::Setter {
                    name, signature, ..
                } => (*name, *signature, Some(dir::FunctionRole::Setter)),
                DynamicMember::Method {
                    name, signature, ..
                } => (*name, *signature, None),
                DynamicMember::Call { signature, .. } => {
                    (self.context.dispatch_call_name, *signature, Some(dir::FunctionRole::Call))
                }
                DynamicMember::Field { .. } => continue,
            };

            if name != method_key.name()
                || role != method_key.role()
                || signature != method_key.signature()
            {
                continue;
            }

            let signature = self
                .context
                .type_lowerer
                .function_signature_types
                .get(&signature)
                .copied()
                .ok_or_else(|| self.missing_type_error(expression_id))
                .map_err(CompilerError::from)?;

            return Ok(signature);
        }

        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                expression_id
                    .into_global_any(self.context.module_id)
                    .into_anchored(Some(self.context.profile)),
            ),
            message: "dynamic method signature missing".to_string(),
        }
        .into())
    }

    /// Resolve the dynamic constraint MIR type for dispatch.
    fn dynamic_constraint_type(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        constraint_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the constraint instance type
        let constraint_type_id =
            self.instance_type_id_for_symbol_or_error(expression_id.into_any(), constraint_symbol)?;

        // resolve the MIR type
        let constraint = self
            .context
            .type_lowerer
            .cached_type(constraint_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;

        Ok(constraint)
    }

    /// Resolve a virtual dispatch key for a symbol.
    fn method_key_for_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<MethodKey> {
        // require a local symbol for now
        if symbol.module_id != self.context.module_id {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "dynamic dispatch across modules is not supported".to_string(),
            }
            .into());
        }

        // resolve the declaration node
        let symbol_entry = self.context.symbols.get_symbol(symbol.local_id);
        let Some(primary) = symbol_entry.declaration else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "member symbol missing declaration".to_string(),
            }
            .into());
        };

        // resolve the member or property node
        let (dynamic_key, signature, node_id) =
            if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
                let member = self.context.dir_tree.get(member_id);
                let dir::Member::Method { key, signature, .. } = member else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "member symbol is not a method".to_string(),
                    }
                    .into());
                };
                (
                    key.as_ref(),
                    signature,
                    member_id.into_global_any(self.context.module_id),
                )
            } else if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
                let property = self.context.dir_tree.get(property_id);
                let dir::Property::Method { key, signature, .. } = property else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "property symbol is not a method".to_string(),
                    }
                    .into());
                };
                (
                    key.as_ref(),
                    signature,
                    property_id.into_global_any(self.context.module_id),
                )
            } else if let Ok(member_id) = primary.local_id.try_into_typed::<dir::TypeMember>() {
                let member = self.context.dir_tree.get(member_id);
                let dir::TypeMember::Method { key, signature, .. } = member else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            expression_id
                                .into_global_any(self.context.module_id)
                                .into_anchored(Some(self.context.profile)),
                        ),
                        message: "type member symbol is not a method".to_string(),
                    }
                    .into());
                };
                (
                    Some(key),
                    signature,
                    member_id.into_global_any(self.context.module_id),
                )
            } else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "member symbol is not a method".to_string(),
                }
                .into());
            };

        // resolve the method name
        let method_name = match (dynamic_key, signature.role) {
            (Some(dir::Key::Name(name)), _) => name.string(),
            (None, Some(dir::FunctionRole::Call)) => self.context.dispatch_call_name,
            (None, Some(dir::FunctionRole::Constructor | dir::FunctionRole::New)) => {
                self.context.dispatch_construct_name
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        expression_id
                            .into_global_any(self.context.module_id)
                            .into_anchored(Some(self.context.profile)),
                    ),
                    message: "method must have a static name".to_string(),
                }
                .into());
            }
        };

        // resolve the signature type id
        let signature_type_id = self.signature_type_id_for_node_or_error(node_id)?;

        Ok(MethodKey::new(
            method_name,
            signature.role,
            signature_type_id,
        ))
    }

    /// Resolve the virtual dispatch slot for a class method symbol.
    fn virtual_method_slot(
        &self,
        class_symbol: dir::GlobalSymbolId,
        method_key: MethodKey,
    ) -> Option<u32> {
        self.context
            .virtual_method_slots_by_key
            .get(&(class_symbol, method_key))
            .copied()
    }

    /// Resolve the class MIR type for dispatch.
    fn class_type_for_call(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        receiver_type_id: dir::LocalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the class symbol
        let class_symbol = self
            .class_symbol_for_type(receiver_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.context.module_id)
                        .into_anchored(Some(self.context.profile)),
                ),
                message: "virtual dispatch requires a class receiver".to_string(),
            })
            .map_err(CompilerError::from)?;

        // resolve the instance type id
        let instance_type_id =
            self.instance_type_id_for_symbol_or_error(expression_id.into_any(), class_symbol)?;

        // resolve the MIR type
        let class = self
            .context
            .type_lowerer
            .cached_type(instance_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))
            .map_err(CompilerError::from)?;

        Ok(class)
    }
}
