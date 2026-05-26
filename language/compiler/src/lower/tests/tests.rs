use destack_core::StringPool;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::TestProgram;
use crate::lower::module::string_literal_global_name_for_content;

/// Interface call information extracted from MIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InterfaceCall {
    /// The interface type.
    pub(crate) interface: mir::LocalNodeId<mir::Type>,
    /// The dispatch slot.
    pub(crate) slot: mir::DispatchSlot,
}

/// Class call information extracted from MIR.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ClassCall {
    /// The class type.
    pub(crate) class: mir::LocalNodeId<mir::Type>,
    /// The dispatch slot.
    pub(crate) slot: mir::DispatchSlot,
}

impl TestProgram {
    /// Build the synthetic global name for a string literal.
    pub(crate) fn string_literal_global_name(&self, value: &str) -> String {
        string_literal_global_name_for_content(value)
    }

    /// Return the canonical string type alias definition.
    pub(crate) fn string_type_alias_definition(&self) -> &'static str {
        r#"
type String {
    lengthUtf16: uint32;
    lengthBytes: uint32;
    data: ref<uint8, raw, readonly>;
}"#
    }

    /// Collect class dispatch tables from a MIR tree.
    pub(crate) fn class_dispatch_tables<'a>(&self, tree: &'a mir::Tree) -> Vec<&'a mir::Vtable> {
        // collect class vtables
        tree.metadata.dispatch.vtables.iter().collect()
    }

    /// Collect interface dispatch tables from a MIR tree.
    pub(crate) fn interface_dispatch_tables<'a>(
        &self,
        tree: &'a mir::Tree,
    ) -> Vec<&'a mir::InterfaceTable> {
        // collect interface tables
        tree.metadata.dispatch.interface_tables.iter().collect()
    }

    /// Resolve an interface table for a concrete and interface pair.
    pub(crate) fn interface_dispatch_table<'a>(
        &self,
        tree: &'a mir::Tree,
        strings: &StringPool,
        concrete_name: &str,
        interface_name: &str,
    ) -> &'a mir::InterfaceTable {
        let concrete_type = self.type_by_metadata_name(tree, strings, concrete_name);
        let interface_type = self.type_by_metadata_name(tree, strings, interface_name);

        tree.metadata
            .dispatch
            .interface_tables
            .iter()
            .find(|table| table.concrete == concrete_type && table.interface == interface_type)
            .unwrap_or_else(|| {
                panic!(
                    "missing interface table for '{concrete_name}' -> '{interface_name}'"
                )
            })
    }

    /// Assert that a tree has exactly one interface table.
    pub(crate) fn expect_single_interface_table<'a>(
        &self,
        tree: &'a mir::Tree,
    ) -> &'a mir::InterfaceTable {
        // collect interface tables
        let tables = self.interface_dispatch_tables(tree);
        assert_eq!(tables.len(), 1);
        tables[0]
    }

    /// Assert that a vtable has the fixed prefix slots.
    pub(crate) fn assert_vtable_prefix(&self, table: &mir::Vtable) {
        // require the type descriptor slot
        assert!(matches!(table.entries[0], mir::VtableEntry::TypeDescriptor));

        // require the destructor slot
        assert!(matches!(
            table.entries[1],
            mir::VtableEntry::Destructor { .. }
        ));
    }

    /// Count method slots in a class vtable.
    pub(crate) fn count_vtable_methods(&self, table: &mir::Vtable) -> usize {
        // count method slots
        table
            .entries
            .iter()
            .filter(|slot| matches!(slot, mir::VtableEntry::Method { .. }))
            .count()
    }

    /// Collect method names from a class vtable in slot order.
    pub(crate) fn vtable_method_names(
        &self,
        table: &mir::Vtable,
        tree: &mir::Tree,
        strings: &StringPool,
    ) -> Vec<String> {
        table
            .entries
            .iter()
            .filter_map(|slot| {
                let mir::VtableEntry::Method { function } = slot else {
                    return None;
                };
                Some(strings.get(tree.get(*function).name).to_string())
            })
            .collect()
    }

    /// Resolve an interface field offset for a given field name.
    pub(crate) fn interface_field_offset(
        &self,
        table: &mir::InterfaceTable,
        tree: &mir::Tree,
        strings: &StringPool,
        field_name: &str,
    ) -> Option<u32> {
        let shape = tree.metadata.dispatch.interface_shape(table.interface)?;

        // scan field slots by interface shape
        for (index, slot) in table.entries.iter().enumerate() {
            if let mir::InterfaceTableEntry::FieldOffset { offset } = slot
                && let Some(mir::InterfaceSlot::Field {
                    field_name: slot_name,
                    ..
                }) = shape.slots.get(index)
                && strings.get(*slot_name) == field_name
            {
                return Some(*offset);
            }
        }

        None
    }

    /// Resolve an interface field offset or panic.
    pub(crate) fn expect_interface_field_offset(
        &self,
        table: &mir::InterfaceTable,
        tree: &mir::Tree,
        strings: &StringPool,
        field_name: &str,
    ) -> u32 {
        self.interface_field_offset(table, tree, strings, field_name)
            .unwrap_or_else(|| panic!("missing interface field offset '{field_name}'"))
    }

    /// Resolve the target method name for an interface method slot.
    pub(crate) fn interface_method_target_name(
        &self,
        table: &mir::InterfaceTable,
        tree: &mir::Tree,
        strings: &StringPool,
        method_name: &str,
    ) -> Option<String> {
        let shape = tree.metadata.dispatch.interface_shape(table.interface)?;

        // scan method slots by interface shape
        for (index, slot) in table.entries.iter().enumerate() {
            if let mir::InterfaceTableEntry::Method { function } = slot
                && let Some(mir::InterfaceSlot::Method { name, .. }) = shape.slots.get(index)
                && strings.get(*name) == method_name
            {
                let target_name = strings.get(tree.get(*function).name);
                return Some(target_name.to_string());
            }
        }

        None
    }

    /// Resolve an interface method target name or panic.
    pub(crate) fn expect_interface_method_target_name(
        &self,
        table: &mir::InterfaceTable,
        tree: &mir::Tree,
        strings: &StringPool,
        method_name: &str,
    ) -> String {
        self.interface_method_target_name(table, tree, strings, method_name)
            .unwrap_or_else(|| panic!("missing interface method target '{method_name}'"))
    }

    /// Find a type with a matching type metadata name.
    pub(crate) fn find_type_by_metadata_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // scan type display names for a matching name
        for (ty, name_id) in &tree.metadata.types.display_name_by_type {
            if strings.get(*name_id) != name {
                continue;
            }

            return Some(*ty);
        }

        None
    }

    /// Find a type with a matching type metadata name or panic.
    pub(crate) fn type_by_metadata_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> mir::LocalNodeId<mir::Type> {
        self.find_type_by_metadata_name(tree, strings, name)
            .unwrap_or_else(|| panic!("missing type metadata name '{name}'"))
    }

    /// Resolve lineage metadata for a type id or panic.
    pub(crate) fn type_lineage<'a>(
        &self,
        tree: &'a mir::Tree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> &'a mir::TypeLineage {
        tree.metadata
            .types
            .lineage(type_id)
            .unwrap_or_else(|| panic!("missing lineage metadata for '{type_id:?}'"))
    }

    /// Resolve a parent type from lineage metadata or panic.
    pub(crate) fn type_parent(
        &self,
        tree: &mir::Tree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        let lineage = self.type_lineage(tree, type_id);
        lineage
            .parent
            .unwrap_or_else(|| panic!("missing parent type for '{type_id:?}'"))
    }

    /// Resolve a vtable for a type or panic.
    pub(crate) fn type_vtable<'a>(
        &self,
        tree: &'a mir::Tree,
        type_id: mir::LocalNodeId<mir::Type>,
    ) -> &'a mir::Vtable {
        tree.metadata
            .dispatch
            .vtable(type_id)
            .unwrap_or_else(|| panic!("missing vtable for '{type_id:?}'"))
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
        let profile = self.profile_id();
        let dir = self.artifact_dir(module_id, profile);
        let tree = &dir.tree;
        let types = &dir.types;

        // scan for the named function declaration
        for (_, declaration) in tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };

            let Some(name) = declaration.name else {
                continue;
            };

            let name_str = self.program.strings.get(name.string());
            if name_str != function_name {
                continue;
            }

            let parameter_id = declaration.signature.parameters.get(index)?;
            let parameter: &dir::Parameter = tree.get(*parameter_id);
            let symbol_id = parameter.symbol();
            let global_symbol = dir::GlobalSymbolId::new(module_id, symbol_id);
            return types.get_value_type_id(global_symbol);
        }

        None
    }

    /// Resolve the field type id for a struct field name.
    pub(crate) fn struct_field_type_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
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
                return field.ty.ty();
            }
        }

        None
    }

    /// Resolve the field type id for a struct field name or panic.
    pub(crate) fn expect_struct_field_type_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> mir::LocalNodeId<mir::Type> {
        self.struct_field_type_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing struct field '{field_name}'"))
    }

    /// Resolve the byte offset for a struct field name.
    pub(crate) fn struct_field_offset_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> Option<u32> {
        // load the struct type
        let mir::Type::Struct { fields, .. } = tree.get(struct_type) else {
            return None;
        };

        // locate the requested field name
        let expected_name = fields.iter().find_map(|field_id| {
            let field = tree.get(*field_id);
            field
                .name
                .and_then(|name| (strings.get(name) == field_name).then_some(name))
        })?;

        // resolve the layout metadata for offsets
        let layout_id = tree.metadata.layout.layout_id(struct_type)?;
        let layout = tree.metadata.layout.layout_table.layout(layout_id);
        layout
            .shape
            .fields()
            .iter()
            .find(|field| field.name == Some(expected_name))
            .map(|field| field.offset)
    }

    /// Resolve the byte offset for a struct field name or panic.
    pub(crate) fn expect_struct_field_offset_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> u32 {
        self.struct_field_offset_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing struct field offset '{field_name}'"))
    }

    /// Resolve one struct field by field name.
    pub(crate) fn struct_field_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> Option<mir::LocalNodeId<mir::Field>> {
        let mir::Type::Struct { fields, .. } = tree.get(struct_type) else {
            return None;
        };

        fields.iter().find_map(|field_id| {
            let field = tree.get(*field_id);
            field
                .name
                .and_then(|name| (strings.get(name) == field_name).then_some(*field_id))
        })
    }

    /// Resolve one struct field by field name or panic.
    pub(crate) fn expect_struct_field_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        struct_type: mir::LocalNodeId<mir::Type>,
        field_name: &str,
    ) -> mir::LocalNodeId<mir::Field> {
        self.struct_field_by_name(tree, strings, struct_type, field_name)
            .unwrap_or_else(|| panic!("missing struct field '{field_name}'"))
    }

    /// Resolve a field name for a field id or panic.
    pub(crate) fn field_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        field_id: mir::LocalNodeId<mir::Field>,
    ) -> String {
        tree.get(field_id)
            .name
            .map(|name| strings.get(name).to_string())
            .unwrap_or_else(|| panic!("missing field name for '{field_id:?}'"))
    }

    /// Resolve a MIR function id by name or panic.
    pub(crate) fn function_id_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> mir::LocalNodeId<mir::Function> {
        self.find_function_by_name(tree, strings, name)
            .unwrap_or_else(|| panic!("missing function '{name}'"))
    }

    /// Resolve a MIR function by name or panic.
    pub(crate) fn function_by_name<'a>(
        &self,
        tree: &'a mir::Tree,
        strings: &StringPool,
        name: &str,
    ) -> &'a mir::Function {
        let function_id = self.function_id_by_name(tree, strings, name);
        tree.get(function_id)
    }

    /// Find a MIR function id by name.
    pub(crate) fn find_function_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
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
        tree: &mir::Tree,
        strings: &StringPool,
        name: &str,
        index: usize,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // find the matching function
        let function_id = self.find_function_by_name(tree, strings, name)?;
        let function = tree.get(function_id);

        // resolve the parameter type
        function
            .parameters
            .get(index)
            .and_then(|param| param.ty.ty())
    }

    /// Find the first interface dispatch call in a function body.
    pub(crate) fn find_interface_call_info(
        &self,
        tree: &mir::Tree,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<InterfaceCall> {
        // scan call instructions for interface dispatch
        let function = tree.get(function_id);
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::CallInterface {
                    interface,
                    slot,
                    ..
                } = tree.get(*instruction_id)
                {
                    return Some(InterfaceCall {
                        interface: interface
                            .ty()
                            .expect("interface call should name a concrete interface type"),
                        slot: *slot,
                    });
                }
            }
        }

        None
    }

    /// Find the first class dispatch call in a function body.
    pub(crate) fn find_class_call_info(
        &self,
        tree: &mir::Tree,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<ClassCall> {
        // scan call instructions for class dispatch
        let function = tree.get(function_id);
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::CallClass {
                    class,
                    slot,
                    ..
                } = tree.get(*instruction_id)
                {
                    return Some(ClassCall {
                        class: class
                            .ty()
                            .expect("class call should name a concrete class type"),
                        slot: *slot,
                    });
                }
            }
        }

        None
    }

    /// Find interface dispatch calls for a function name.
    pub(crate) fn find_interface_call_info_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
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
        tree: &mir::Tree,
        strings: &StringPool,
        function_name: &str,
    ) -> InterfaceCall {
        self.find_interface_call_info_by_name(tree, strings, function_name)
            .unwrap_or_else(|| panic!("missing interface call info '{function_name}'"))
    }

    /// Find class dispatch calls for a function name.
    pub(crate) fn find_class_call_info_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        function_name: &str,
    ) -> Option<ClassCall> {
        // scan the function for class dispatch
        let function_id = self.find_function_by_name(tree, strings, function_name)?;
        self.find_class_call_info(tree, function_id)
    }

    /// Find class dispatch calls for a function name or panic.
    pub(crate) fn class_call_info_by_name(
        &self,
        tree: &mir::Tree,
        strings: &StringPool,
        function_name: &str,
    ) -> ClassCall {
        self.find_class_call_info_by_name(tree, strings, function_name)
            .unwrap_or_else(|| panic!("missing class call info '{function_name}'"))
    }
}
