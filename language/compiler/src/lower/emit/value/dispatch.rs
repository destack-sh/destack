use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::emit::FunctionContext;
use crate::lower::table::VirtualMethodKey;
use crate::lower::table::interface::InterfaceSlot;

/// Dispatch target details for lowering.
pub(super) enum DispatchTarget {
    /// Interface dispatch target details.
    Interface {
        /// The declaring interface type id.
        declaring_type: mir::LocalNodeId<mir::Type>,
        /// The itab slot id for the method.
        slot_id: u32,
        /// The declared target function id for the method.
        function_id: mir::LocalNodeId<mir::Function>,
    },
    /// Virtual dispatch target details.
    Virtual {
        /// The declaring class type id.
        declaring_type: mir::LocalNodeId<mir::Type>,
        /// The vtable slot id for the method.
        slot_id: u32,
        /// The declared target function id for the method.
        function_id: mir::LocalNodeId<mir::Function>,
    },
}

impl FunctionContext<'_> {
    /// Resolve a dispatch target for a receiver and target symbol.
    pub(super) fn dispatch_target_for_symbol(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
        target_symbol: GlobalSymbolId,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> LowerResult<Option<DispatchTarget>> {
        // resolve interface and class symbols for the receiver
        let interface_symbol = self.interface_symbol_for_type(receiver_type_id);
        let class_symbol = self.class_symbol_for_type(receiver_type_id);
        if interface_symbol.is_none() && class_symbol.is_none() {
            return Ok(None);
        }

        // resolve the dispatch key for this symbol
        let method_key = self.method_key_for_symbol(expression_id, target_symbol)?;

        // prefer interface dispatch when available
        if let Some(interface_symbol) = interface_symbol {
            let slot_id =
                self.interface_method_slot_id(expression_id, interface_symbol, method_key)?;
            let declaring_type = self.interface_declaring_type(expression_id, interface_symbol)?;
            return Ok(Some(DispatchTarget::Interface {
                declaring_type,
                slot_id,
                function_id,
            }));
        }

        // resolve virtual dispatch when a slot is present
        if let Some(class_symbol) = class_symbol
            && let Some(slot_id) = self.virtual_method_slot_id(class_symbol, method_key)
        {
            let declaring_type =
                self.declaring_type_for_virtual_call(expression_id, receiver_type_id)?;
            return Ok(Some(DispatchTarget::Virtual {
                declaring_type,
                slot_id,
                function_id,
            }));
        }

        Ok(None)
    }

    /// Resolve the interface symbol for a receiver type.
    pub(super) fn interface_symbol_for_type(
        &self,
        receiver_type_id: dir::LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // resolve the receiver type
        let dir_type = self.env.types.get_type(receiver_type_id);
        match dir_type {
            dir::Type::Reference { symbol, .. } if symbol.ty() == dir::SymbolType::Interface => {
                Some(*symbol)
            }
            dir::Type::Value { value } => self.interface_symbol_for_type(*value),
            dir::Type::Intersection { elements } => elements
                .iter()
                .find_map(|element| self.interface_symbol_for_type(*element)),
            _ => None,
        }
    }

    /// Resolve the interface dispatch slot id for a method key.
    fn interface_method_slot_id(
        &self,
        expression_id: LocalNodeId<Expression>,
        interface_symbol: GlobalSymbolId,
        method_key: VirtualMethodKey,
    ) -> LowerResult<u32> {
        // load interface slots for dispatch
        let slots = self
            .env
            .interface_slots_by_symbol
            .get(&interface_symbol)
            .map(|slots| slots.as_slice())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface dispatch layout missing".to_string(),
            })?;

        // locate the matching method slot
        let slot_index = slots.iter().position(|slot| {
            let InterfaceSlot::Method {
                name, signature, ..
            } = slot
            else {
                return false;
            };
            *name == method_key.name()
                && dir::are_types_equal(*signature, method_key.signature(), self.env.types)
        });

        // require a matching slot
        let Some(slot_index) = slot_index else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface method slot missing".to_string(),
            });
        };

        Ok(slot_index as u32 + 1)
    }

    /// Resolve the declaring interface type id for dispatch.
    fn interface_declaring_type(
        &self,
        expression_id: LocalNodeId<Expression>,
        interface_symbol: GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the declaring interface type id
        let interface_type_id =
            self.instance_type_id_for_symbol_or_error(expression_id.into_any(), interface_symbol)?;

        // resolve the declaring mir type
        let declaring_type = self
            .env
            .type_lowerer
            .cached_type(interface_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        Ok(declaring_type)
    }

    /// Resolve a virtual dispatch key for a symbol.
    fn method_key_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
    ) -> LowerResult<VirtualMethodKey> {
        // require a local symbol for now
        if symbol.module_id != self.env.module_id {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "interface dispatch across modules is not supported".to_string(),
            });
        }

        // resolve the primary declaration node
        let symbol_entry = self.env.symbols.get_symbol(symbol.local_id);
        let Some(primary) = symbol_entry.primary_declaration else {
            return Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "member symbol missing declaration".to_string(),
            });
        };

        // resolve the member or property node
        let (dynamic_key, signature, node_id) =
            if let Ok(member_id) = primary.local_id.try_into_typed::<dir::Member>() {
                let member = self.env.dir_tree.get(member_id);
                let dir::Member::Method { key, signature, .. } = member else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "member symbol is not a method".to_string(),
                    });
                };
                (
                    key,
                    signature,
                    member_id.into_global_any(self.env.module_id),
                )
            } else if let Ok(property_id) = primary.local_id.try_into_typed::<dir::Property>() {
                let property = self.env.dir_tree.get(property_id);
                let dir::Property::Method { key, signature, .. } = property else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id
                            .into_global_any(self.env.module_id)
                            .into_anchored(Some(self.env.profile)),
                        message: "property symbol is not a method".to_string(),
                    });
                };
                (
                    key,
                    signature,
                    property_id.into_global_any(self.env.module_id),
                )
            } else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "member symbol is not a method".to_string(),
                });
            };

        // resolve the method name
        let method_name = match (dynamic_key, signature.mode) {
            (Some(dir::DynamicKey::Name(name)), _) => *name,
            (None, Some(dir::FunctionMode::Call)) => self.env.dispatch_call_name,
            (None, Some(dir::FunctionMode::Constructor | dir::FunctionMode::New)) => {
                self.env.dispatch_construct_name
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.env.module_id)
                        .into_anchored(Some(self.env.profile)),
                    message: "method must have a static name".to_string(),
                });
            }
        };

        // resolve the signature type id
        let signature_type_id = self.signature_type_id_for_node_or_error(node_id)?;

        Ok(VirtualMethodKey::new(
            method_name,
            signature.mode,
            signature_type_id,
        ))
    }

    /// Resolve the vtable slot id for a virtual method symbol.
    fn virtual_method_slot_id(
        &self,
        class_symbol: GlobalSymbolId,
        method_key: VirtualMethodKey,
    ) -> Option<u32> {
        self.env
            .virtual_method_slots_by_key
            .get(&(class_symbol, method_key))
            .copied()
    }

    /// Resolve the declaring MIR type for a virtual call.
    fn declaring_type_for_virtual_call(
        &self,
        expression_id: LocalNodeId<Expression>,
        receiver_type_id: dir::LocalTypeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the declaring class symbol
        let class_symbol = self
            .class_symbol_for_type(receiver_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.env.module_id)
                    .into_anchored(Some(self.env.profile)),
                message: "virtual dispatch requires a class receiver".to_string(),
            })?;

        // resolve the instance type id
        let instance_type_id =
            self.instance_type_id_for_symbol_or_error(expression_id.into_any(), class_symbol)?;

        // resolve the declaring mir type
        let mir_type = self
            .env
            .type_lowerer
            .cached_type(instance_type_id)
            .ok_or_else(|| self.missing_type_error(expression_id))?;

        Ok(mir_type)
    }
}
