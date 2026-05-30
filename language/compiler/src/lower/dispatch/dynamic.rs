use destack_dir as dir;
use destack_mir as mir;

use destack_core::StringId;

use crate::{LowerError, LowerResult};

use crate::lower::{DynamicMember, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Write dynamic dispatch tables.
    pub(crate) fn emit_dynamic_tables(&mut self) -> LowerResult<()> {
        // concrete constraint pairs
        for (concrete, constraint) in self.dynamic_table_pairs.clone() {
            self.dynamic_table_for_pair(concrete, constraint)?;
        }

        Ok(())
    }

    /// Write one dynamic dispatch table.
    fn dynamic_table_for_pair(
        &mut self,
        concrete: dir::GlobalSymbolId,
        constraint: dir::GlobalSymbolId,
    ) -> LowerResult<()> {
        if self
            .lowered_dynamic_tables
            .contains(&(concrete, constraint))
        {
            return Ok(());
        }

        // check for cycles
        if self
            .dynamic_table_in_progress
            .contains(&(concrete, constraint))
        {
            let anchor = self
                .declaration_ids_for_symbol(constraint)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: "dynamic table cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "cycle detected while lowering dynamic table".to_string(),
            }
            .into());
        }
        self.dynamic_table_in_progress
            .insert((concrete, constraint));

        // resolve the declaration
        let declaration_id = self
            .declaration_ids_for_symbol(constraint)
            .first()
            .copied()
            .map(|id| {
                id.into_global_any(self.module_id)
                    .into_anchored(Some(self.profile))
            });
        let Some(declaration_id) = declaration_id else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: "dynamic constraint declaration missing for table".to_string(),
            }
            .into());
        };

        // collect dynamic members in declaration order
        let dynamic_members = self.lower_dynamic_members(constraint)?;

        // resolve concrete instance type
        let instance_type_id = self.types.get_instance_type_id(concrete).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(declaration_id),
                message: "missing concrete instance type".to_string(),
            }
        })?;

        // resolve concrete layout from cache
        let concrete_mir_type = self.lower_type(instance_type_id, declaration_id)?;

        // resolve constraint instance type
        let constraint_type_id = self.types.get_instance_type_id(constraint).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(declaration_id),
                message: "missing dynamic constraint instance type".to_string(),
            }
        })?;

        // lower constraint instance type
        let constraint_mir_type = self.lower_type(constraint_type_id, declaration_id)?;

        // build dynamic slots and table entries
        let mut entries = Vec::with_capacity(dynamic_members.len());
        let mut shape_slots = Vec::with_capacity(dynamic_members.len());

        // append dynamic slots
        for member in dynamic_members {
            match member {
                DynamicMember::Field {
                    name,
                    field,
                    member_id,
                    ..
                } => {
                    // resolve field offset slot
                    let offset = self.dynamic_field_offset(
                        concrete_mir_type,
                        name,
                        member_id,
                        declaration_id,
                    )?;
                    entries.push(mir::DynamicEntry::Field { offset });
                    shape_slots.push(mir::DynamicSlot::Field { field, name });
                }
                DynamicMember::Getter {
                    name,
                    signature,
                    member_id,
                } => {
                    let signature_type_id = signature;
                    let signature = self.lower_type(signature_type_id, declaration_id)?;
                    let target_method =
                        self.dynamic_method_target(concrete, name, signature_type_id, member_id)?;
                    entries.push(mir::DynamicEntry::Getter {
                        function: target_method,
                    });
                    shape_slots.push(mir::DynamicSlot::Getter { name, signature });
                }
                DynamicMember::Setter {
                    name,
                    signature,
                    member_id,
                } => {
                    let signature_type_id = signature;
                    let signature = self.lower_type(signature_type_id, declaration_id)?;
                    let target_method =
                        self.dynamic_method_target(concrete, name, signature_type_id, member_id)?;
                    entries.push(mir::DynamicEntry::Setter {
                        function: target_method,
                    });
                    shape_slots.push(mir::DynamicSlot::Setter { name, signature });
                }
                DynamicMember::Method {
                    name,
                    signature,
                    member_id,
                    ..
                } => {
                    let signature_type_id = signature;
                    let signature = self.lower_type(signature_type_id, declaration_id)?;
                    let target_method =
                        self.dynamic_method_target(concrete, name, signature_type_id, member_id)?;
                    entries.push(mir::DynamicEntry::Method {
                        function: target_method,
                    });
                    shape_slots.push(mir::DynamicSlot::Method { name, signature });
                }
                DynamicMember::Call {
                    signature,
                    member_id,
                } => {
                    let signature_type_id = signature;
                    let signature = self.lower_type(signature_type_id, declaration_id)?;
                    let target_method = self.dynamic_method_target(
                        concrete,
                        self.dispatch_call_name,
                        signature_type_id,
                        member_id,
                    )?;
                    entries.push(mir::DynamicEntry::Call {
                        function: target_method,
                    });
                    shape_slots.push(mir::DynamicSlot::Call { signature });
                }
            }
        }

        // register the canonical dynamic shape
        let dispatch_table = &mut self.builder.tree_mut().metadata.dispatch;
        let shape = mir::DynamicShape {
            constraint: constraint_mir_type,
            slots: shape_slots,
        };
        match dispatch_table.dynamic_shape(constraint_mir_type) {
            Some(existing_shape) => {
                if existing_shape != &shape {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(declaration_id),
                        message: "inconsistent dynamic shape".to_string(),
                    }
                    .into());
                }
            }
            None => {
                dispatch_table.insert_dynamic_shape(constraint_mir_type, shape);
            }
        }

        // table metadata and static storage
        {
            let dynamic_table_global = self
                .dynamic_table_globals_by_pair
                .get(&(concrete, constraint))
                .copied()
                .ok_or_else(|| LowerError::Internal {
                    anchor: self.diagnostic_anchor(declaration_id),
                    module: self.module_id,
                    message: "missing dynamic table global".to_string(),
                })?;
            let initializer = self.dynamic_table_initializer(&entries)?;
            self.builder
                .tree_mut()
                .get_mut(dynamic_table_global.global_id)
                .initializer = Some(initializer);

            let table = mir::DynamicTable {
                concrete: concrete_mir_type,
                constraint: constraint_mir_type,
                global: dynamic_table_global.global_id,
                entries,
            };
            self.builder
                .tree_mut()
                .metadata
                .dispatch
                .insert_dynamic_table(table);
        }

        // lowered table guard
        self.record_dynamic_table((concrete, constraint))?;
        self.dynamic_table_in_progress
            .shift_remove(&(concrete, constraint));

        Ok(())
    }

    /// Resolve the concrete field offset for a dynamic field.
    fn dynamic_field_offset(
        &self,
        concrete_mir_type: mir::LocalNodeId<mir::Type>,
        name: StringId,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<u32> {
        // resolve the struct layout for the concrete type
        let layout = self
            .type_lowerer
            .layout_for_type_or_error(concrete_mir_type, anchor)?;

        // map the field name to an offset
        let field_index =
            layout
                .field_index(name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "dynamic field missing on concrete type".to_string(),
                })?;

        // resolve the field offset
        let field_offset = layout
            .field(field_index)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "dynamic field offset missing".to_string(),
            })?
            .offset;

        Ok(field_offset)
    }

    /// Resolve the concrete method target for a dynamic method.
    fn dynamic_method_target(
        &self,
        concrete: dir::GlobalSymbolId,
        method_name: StringId,
        signature_type_id: dir::LocalTypeId,
        member_id: dir::LocalNodeId<dir::TypeMember>,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // decide the search order for concrete methods
        let mut symbols = Vec::new();
        if self.symbol_kind_matches(concrete, dir::SymbolKind::Class) {
            symbols.extend(self.collect_class_lineage(concrete).into_iter().rev());
        } else {
            symbols.push(concrete);
        }

        // scan class symbols for a matching method
        for class_symbol in symbols {
            // collect declaration ids
            let declaration_ids = self.declaration_ids_for_symbol(class_symbol);

            // scan declarations for members
            for declaration_id in declaration_ids {
                let declaration = self.dir_tree.get(declaration_id);
                let members = match declaration {
                    dir::Declaration::Class(declaration) => &declaration.members,
                    dir::Declaration::Struct(declaration) => &declaration.members,
                    _ => continue,
                };

                // scan members for a matching method
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    let dir::Member::Method { key, signature, .. } = member else {
                        continue;
                    };

                    // skip non instance members
                    if member.has_static_modifier() || self.member_is_private(member) {
                        continue;
                    }

                    // match the method name
                    let name = self.member_dispatch_name_or_error(
                        key.as_ref(),
                        signature.role,
                        member_id.into_any(),
                    )?;
                    if name != method_name {
                        continue;
                    }

                    // match the method signature
                    let signature_id = self.method_signature_type_id(*member_id)?;
                    if !self.method_signatures_equivalent(signature_id, signature_type_id) {
                        continue;
                    }

                    // resolve the method function id
                    let method_symbol = self.require_symbol_for_node(*member_id)?;
                    let function_id = self.method_function_id(*member_id, method_symbol)?;
                    return Ok(function_id);
                }
            }
        }

        // report missing implementation
        Err(LowerError::UnsupportedConstruct {
            anchor: self.diagnostic_anchor(
                member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
            ),
            message: "missing dynamic method implementation".to_string(),
        }
        .into())
    }

    /// Collect concrete to constraint pairs for dynamic table generation.
    pub(crate) fn collect_dynamic_pairs(&self) -> Vec<(dir::GlobalSymbolId, dir::GlobalSymbolId)> {
        Vec::new()
    }

    /// Build the static initializer for one dynamic dispatch table.
    fn dynamic_table_initializer(
        &self,
        entries: &[mir::DynamicEntry],
    ) -> LowerResult<mir::GlobalInitializer> {
        let mut elements = Vec::with_capacity(mir::DynamicTable::storage_len(entries.len()));

        // dynamic slots
        for entry in entries {
            let element = match entry {
                mir::DynamicEntry::Getter { function }
                | mir::DynamicEntry::Setter { function }
                | mir::DynamicEntry::Method { function }
                | mir::DynamicEntry::Call { function } => {
                    mir::GlobalInitializer::function_address((*function).into())
                }
                mir::DynamicEntry::Field { offset, .. } => {
                    let constant = self.dynamic_table_field_offset_constant(*offset)?;

                    mir::GlobalInitializer::scalar(constant)
                }
            };
            elements.push(element);
        }

        Ok(mir::GlobalInitializer::aggregate(elements))
    }

    /// Build one pointer sized field offset constant.
    fn dynamic_table_field_offset_constant(&self, offset: u32) -> LowerResult<mir::Constant> {
        let constant = match self.type_lowerer.pointer_width_bits() {
            32 => mir::Constant::uint32(offset),
            64 => mir::Constant::uint64(u64::from(offset)),
            width => {
                return Err(LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: format!("unsupported pointer width for dynamic table offset: {width}"),
                }
                .into());
            }
        };

        Ok(constant)
    }
}
