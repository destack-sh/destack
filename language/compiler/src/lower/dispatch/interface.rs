use {destack_dir as dir, destack_mir as mir};

use destack_core::StringId;

use crate::{LowerError, LowerResult};

use crate::lower::{InterfaceEntry, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Write interface dispatch tables.
    pub(crate) fn emit_interface_tables(&mut self) -> LowerResult<()> {
        // concrete interface pairs
        for (concrete, interface) in self.interface_table_pairs.clone() {
            self.interface_table_for_pair(concrete, interface)?;
        }

        Ok(())
    }

    /// Write one interface dispatch table.
    fn interface_table_for_pair(
        &mut self,
        concrete: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
    ) -> LowerResult<()> {
        if self
            .lowered_interface_tables
            .contains(&(concrete, interface))
        {
            return Ok(());
        }

        // check for cycles
        if self
            .interface_table_in_progress
            .contains(&(concrete, interface))
        {
            let anchor = self
                .declaration_ids_for_symbol(interface)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: "interface table cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "cycle detected while lowering interface table".to_string(),
            }
            .into());
        }
        self.interface_table_in_progress
            .insert((concrete, interface));

        // resolve the declaration
        let declaration_id = self
            .declaration_ids_for_symbol(interface)
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
                message: "interface declaration missing for interface table".to_string(),
            }
            .into());
        };

        // collect interface slots in declaration order
        let interface_slots = self.lower_interface_slots(interface)?;

        // resolve concrete instance type
        let instance_type_id = self.types.get_instance_type_id(concrete).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(declaration_id),
                message: "missing concrete instance type".to_string(),
            }
        })?;

        // resolve concrete layout from cache
        let concrete_mir_type = self.lower_type(instance_type_id, declaration_id)?;

        // resolve interface instance type
        let interface_type_id = self.types.get_instance_type_id(interface).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(declaration_id),
                message: "missing interface instance type".to_string(),
            }
        })?;

        // lower interface instance type
        let interface_mir_type = self.lower_type(interface_type_id, declaration_id)?;

        // build interface slots and table entries
        let mut entries = Vec::with_capacity(interface_slots.len());
        let mut shape_slots = Vec::with_capacity(interface_slots.len());

        // append interface slots
        for slot in interface_slots {
            match slot {
                InterfaceEntry::Field {
                    name,
                    field,
                    member_id,
                    ..
                } => {
                    // resolve field offset slot
                    let offset = self.interface_field_offset(
                        concrete_mir_type,
                        name,
                        member_id,
                        declaration_id,
                    )?;
                    entries.push(mir::InterfaceTableEntry::FieldOffset { offset });
                    shape_slots.push(mir::InterfaceSlot::Field {
                        field,
                        field_name: name,
                    });
                }
                InterfaceEntry::Method {
                    name,
                    signature,
                    member_id,
                    ..
                } => {
                    let signature_type_id = signature;
                    let signature = self.lower_type(signature_type_id, declaration_id)?;
                    let target_method =
                        self.interface_method_target(concrete, name, signature_type_id, member_id)?;
                    entries.push(mir::InterfaceTableEntry::Method {
                        function: target_method,
                    });
                    shape_slots.push(mir::InterfaceSlot::Method { name, signature });
                }
            }
        }

        // register the canonical interface shape
        let dispatch_table = &mut self.builder.tree_mut().metadata.dispatch;
        let shape = mir::InterfaceShape {
            interface: interface_mir_type,
            slots: shape_slots,
        };
        match dispatch_table.interface_shape(interface_mir_type) {
            Some(existing_shape) => {
                if existing_shape != &shape {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(declaration_id),
                        message: "inconsistent interface shape".to_string(),
                    }
                    .into());
                }
            }
            None => {
                dispatch_table.insert_interface_shape(interface_mir_type, shape);
            }
        }

        // table metadata and static storage
        {
            let interface_table_global = self
                .interface_table_globals_by_pair
                .get(&(concrete, interface))
                .copied()
                .ok_or_else(|| LowerError::Internal {
                    anchor: self.diagnostic_anchor(declaration_id),
                    module: self.module_id,
                    message: "missing interface table global for interface pair".to_string(),
                })?;
            let initializer = self.interface_table_initializer(&entries)?;
            self.builder
                .tree_mut()
                .get_mut(interface_table_global.global_id)
                .initializer = Some(initializer);

            let table = mir::InterfaceTable {
                concrete: concrete_mir_type,
                interface: interface_mir_type,
                global: interface_table_global.global_id,
                entries,
            };
            self.builder
                .tree_mut()
                .metadata
                .dispatch
                .insert_interface_table(table);
        }

        // lowered table guard
        self.record_interface_table((concrete, interface))?;
        self.interface_table_in_progress
            .shift_remove(&(concrete, interface));

        Ok(())
    }

    /// Resolve the concrete field offset for an interface field.
    fn interface_field_offset(
        &self,
        concrete_mir_type: mir::LocalNodeId<mir::Type>,
        field_name: StringId,
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
                .field_index(field_name)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "interface field missing on concrete type".to_string(),
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
                message: "interface field offset missing".to_string(),
            })?
            .offset;

        Ok(field_offset)
    }

    /// Resolve the concrete method target for an interface method.
    fn interface_method_target(
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
            message: "missing interface method implementation".to_string(),
        }
        .into())
    }

    /// Collect concrete to interface pairs for interface table generation.
    pub(crate) fn collect_interface_pairs(
        &self,
    ) -> Vec<(dir::GlobalSymbolId, dir::GlobalSymbolId)> {
        Vec::new()
    }

    /// Build the static initializer for one interface dispatch table.
    fn interface_table_initializer(
        &self,
        entries: &[mir::InterfaceTableEntry],
    ) -> LowerResult<mir::GlobalInitializer> {
        let mut elements = Vec::with_capacity(mir::InterfaceTable::storage_len(entries.len()));
        elements.push(mir::GlobalInitializer::zero());

        // interface slots
        for entry in entries {
            let element = match entry {
                mir::InterfaceTableEntry::Method { function } => {
                    mir::GlobalInitializer::function_address((*function).into())
                }
                mir::InterfaceTableEntry::FieldOffset { offset, .. } => {
                    let constant = self.interface_table_field_offset_constant(*offset)?;

                    mir::GlobalInitializer::scalar(constant)
                }
            };
            elements.push(element);
        }

        Ok(mir::GlobalInitializer::aggregate(elements))
    }

    /// Build one pointer sized field offset constant.
    fn interface_table_field_offset_constant(&self, offset: u32) -> LowerResult<mir::Constant> {
        let constant = match self.type_lowerer.pointer_width_bits() {
            32 => mir::Constant::uint32(offset),
            64 => mir::Constant::uint64(u64::from(offset)),
            width => {
                return Err(LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: format!(
                        "unsupported pointer width for interface table offset: {width}"
                    ),
                }
                .into());
            }
        };

        Ok(constant)
    }
}
