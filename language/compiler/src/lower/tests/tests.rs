use destack_base::ImmutableStringPool;
use destack_mir as mir;

use crate::TestProgram;

impl TestProgram {
    /// Collect class dispatch tables from a MIR tree.
    pub(crate) fn class_dispatch_tables<'a>(
        &self,
        tree: &'a mir::NodeTree,
    ) -> Vec<&'a mir::DispatchTable> {
        // filter tables by kind
        tree.type_table
            .dispatch_tables
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
            .dispatch_tables
            .tables
            .iter()
            .filter(|table| matches!(table.kind, mir::DispatchTableKind::Interface { .. }))
            .collect()
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

    /// Find a class vtable whose struct type contains the requested field name.
    pub(crate) fn find_class_vtable_by_field_name<'a>(
        &self,
        tree: &'a mir::NodeTree,
        strings: &ImmutableStringPool,
        vtables: &'a [&'a mir::DispatchTable],
        field_name: &str,
    ) -> Option<&'a mir::DispatchTable> {
        // scan vtables for matching struct fields
        for table in vtables {
            let mir::DispatchTableKind::Class { ty } = table.kind else {
                continue;
            };

            // read the struct fields for the class type
            let mir::Type::Struct { fields, .. } = tree.get(ty) else {
                continue;
            };

            // check for a matching field name
            let has_field = fields.iter().any(|field_id| {
                let field = tree.get(*field_id);
                field
                    .name
                    .map(|name| strings.get(name) == field_name)
                    .unwrap_or(false)
            });

            if has_field {
                return Some(*table);
            }
        }

        None
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

    /// Find the first interface dispatch metadata in a function body.
    pub(crate) fn find_interface_call_metadata(
        &self,
        tree: &mir::NodeTree,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<mir::CallMetadata> {
        // scan call instructions for interface metadata
        let function = tree.get(function_id);
        for block_id in &function.blocks {
            let block = tree.get(*block_id);
            for instruction_id in &block.instructions {
                let mir::Instruction::Call { .. } = tree.get(*instruction_id) else {
                    continue;
                };

                let Some(metadata) = tree.call_table.call_metadata(*instruction_id) else {
                    continue;
                };

                if matches!(metadata.dispatch, mir::CallDispatchKind::Interface { .. }) {
                    return Some(metadata.clone());
                }
            }
        }

        None
    }

    /// Find interface dispatch metadata for a function name.
    pub(crate) fn find_interface_call_metadata_by_name(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        function_name: &str,
    ) -> Option<mir::CallMetadata> {
        // resolve the function id by name
        let function_id = self.find_function_by_name(tree, strings, function_name)?;

        // scan the function for interface metadata
        self.find_interface_call_metadata(tree, function_id)
    }
}
