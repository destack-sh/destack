use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use destack_core::StringId;

use crate::{LowerError, LowerResult};

use crate::lower::{ModuleLowerer, static_key_to_field_name};

/// A member in a dynamic dispatch layout.
#[derive(Debug, Clone)]
pub(crate) enum DynamicMember {
    /// Field offset member.
    Field {
        /// The field name.
        name: StringId,
        /// The canonical field id.
        field: mir::LocalNodeId<mir::Field>,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
    /// Getter member.
    Getter {
        /// The getter name.
        name: StringId,
        /// The signature type id.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
    /// Setter member.
    Setter {
        /// The setter name.
        name: StringId,
        /// The signature type id.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
    /// Method member.
    Method {
        /// The method name.
        name: StringId,
        /// The signature type id.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
    /// Call signature member.
    Call {
        /// The signature type id.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: dir::LocalNodeId<dir::TypeMember>,
    },
}

impl ModuleLowerer<'_> {
    /// Lower and cache dynamic members for a dynamic constraint.
    pub(crate) fn lower_dynamic_members(
        &mut self,
        constraint: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<DynamicMember>> {
        if let Some(slots) = self.dynamic_members_by_symbol.get(&constraint) {
            return Ok(slots.clone());
        }

        if self.dynamic_members_in_progress.contains(&constraint) {
            let anchor = self
                .declaration_ids_for_symbol(constraint)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: "dynamic member lowering cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "cycle detected while lowering dynamic members".to_string(),
            }
            .into());
        }

        self.dynamic_members_in_progress.insert(constraint);
        let slots = self.collect_dynamic_members(constraint)?;
        self.insert_dynamic_members(constraint, slots.clone())?;
        self.dynamic_members_in_progress.shift_remove(&constraint);

        Ok(slots)
    }

    /// Collect dynamic members in declaration order.
    fn collect_dynamic_members(
        &mut self,
        constraint: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<DynamicMember>> {
        // seed the collection state
        let mut slots = Vec::new();
        let mut seen_fields = HashMap::new();
        let mut seen_methods = HashMap::new();
        let mut visited = HashSet::new();

        // collect members across the dynamic constraint lineage
        self.collect_dynamic_members_inner(
            constraint,
            &mut slots,
            &mut seen_fields,
            &mut seen_methods,
            &mut visited,
        )?;

        Ok(slots)
    }

    /// Collect dynamic members with inheritance ordering.
    fn collect_dynamic_members_inner(
        &mut self,
        constraint: dir::GlobalSymbolId,
        slots: &mut Vec<DynamicMember>,
        seen_fields: &mut HashMap<StringId, dir::LocalTypeId>,
        seen_methods: &mut HashMap<StringId, Vec<dir::LocalTypeId>>,
        visited: &mut HashSet<dir::GlobalSymbolId>,
    ) -> LowerResult<()> {
        // avoid cycles in dynamic constraint inheritance
        if !visited.insert(constraint) {
            return Ok(());
        }

        // collect local dynamic members
        let declaration_ids = self.declaration_ids_for_symbol(constraint);
        for declaration_id in declaration_ids {
            // load dynamic members from the declaration
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                dir::Declaration::Interface(declaration) => &declaration.members,
                _ => continue,
            };

            // scan dynamic members
            for member_id in members {
                self.collect_dynamic_member(*member_id, slots, seen_fields, seen_methods)?;
            }
        }

        Ok(())
    }

    /// Collect dynamic metadata for a single dynamic member.
    fn collect_dynamic_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        slots: &mut Vec<DynamicMember>,
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
                let field_name = self.dynamic_field_name(member_id, *key)?;

                // resolve the field type
                let Some(declared_type) = declared_type else {
                    return Err(LowerError::MissingType {
                        anchor: self
                            .diagnostic_anchor(member_id.into_global_any(self.module_id).into()),
                    }
                    .into());
                };
                let field_type = self.declared_or_inferred_type_id_for_node_or_error(
                    declared_type.into_global_any(self.module_id),
                )?;

                // only keep the first matching field type
                if let Some(existing) = seen_fields.get(&field_name) {
                    if !self.types_are_equivalent(*existing, field_type) {
                        return Err(LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(
                                member_id
                                    .into_global_any(self.module_id)
                                    .into_anchored(Some(self.profile)),
                            ),
                            message: "dynamic field type mismatch".to_string(),
                        }
                        .into());
                    }
                    return Ok(());
                }

                seen_fields.insert(field_name, field_type);
                let dispatch_field =
                    self.dynamic_dispatch_field(member_id, field_name, field_type)?;
                slots.push(DynamicMember::Field {
                    name: field_name,
                    field: dispatch_field,
                    member_id,
                });
            }
            dir::TypeMember::Method { key, signature, .. } => {
                // resolve the member name
                let member_name = self.member_dispatch_name_or_error(
                    Some(key),
                    signature.role,
                    member_id.into_any(),
                )?;

                // resolve the signature type
                let signature_type_id =
                    self.signature_type_id_for_node(member_id.into_global_any(self.module_id))?;

                // detect duplicate callable members
                if let Some(signature_ids) = seen_methods.get_mut(&member_name) {
                    if signature_ids.iter().any(|existing| {
                        self.method_signatures_equivalent(*existing, signature_type_id)
                    }) {
                        return Ok(());
                    }
                    signature_ids.push(signature_type_id);
                } else {
                    seen_methods.insert(member_name, vec![signature_type_id]);
                }

                // lower accessors and methods as explicit dynamic members
                match signature.role {
                    Some(dir::FunctionRole::Getter) => slots.push(DynamicMember::Getter {
                        name: member_name,
                        signature: signature_type_id,
                        member_id,
                    }),
                    Some(dir::FunctionRole::Setter) => slots.push(DynamicMember::Setter {
                        name: member_name,
                        signature: signature_type_id,
                        member_id,
                    }),
                    Some(dir::FunctionRole::Call) => slots.push(DynamicMember::Call {
                        signature: signature_type_id,
                        member_id,
                    }),
                    Some(dir::FunctionRole::Constructor | dir::FunctionRole::New) => {
                        return Err(LowerError::UnsupportedConstruct {
                            anchor: self.diagnostic_anchor(
                                member_id
                                    .into_global_any(self.module_id)
                                    .into_anchored(Some(self.profile)),
                            ),
                            message: "construct signatures are not dynamic compatible".to_string(),
                        }
                        .into());
                    }
                    None => slots.push(DynamicMember::Method {
                        name: member_name,
                        signature: signature_type_id,
                        member_id,
                    }),
                }
            }
            dir::TypeMember::CallSignature { .. } => {
                let signature_type_id =
                    self.signature_type_id_for_node(member_id.into_global_any(self.module_id))?;
                slots.push(DynamicMember::Call {
                    signature: signature_type_id,
                    member_id,
                });
            }
            dir::TypeMember::ConstructSignature { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "construct signatures are not dynamic compatible".to_string(),
                }
                .into());
            }
            dir::TypeMember::IndexSignature { .. } => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "index signatures are not supported for native lowering".to_string(),
                }
                .into());
            }
            _ => {}
        }

        Ok(())
    }

    /// Resolve a static dynamic field name for dispatch metadata.
    fn dynamic_field_name(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        key: dir::Key,
    ) -> LowerResult<StringId> {
        let Some(key) = key.static_key(self.dir_tree) else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "unsupported non-public field key in dynamic layout".to_string(),
            }
            .into());
        };

        Ok(static_key_to_field_name(&key, &mut self.builder))
    }

    /// Return the canonical dynamic dispatch field node for a member.
    fn dynamic_dispatch_field(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        field_name: StringId,
        field_type: dir::LocalTypeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Field>> {
        // reuse an existing field id when available
        let member_key = member_id.id;
        if let Some(field_id) = self.dynamic_fields_by_member.get(&member_key) {
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
        self.dynamic_fields_by_member
            .insert(member_key, field_id);

        Ok(field_id)
    }
}
