use destack_base::ImmutableStringPool;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

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

    /// Find a struct type that contains all requested field names.
    pub(crate) fn find_struct_type_by_field_names(
        &self,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        field_names: &[&str],
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // scan struct types for matching field names
        for (type_id, ty) in tree.iter_nodes::<mir::Type>() {
            let mir::Type::Struct { fields, .. } = ty else {
                continue;
            };

            // check if all requested field names exist
            let has_all = field_names.iter().all(|name| {
                fields.iter().any(|field_id| {
                    let field = tree.get(*field_id);
                    field
                        .name
                        .map(|field_name| strings.get(field_name) == *name)
                        .unwrap_or(false)
                })
            });

            if has_all {
                return Some(type_id);
            }
        }

        None
    }

    /// Find a type with a matching type metadata name.
    #[allow(dead_code)]
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

    /// Find a struct type by source name.
    #[allow(dead_code)]
    pub(crate) fn find_struct_type_by_name(
        &self,
        module_id: ModuleId,
        tree: &mir::NodeTree,
        strings: &ImmutableStringPool,
        struct_name: &str,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        // load the dir tree for the module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module
            .dir_base_maybe()
            .or_else(|| module.dir_maybe(self.default_profile_id(module_id)))
            .unwrap_or_else(|| {
                panic!("no DIR available for module {module_id:?}");
            });
        let dir_tree = dir.tree.read();
        let types = dir.types.read();

        // locate the struct declaration
        for (_, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            let dir::Declaration::Struct { descriptor, .. } = declaration else {
                continue;
            };

            let Some(name) = descriptor.name else {
                continue;
            };

            let name_id = name.string();
            let name_str = self.program.strings.get(name_id);
            if name_str != struct_name {
                continue;
            }

            let symbol_id = dir::GlobalSymbolId::new(module_id, descriptor.symbol);
            let Some(instance_type_id) = types.get_instance_type_id(symbol_id) else {
                return None;
            };

            // collect field names from the instance type
            let dir::Type::Object { fields, .. } = types.get_type(instance_type_id) else {
                return None;
            };

            let mut names = Vec::with_capacity(fields.len());
            for field in fields {
                let Some(name_id) = field.key.name() else {
                    return None;
                };

                let name_str = self.program.strings.get(name_id);
                names.push(name_str.to_string());
            }

            let name_refs: Vec<&str> = names.iter().map(|name| name.as_str()).collect();
            return self.find_struct_type_by_field_names(tree, strings, &name_refs);
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
