use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use destack_core::StringId;

use crate::{LowerError, LowerResult};

use crate::lower::{ModuleLowerer, static_key_to_field_name};

/// A slot in an interface dispatch layout.
#[derive(Debug, Clone)]
pub(crate) enum InterfaceEntry {
    /// A field offset slot for interface property access.
    Field {
        /// The interface field name.
        name: StringId,
        /// The canonical interface dispatch field id.
        field: mir::LocalNodeId<mir::Field>,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
    /// A method slot for interface method dispatch.
    Method {
        /// The interface method name.
        name: StringId,
        /// The signature type id for the interface method.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
}

impl ModuleLowerer<'_> {
    /// Lower and cache interface dispatch slots for an interface symbol.
    pub(crate) fn lower_interface_slots(
        &mut self,
        interface: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<InterfaceEntry>> {
        if let Some(slots) = self.interface_slots_by_symbol.get(&interface) {
            return Ok(slots.clone());
        }

        if self.interface_slots_in_progress.contains(&interface) {
            let anchor = self
                .declaration_ids_for_symbol(interface)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    module: self.module_id,
                    message: "interface slot lowering cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "cycle detected while lowering interface slots".to_string(),
            });
        }

        self.interface_slots_in_progress.insert(interface);
        let slots = self.collect_interface_slots(interface)?;
        self.insert_interface_slots(interface, slots.clone())?;
        self.interface_slots_in_progress.shift_remove(&interface);

        Ok(slots)
    }

