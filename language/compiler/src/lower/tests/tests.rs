use destack_base::ImmutableStringPool;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::TestProgram;

/// Interface call information extracted from MIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InterfaceCall {
    /// The declaring interface type.
    pub(crate) declaring_type: mir::LocalNodeId<mir::Type>,
    /// The interface slot id.
    pub(crate) slot_id: u32,
}

/// Virtual call information extracted from MIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VirtualCall {
    /// The declaring type for dispatch.
    pub(crate) declaring_type: mir::LocalNodeId<mir::Type>,
    /// The vtable slot id.
    pub(crate) slot_id: u32,
}

impl TestProgram {
    /// Collect class dispatch tables from a MIR tree.
    pub(crate) fn class_dispatch_tables<'a>(
        &self,
        tree: &'a mir::NodeTree,
    ) -> Vec<&'a mir::DispatchTable> {
        // filter tables by kind
        tree.type_table
            .dispatch_registry
            .tables
            .iter()
            .filter(|table| matches!(table.kind, mir::DispatchTableKind::Class { .. }))
            .collect()
    }

    /// Collect interface dispatch tables from a MIR tree.
    pub(crate) fn interface_dispatch_tables<'a>(
        &self,
        tree: &'a mir::NodeTree,
    ) -> Vec<&'a mir::DispatchTable> {
        // filter tables by kind
        tree.type_table
            .dispatch_registry
            .tables
            .iter()
            .filter(|table| matches!(table.kind, mir::DispatchTableKind::Interface { .. }))
            .collect()
    }

    /// Resolve an interface dispatch table for a concrete and interface object pair.
    pub(crate) fn interface_dispatch_table<'a>(
        &self,
        tree: &'a mir::NodeTree,
        strings: &ImmutableStringPool,
        concrete_name: &str,
        interface_name: &str,
    ) -> &'a mir::DispatchTable {
        let concrete_type = self.type_by_metadata_name(tree, strings, concrete_name);
        let interface_type = self.type_by_metadata_name(tree, strings, interface_name);

        tree.type_table
            .dispatch_registry
            .tables
            .iter()
            .find(|table| {
                matches!(
                    table.kind,
                    mir::DispatchTableKind::Interface { concrete, interface }
                        if concrete == concrete_type && interface == interface_type
                )
            })
            .unwrap_or_else(|| {
                panic!(
                    "missing interface dispatch table for '{concrete_name}' -> '{interface_name}'"
                )
            })
    }

    /// Assert that a tree has exactly one interface dispatch table.
    pub(crate) fn expect_single_interface_table<'a>(
        &self,
        tree: &'a mir::NodeTree,
    ) -> &'a mir::DispatchTable {
        // collect interface tables
        let tables = self.interface_dispatch_tables(tree);
        assert_eq!(tables.len(), 1);
        tables[0]
    }

    /// Assert that a vtable has the fixed prefix slots.
    pub(crate) fn assert_vtable_prefix(&self, table: &mir::DispatchTable) {
        // require the type tag slot
        assert!(matches!(table.slots[0], mir::DispatchSlot::TypeTag));

        // require the destructor slot
        assert!(matches!(
            table.slots[1],
            mir::DispatchSlot::Destructor { .. }
        ));
    }

    /// Count method slots in a class vtable.
    pub(crate) fn count_vtable_methods(&self, table: &mir::DispatchTable) -> usize {
        // count method slots
        table
            .slots
            .iter()
            .filter(|slot| matches!(slot, mir::DispatchSlot::Method { .. }))
            .count()
    }

    /// Collect method names from a class vtable in slot order.
    pub(crate) fn vtable_method_names(
        &self,
        table: &mir::DispatchTable,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
    ) -> Vec<String> {
        table
            .slots
            .iter()
            .filter_map(|slot| {
                let mir::DispatchSlot::Method { function } = slot else {
                    return None;
                };
                Some(strings.get(tree.get(*function).name).to_string())
            })
            .collect()
    }

    /// Resolve an interface field offset for a given field name.
    pub(crate) fn interface_field_offset(
        &self,
        table: &mir::DispatchTable,
        strings: &ImmutableStringPool,
        field_name: &str,
    ) -> Option<u32> {
        // scan field offset slots for the field name
        for slot in &table.slots {
            let mir::DispatchSlot::FieldOffset {
                field_name: slot_name,
                offset,
            } = slot
            else {
                continue;
            };

            if strings.get(*slot_name) == field_name {
                return Some(*offset);
            }
        }

        None
    }

    /// Resolve an interface field offset or panic.
    pub(crate) fn expect_interface_field_offset(
        &self,
        table: &mir::DispatchTable,
        strings: &ImmutableStringPool,
        field_name: &str,
    ) -> u32 {
        self.interface_field_offset(table, strings, field_name)
            .unwrap_or_else(|| panic!("missing interface field offset '{field_name}'"))
    }

    /// Resolve the target method name for an interface method slot.
    pub(crate) fn interface_method_target_name(
        &self,
        table: &mir::DispatchTable,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        method_name: &str,
    ) -> Option<String> {
        // scan interface method slots for the method name
        for slot in &table.slots {
            let mir::DispatchSlot::InterfaceMethod {
                interface_method,
                target,
            } = slot
            else {
                continue;
            };

            let interface_name = strings.get(tree.get(*interface_method).name);
            if interface_name == method_name {
                let target_name = strings.get(tree.get(*target).name);
                return Some(target_name.to_string());
            }
        }

        None
    }

    /// Resolve an interface method target name or panic.
    pub(crate) fn expect_interface_method_target_name(
        &self,
        table: &mir::DispatchTable,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        method_name: &str,
    ) -> String {
        self.interface_method_target_name(table, tree, strings, method_name)
            .unwrap_or_else(|| panic!("missing interface method target '{method_name}'"))
    }

    /// Find a type with a matching type metadata name.
    pub(crate) fn find_type_by_metadata_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // scan metadata entries for a matching name
        for (ty, metadata) in &tree.type_table.type_metadata_by_id {
            let Some(name_id) = metadata.name else {
                continue;
            };

            if strings.get(name_id) != name {
                continue;
            }

            return Some(*ty);
        }

        None
    }

    /// Find a type with a matching type metadata name or panic.
    pub(crate) fn type_by_metadata_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> mir::LocalNodeId<mir::Type> {
        self.find_type_by_metadata_name(tree, strings, name)
            .unwrap_or_else(|| panic!("missing type metadata name '{name}'"))
    }

    /// Resolve type metadata for a mir type id or panic.
    pub(crate) fn type_metadata<'a>(
        &self,
        tree: &'a mir::NodeTree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> &'a mir::TypeMetadata {
        tree.type_table
            .type_metadata_by_id
            .get(&type_id)
            .unwrap_or_else(|| panic!("missing type metadata for '{type_id:?}'"))
    }

    /// Resolve lineage metadata for a type id or panic.
    pub(crate) fn type_lineage<'a>(
        &self,
        tree: &'a mir::NodeTree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> &'a mir::TypeLineage {
        let metadata = self.type_metadata(tree, type_id);
        metadata
            .lineage
            .as_ref()
            .unwrap_or_else(|| panic!("missing lineage metadata for '{type_id:?}'"))
    }

    /// Resolve a parent type from lineage metadata or panic.
    pub(crate) fn type_parent(
        &self,
        tree: &mir::NodeTree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        let lineage = self.type_lineage(tree, type_id);
        lineage
            .parent
            .unwrap_or_else(|| panic!("missing parent type for '{type_id:?}'"))
    }

    /// Resolve a vtable id for a type or panic.
    pub(crate) fn type_vtable_id(
        &self,
        tree: &mir::NodeTree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> mir::DispatchTableId {
        let metadata = self.type_metadata(tree, type_id);
        metadata
            .vtable
            .unwrap_or_else(|| panic!("missing vtable id for '{type_id:?}'"))
    }

    /// Resolve a function parameter type id from DIR by function name.
    #[allow(dead_code)]
    pub(crate) fn dir_function_parameter_type_id(
        &self,
        module_id: ModuleId,
        function_name: &str,
        index: usize,
    ) -> Option<dir::LocalTypeId> {
        // load the dir module state
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module
            .dir_base_maybe()
            .or_else(|| module.dir_maybe(self.default_profile_id(module_id)))
            .unwrap_or_else(|| {
                panic!("no DIR available for module {module_id:?}");
            });
        let tree = dir.tree.read();
        let types = dir.types.read();

        // scan for the named function declaration
        for (_, declaration) in tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function {
                descriptor,
                signature,
                ..
            } = declaration
            else {
                continue;
            };

            let Some(name) = descriptor.name else {
                continue;
            };

            let name_str = self.program.strings.get(name.string());
            if name_str != function_name {
                continue;
            }

            let parameter_id = signature.dynamic_parameters.get(index)?;
            let parameter = tree.get(*parameter_id);
            let symbol_id = parameter.symbol();
            let global_symbol = dir::GlobalSymbolId::new(module_id, symbol_id);
            return types.get_value_type_id(global_symbol);
        }

        None
    }

    /// Resolve the field type id for a struct field name.
    pub(crate) fn struct_field_type_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // load the struct type
        let mir::Type::Struct { fields, .. } = tree.get(struct_type) else {
            return None;
        };

        // scan for the named field
        for field_id in fields {
            let field = tree.get(*field_id);
            let Some(name_id) = field.name else {
                continue;
            };

            if strings.get(name_id) == field_name {
                return Some(field.ty);
            }
        }

        None
    }

    /// Resolve the field type id for a struct field name or panic.
    pub(crate) fn expect_struct_field_type_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> mir::LocalNodeId<mir::Type> {
        self.struct_field_type_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing struct field '{field_name}'"))
    }

    /// Resolve the byte offset for a struct field name.
    pub(crate) fn struct_field_offset_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> Option<u32> {
        // load the struct type
        let mir::Type::Struct { fields, .. } = tree.get(struct_type) else {
            return None;
        };

        // locate the requested field index
        let field_index = fields.iter().position(|field_id| {
            let field = tree.get(*field_id);
            field
                .name
                .map(|name| strings.get(name) == field_name)
                .unwrap_or(false)
        })?;

        // resolve the layout metadata for offsets
        let metadata = tree.type_table.type_metadata_by_id.get(&struct_type)?;
        let layout = metadata.layout.as_ref()?;
        layout.field_offsets.get(field_index).copied()
    }

    /// Resolve the byte offset for a struct field name or panic.
    pub(crate) fn expect_struct_field_offset_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> u32 {
        self.struct_field_offset_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing struct field offset '{field_name}'"))
    }

    /// Resolve a field map entry by field name.
    pub(crate) fn field_map_entry_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> Option<mir::LocalNodeId<mir::Field>> {
        let metadata = tree.type_table.type_metadata_by_id.get(&struct_type)?;
        metadata
            .field_map
            .iter()
            .find_map(|(name, field_id)| (strings.get(*name) == field_name).then_some(*field_id))
    }

    /// Resolve a field map entry by field name or panic.
    pub(crate) fn expect_field_map_entry_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> mir::LocalNodeId<mir::Field> {
        self.field_map_entry_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing field map entry '{field_name}'"))
    }

    /// Resolve a field name for a field id or panic.
    pub(crate) fn field_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        field_id: mir::LocalNodeId<mir::Field>,
    ) -> String {
        tree.get(field_id)
            .name
            .map(|name| strings.get(name).to_string())
            .unwrap_or_else(|| panic!("missing field name for '{field_id:?}'"))
    }

    /// Find a MIR function id by name.
    pub(crate) fn find_function_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        name: &str,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        // scan for a matching function name
        tree.iter_nodes::<mir::Function>()
            .find(|(_, function)| strings.get(function.name) == name)
            .map(|(id, _)| id)
    }

    /// Resolve a function parameter type by function name and index.
    #[allow(dead_code)]
    pub(crate) fn function_parameter_type_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        name: &str,
        index: usize,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // find the matching function
        let function_id = self.find_function_by_name(tree, strings, name)?;
        let function = tree.get(function_id);

        // resolve the parameter type
        function.parameters.get(index).map(|param| param.ty)
    }

    /// Find the first interface dispatch call in a function body.
    pub(crate) fn find_interface_call_info(
        &self,
        tree: &mir::NodeTree,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<InterfaceCall> {
        // scan call instructions for interface dispatch
        let function = tree.get(function_id);
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::CallInterface {
                    declaring_type,
                    slot_id,
                    ..
                } = tree.get(*instruction_id)
                {
                    return Some(InterfaceCall {
                        declaring_type: *declaring_type,
                        slot_id: *slot_id,
                    });
                }
            }
        }

        None
    }

    /// Find the first virtual dispatch call in a function body.
    pub(crate) fn find_virtual_call_info(
        &self,
        tree: &mir::NodeTree,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<VirtualCall> {
        // scan call instructions for virtual dispatch
        let function = tree.get(function_id);
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::CallVirtual {
                    declaring_type,
                    slot_id,
                    ..
                } = tree.get(*instruction_id)
                {
                    return Some(VirtualCall {
                        declaring_type: *declaring_type,
                        slot_id: *slot_id,
                    });
                }
            }
        }

        None
    }

    /// Find interface dispatch calls for a function name.
    pub(crate) fn find_interface_call_info_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        function_name: &str,
    ) -> Option<InterfaceCall> {
        // resolve the function id by name
        let function_id = self.find_function_by_name(tree, strings, function_name)?;

        // scan the function for interface dispatch
        self.find_interface_call_info(tree, function_id)
    }

    /// Find interface dispatch calls for a function name or panic.
    pub(crate) fn interface_call_info_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        function_name: &str,
    ) -> InterfaceCall {
        self.find_interface_call_info_by_name(tree, strings, function_name)
            .unwrap_or_else(|| panic!("missing interface call info '{function_name}'"))
    }

    /// Find virtual dispatch calls for a function name.
    pub(crate) fn find_virtual_call_info_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        function_name: &str,
    ) -> Option<VirtualCall> {
        // scan the function for virtual dispatch
        let function_id = self.find_function_by_name(tree, strings, function_name)?;
        self.find_virtual_call_info(tree, function_id)
    }

    /// Find virtual dispatch calls for a function name or panic.
    pub(crate) fn virtual_call_info_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        function_name: &str,
    ) -> VirtualCall {
        self.find_virtual_call_info_by_name(tree, strings, function_name)
            .unwrap_or_else(|| panic!("missing virtual call info '{function_name}'"))
    }
}
