use std::collections::HashSet;

use destack_base::StringId;
use destack_dir::{FunctionAbstraction, FunctionMode, GlobalSymbolId, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

/// A key that identifies a virtual method slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VirtualMethodKey {
    /// The method name.
    name: StringId,
    /// The signature type id for the method.
    signature: dir::LocalTypeId,
}

/// A virtual method candidate for vtable construction.
#[derive(Debug, Clone)]
struct VirtualMethodDescriptor {
    /// The slot identity for overrides.
    key: VirtualMethodKey,
    /// The abstraction mode for override handling.
    abstraction: FunctionAbstraction,
    /// The member node for diagnostics.
    member_id: LocalNodeId<Member>,
    /// The method symbol for this implementation.
    symbol: GlobalSymbolId,
}

impl ModuleLowerer<'_> {
    /// Predeclare virtual dispatch slots for method call metadata.
    pub(crate) fn predeclare_virtual_dispatch(&mut self) -> LowerResult<()> {
        // collect class symbols in declaration order
        let mut class_symbols = Vec::new();
        for (_declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Class { descriptor, .. } = declaration else {
                continue;
            };
            class_symbols.push(descriptor.symbol.into_global(self.module_id));
        }

        // deduplicate and sort for determinism
        class_symbols.sort_by_key(|symbol| symbol.local_id.id);
        class_symbols.dedup();

        self.vtable_class_symbols.clear();
        self.virtual_method_slots_by_symbol.clear();

        for symbol in class_symbols {
            let slots = self.virtual_method_slots_for_class(symbol)?;
            if slots.is_empty() {
                continue;
            }

            self.vtable_class_symbols.push(symbol);

            for (index, slot) in slots.iter().enumerate() {
                let slot_id = index as u32 + 2;
                self.virtual_method_slots_by_symbol
                    .insert(slot.symbol, slot_id);
            }
        }

        Ok(())
    }

    /// Lower vtables for classes that require virtual dispatch.
    pub(crate) fn lower_vtables(&mut self) -> LowerResult<()> {
        // generate vtables for each class
        for symbol in self.vtable_class_symbols.clone() {
            self.lower_vtable_for_class(symbol)?;
        }

        Ok(())
    }

    /// Lower the vtable for a single class symbol.
    fn lower_vtable_for_class(&mut self, symbol: GlobalSymbolId) -> LowerResult<()> {
        // collect a diagnostic anchor
        let anchor = self
            .declaration_ids_for_symbol(symbol)
            .first()
            .copied()
            .map(|id| {
                id.into_global_any(self.module_id)
                    .into_anchored(Some(self.profile))
            });
        let Some(anchor) = anchor else {
            return Ok(());
        };

        // collect virtual methods in lineage order
        let virtual_slots = self.virtual_method_slots_for_class(symbol)?;

        // skip when no virtual methods exist
        if virtual_slots.is_empty() {
            return Ok(());
        }

        // resolve the class instance mir type
        let instance_type_id = self.types.get_instance_type_id(symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: anchor,
                message: "class missing instance type".to_string(),
            }
        })?;
        let mir_type = *self
            .type_lowerer
            .type_cache
            .get(&instance_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: anchor,
                message: "class instance layout not lowered".to_string(),
            })?;

        // resolve drop glue when available
        let drop_function = self
            .builder
            .tree()
            .type_table
            .drop_function_by_type_id
            .get(&mir_type)
            .copied();

        // build vtable slots with fixed prefix
        let mut slots = Vec::with_capacity(virtual_slots.len() + 2);
        slots.push(mir::DispatchSlot::TypeTag);
        slots.push(mir::DispatchSlot::Destructor {
            function: drop_function,
        });
        for method in virtual_slots {
            let function = self.method_function_id(method.member_id, method.symbol)?;
            slots.push(mir::DispatchSlot::Method { function });
        }

        // insert the dispatch table
        let table_id = {
            let table = mir::DispatchTable {
                kind: mir::DispatchTableKind::Class { ty: mir_type },
                global: None,
                slots,
            };
            self.builder
                .tree_mut()
                .type_table
                .dispatch_tables
                .insert(table)
        };

        // attach the vtable to type metadata
        let type_table = &mut self.builder.tree_mut().type_table;
        let metadata = type_table.type_metadata_by_id.entry(mir_type).or_default();
        metadata.vtable = Some(table_id);

        Ok(())
    }

    /// Collect the class lineage from base to derived.
    pub(crate) fn collect_class_lineage(&self, symbol: GlobalSymbolId) -> Vec<GlobalSymbolId> {
        // walk the extends chain from derived to base
        let mut lineage = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(symbol);

        while let Some(current_symbol) = current {
            if !seen.insert(current_symbol) {
                break;
            }
            lineage.push(current_symbol);
            current = self
                .types
                .get_lineage_for_symbol(current_symbol)
                .and_then(|lineage| lineage.extends);
        }

        // reverse to get base first order
        lineage.reverse();
        lineage
    }

    /// Collect virtual methods declared on a single class.
    fn collect_virtual_methods_for_class(
        &self,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Vec<VirtualMethodDescriptor>> {
        // collect declaration ids for the class symbol
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        let mut methods = Vec::new();

        // scan members in declaration order
        for declaration_id in declaration_ids {
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                dir::Declaration::Class { members, .. } => members,
                _ => continue,
            };

            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                let dir::Member::Method {
                    modifiers,
                    key,
                    signature,
                    symbol: method_symbol,
                    ..
                } = member
                else {
                    continue;
                };

                // skip non virtual methods
                if !self.method_is_virtual(modifiers.as_ref(), signature) {
                    continue;
                }

                // resolve the method name
                let name =
                    self.member_dispatch_name_or_error(key.as_ref(), signature.mode, *member_id)?;

                // resolve the signature type id
                let signature_type_id = self.method_signature_type_id(*member_id)?;
                let key = VirtualMethodKey {
                    name,
                    signature: signature_type_id,
                };

                let method_symbol = method_symbol.into_global(self.module_id);
                methods.push(VirtualMethodDescriptor {
                    key,
                    abstraction: signature.abstraction,
                    member_id: *member_id,
                    symbol: method_symbol,
                });
            }
        }

        Ok(methods)
    }

    /// Return true when a method should participate in virtual dispatch.
    pub(crate) fn method_is_virtual(
        &self,
        modifiers: Option<&dir::BindingModifier>,
        signature: &dir::FunctionSignature,
    ) -> bool {
        // reject static and private methods
        if self.member_is_static(modifiers) || self.member_is_private(modifiers) {
            return false;
        }

        // reject constructor and new members
        if matches!(
            signature.mode,
            Some(FunctionMode::Constructor) | Some(FunctionMode::New)
        ) {
            return false;
        }

        true
    }

    /// Collect virtual method slots for a class in vtable order.
    fn virtual_method_slots_for_class(
        &self,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Vec<VirtualMethodDescriptor>> {
        // collect virtual methods in lineage order
        let lineage = self.collect_class_lineage(symbol);
        let mut virtual_slots = Vec::new();

        // populate slots with override reuse
        for class_symbol in lineage {
            let class_methods = self.collect_virtual_methods_for_class(class_symbol)?;
            for method in class_methods {
                let slot_index = virtual_slots
                    .iter()
                    .position(|slot: &VirtualMethodDescriptor| {
                        slot.key.name == method.key.name
                            && self.types_are_equivalent(slot.key.signature, method.key.signature)
                    });

                match slot_index {
                    Some(index) => {
                        virtual_slots[index] = method;
                    }
                    None => {
                        if matches!(
                            method.abstraction,
                            FunctionAbstraction::ConcreteOverride
                                | FunctionAbstraction::AbstractOverride
                        ) {
                            return Err(LowerError::UnsupportedConstruct {
                                node: method
                                    .member_id
                                    .into_global_any(self.module_id)
                                    .into_anchored(Some(self.profile)),
                                message: "override method has no base slot".to_string(),
                            });
                        }

                        virtual_slots.push(method);
                    }
                }
            }
        }

        Ok(virtual_slots)
    }

    /// Check whether a class has any virtual methods.
    pub(crate) fn class_has_virtual_methods(&self, symbol: GlobalSymbolId) -> LowerResult<bool> {
        let slots = self.virtual_method_slots_for_class(symbol)?;
        Ok(!slots.is_empty())
    }

    /// Resolve the signature type id for a method member.
    pub(crate) fn method_signature_type_id(
        &self,
        member_id: LocalNodeId<Member>,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve the signature type from analysis
        let node_id = member_id.into_global_any(self.module_id);
        let signature_type_id =
            self.types
                .get_signature_type_for_node(node_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: node_id.into_anchored(Some(self.profile)),
                })?;

        Ok(signature_type_id)
    }

    /// Resolve the MIR function id for a method symbol.
    pub(crate) fn method_function_id(
        &self,
        member_id: LocalNodeId<Member>,
        method_symbol: GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // lookup the lowered function by symbol
        let function_id = self
            .functions_by_symbol
            .get(&method_symbol)
            .copied()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing method function".to_string(),
            })?;

        Ok(function_id)
    }

    /// Return true when a member is private.
    pub(crate) fn member_is_private(&self, modifiers: Option<&dir::BindingModifier>) -> bool {
        // check for private visibility
        modifiers.is_some_and(|modifiers| modifiers.visibility == Some(dir::Visibility::Private))
    }
}