    /// Collect interface member slots in declaration order.
    fn collect_interface_slots(
        &mut self,
        interface: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<InterfaceEntry>> {
        // seed the collection state
        let mut slots = Vec::new();
        let mut seen_fields = HashMap::new();
        let mut seen_methods = HashMap::new();
        let mut visited = HashSet::new();

        // collect members across the interface lineage
        self.collect_interface_slots_inner(
            interface,
            &mut slots,
            &mut seen_fields,
            &mut seen_methods,
            &mut visited,
        )?;

        Ok(slots)
    }

    /// Collect interface slots with inheritance ordering.
    fn collect_interface_slots_inner(
        &mut self,
        interface: dir::GlobalSymbolId,
        slots: &mut Vec<InterfaceEntry>,
        seen_fields: &mut HashMap<StringId, dir::LocalTypeId>,
        seen_methods: &mut HashMap<StringId, Vec<dir::LocalTypeId>>,
        visited: &mut HashSet<dir::GlobalSymbolId>,
    ) -> LowerResult<()> {
        // avoid cycles in interface inheritance
        if !visited.insert(interface) {
            return Ok(());
        }

        // visit base interface first
        if let Some(lineage) = self.types.get_lineage_for_symbol(interface)
            && let Some(base) = lineage.extends
        {
            self.collect_interface_slots_inner(base, slots, seen_fields, seen_methods, visited)?;
        }

        // resolve the interface instance type for method stubs
        let declaration_id = self.declaration_ids_for_symbol(interface).first().copied();
        let Some(declaration_id) = declaration_id else {
            return Ok(());
        };
        let anchor = declaration_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));
        let interface_type = self.lower_instance_type(interface, anchor)?;
        let Some(interface_type) = interface_type else {
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "interface missing instance type".to_string(),
            });
        };

        // collect local interface members
        let declaration_ids = self.declaration_ids_for_symbol(interface);
        for declaration_id in declaration_ids {
            // load interface members from the declaration
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                dir::Declaration::Interface(declaration) => &declaration.members,
                _ => continue,
            };

            // scan interface members
            for member_id in members {
                self.collect_interface_member_slots(
                    interface,
                    interface_type,
                    *member_id,
                    slots,
                    seen_fields,
                    seen_methods,
                )?;
            }
        }

        Ok(())
    }

    /// Collect slots for a single interface member.
    fn collect_interface_member_slots(
        &mut self,
        interface_symbol: dir::GlobalSymbolId,
        interface_type: mir::LocalNodeId<mir::Type>,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        slots: &mut Vec<InterfaceEntry>,
        seen_fields: &mut HashMap<StringId, dir::LocalTypeId>,
        seen_methods: &mut HashMap<StringId, Vec<dir::LocalTypeId>>,
    ) -> LowerResult<()> {
        // load member data
        let member = self.dir_tree.get(member_id);

        // handle member slots by kind
        match member {
            dir::TypeMember::Field {
                key, declared_type, ..
            } => {
                // resolve the field name
                let field_name = self.interface_field_name(member_id, *key)?;

                // resolve the field type
                let Some(declared_type) = declared_type else {
                    return Err(LowerError::MissingType {
                        node: member_id.into_global_any(self.module_id).into(),
                    });
                };
                let field_type = self.declared_or_inferred_type_id_for_node_or_error(
                    declared_type.into_global_any(self.module_id),
                )?;

                // only keep the first matching field type
                if let Some(existing) = seen_fields.get(&field_name) {
                    if !self.types_are_equivalent(*existing, field_type) {
                        return Err(LowerError::UnsupportedConstruct {
                            node: member_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "interface field type mismatch".to_string(),
                        });
                    }
                    return Ok(());
                }

                seen_fields.insert(field_name, field_type);
                let dispatch_field =
                    self.interface_dispatch_field(member_id, field_name, field_type)?;
                slots.push(InterfaceEntry::Field {
                    name: field_name,
                    field: dispatch_field,
                    member_id,
                });
            }
            dir::TypeMember::Method {
                key,
                signature,
                symbol,
                ..
            } => {
                // resolve the method name
                let method_name = self.member_dispatch_name_or_error(
                    Some(key),
                    signature.mode,
                    member_id.into_any(),
                )?;

                // resolve the signature type
                let signature_type_id =
                    self.signature_type_id_for_node(member_id.into_global_any(self.module_id))?;

                // detect duplicate method slots
                if let Some(signature_ids) = seen_methods.get_mut(&method_name) {
                    if signature_ids.iter().any(|existing| {
                        self.method_signatures_equivalent(*existing, signature_type_id)
                    }) {
                        return Ok(());
                    }
                    signature_ids.push(signature_type_id);
                } else {
                    seen_methods.insert(method_name, vec![signature_type_id]);
                }

                // create interface method stub when needed
                let method_symbol = symbol.into_global(self.module_id);
                self.lower_interface_method_stub(
                    interface_symbol,
                    interface_type,
                    member_id,
                    Some(key),
                    signature,
                    method_symbol,
                )?;

                slots.push(InterfaceEntry::Method {
                    name: method_name,
                    signature: signature_type_id,
                    member_id,
                });
            }
            dir::TypeMember::CallSignature { .. } | dir::TypeMember::ConstructSignature { .. } => {}
            dir::TypeMember::IndexSignature { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "index signatures are not supported for native lowering".to_string(),
                });
            }
            _ => {}
        }

        Ok(())
    }

    /// Resolve a static interface field name for dispatch metadata.
    fn interface_field_name(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        key: dir::Key,
    ) -> LowerResult<StringId> {
        let Some(key) = self.compiler.static_key_from_key(self.dir_tree, key) else {
            return Err(LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "unsupported dynamic field key in interface layout".to_string(),
            });
        };

        Ok(static_key_to_field_name(&key, &mut self.builder))
    }

    /// Return the canonical interface dispatch field node for a member.
    fn interface_dispatch_field(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        field_name: StringId,
        field_type: dir::LocalTypeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Field>> {
        // reuse an existing field id when available
        let member_key = member_id.id;
        if let Some(field_id) = self.interface_dispatch_fields_by_member.get(&member_key) {
            return Ok(*field_id);
        }

        // lower the field type for stable metadata typing
        let anchor = member_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));
        let field_type = self.lower_type(field_type, anchor)?;

        // create and cache the canonical field node
        let field_id = self.builder.tree_mut().insert(mir::Field {
            name: Some(field_name),
            ty: field_type.into(),
        });
        self.interface_dispatch_fields_by_member
            .insert(member_key, field_id);

        Ok(field_id)
    }
}
