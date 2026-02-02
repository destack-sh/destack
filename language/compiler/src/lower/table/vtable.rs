use std::collections::HashSet;

use destack_base::StringId;
use destack_dir::{FunctionAbstraction, FunctionMode, GlobalSymbolId, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

/// Suffix for vtable global names.
const VTABLE_GLOBAL_SUFFIX: &str = "#vtable";

/// Predeclared vtable global information for lowering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VtableGlobal {
    /// The global id that backs the vtable data.
    pub(crate) global_id: mir::LocalNodeId<mir::Global>,
    /// The raw pointer type for addressing the vtable global.
    pub(crate) address_type: mir::LocalNodeId<mir::Type>,
}

/// A key that identifies a virtual method slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct VirtualMethodKey {
    /// The method name.
    name: StringId,
    /// The function mode for accessor discrimination.
    mode: Option<FunctionMode>,
    /// The signature type id for the method.
    signature: dir::LocalTypeId,
}

impl VirtualMethodKey {
    /// Create a virtual method key for dispatch lookups.
    pub(crate) fn new(
        name: StringId,
        mode: Option<FunctionMode>,
        signature: dir::LocalTypeId,
    ) -> Self {
        Self {
            name,
            mode,
            signature,
        }
    }

    /// Return the method name for this key.
    pub(crate) fn name(&self) -> StringId {
        self.name
    }

    /// Return the signature type id for this key.
    pub(crate) fn signature(&self) -> dir::LocalTypeId {
        self.signature
    }
}

/// A virtual method candidate for vtable construction.
#[derive(Debug, Clone)]
pub(crate) struct VirtualMethodDescriptor {
    /// The slot identity for overrides.
    key: VirtualMethodKey,
    /// The abstraction mode for override handling.
    abstraction: FunctionAbstraction,
    /// The member node for diagnostics.
    member_id: LocalNodeId<Member>,
    /// The method symbol for this implementation.
    symbol: GlobalSymbolId,
}

impl VirtualMethodDescriptor {
    /// Return the slot key for this method.
    pub(crate) fn key(&self) -> VirtualMethodKey {
        self.key
    }

    /// Return the member id for this slot.
    pub(crate) fn member_id(&self) -> LocalNodeId<Member> {
        self.member_id
    }
}

impl ModuleLowerer<'_> {
    /// Return vtables for classes that require virtual dispatch.
    pub(crate) fn vtables(&mut self) -> LowerResult<Vec<mir::DispatchTableId>> {
        // generate vtables for each class
        let mut tables = Vec::new();
        for symbol in self.vtable_class_symbols.clone() {
            let table_id = self.vtable_for_symbol(symbol)?;
            tables.push(table_id);
        }

        Ok(tables)
    }

    /// Return the vtable for a single class symbol.
    fn vtable_for_symbol(&mut self, symbol: GlobalSymbolId) -> LowerResult<mir::DispatchTableId> {
        if let Some(table_id) = self.vtable_by_symbol.get(&symbol).copied() {
            return Ok(table_id);
        }

        // check for cycles
        if self.vtable_in_progress.contains(&symbol) {
            let anchor = self
                .declaration_ids_for_symbol(symbol)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    module: self.module_id,
                    message: "vtable cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "cycle detected while lowering vtable".to_string(),
            });
        }
        self.vtable_in_progress.insert(symbol);

        // resolve the declaration id for this class symbol
        let declaration_id = self.declaration_ids_for_symbol(symbol).first().copied();
        let Some(declaration_id) = declaration_id else {
            return Err(LowerError::Internal {
                module: self.module_id,
                message: format!("missing class declaration for vtable symbol {symbol:?}"),
            });
        };
        let anchor = declaration_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        // collect virtual methods in lineage order
        let virtual_slots = self.virtual_method_slots_for_class(symbol)?;

        // resolve the class instance mir type
        let instance_type_id = self.types.get_instance_type_id(symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: anchor,
                message: "class missing instance type".to_string(),
            }
        })?;
        let mir_type = self.lower_type(instance_type_id, anchor)?;

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
            let vtable_global = self
                .vtable_globals_by_symbol
                .get(&symbol)
                .copied()
                .ok_or_else(|| LowerError::Internal {
                    module: self.module_id,
                    message: format!("missing vtable global for class {symbol:?}"),
                })?;
            let table_id = self.require_vtable_id(symbol)?;
            let table = mir::DispatchTable {
                kind: mir::DispatchTableKind::Class { ty: mir_type },
                global: Some(vtable_global.global_id),
                slots,
            };
            self.builder
                .tree_mut()
                .type_table
                .dispatch_registry
                .insert_at(table_id, table);
            table_id
        };

        // attach the vtable to type metadata
        let type_table = &mut self.builder.tree_mut().type_table;
        let metadata = type_table.type_metadata_by_id.entry(mir_type).or_default();
        metadata.vtable = Some(table_id);

        // register the lowered vtable table
        self.insert_vtable_table(symbol, table_id)?;
        self.vtable_in_progress.shift_remove(&symbol);

        Ok(table_id)
    }

    /// Create the global backing storage for a class vtable.
    pub(crate) fn create_vtable_global(
        &mut self,
        symbol: GlobalSymbolId,
        slot_count: u64,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<VtableGlobal> {
        // build the qualified name for the vtable global
        let base_name =
            self.qualified_symbol_name(symbol)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: anchor,
                    message: "missing qualified name for vtable global".to_string(),
                })?;
        let name = format!("{base_name}{VTABLE_GLOBAL_SUFFIX}");

        // allocate the vtable storage as an array of nullable raw pointers
        let slot_type = self.builder.type_reference(
            mir::ReferenceKind::Raw,
            self.type_lowerer.ty_void,
            mir::Mutability::Immutable,
            mir::AddressSpace::Generic,
            true,
        );
        let vtable_type = self
            .builder
            .type_array(slot_type, slot_count, mir::Copyability::Trivial);
        let global_id =
            self.builder
                .global_constant(&name, vtable_type, mir::GlobalInitializer::zero());
        let address_type = self.builder.type_raw_pointer(vtable_type);

        Ok(VtableGlobal {
            global_id,
            address_type,
        })
    }

    /// Collect the class lineage from base to derived.
    pub(crate) fn collect_class_lineage(&self, symbol: GlobalSymbolId) -> Vec<GlobalSymbolId> {
        // walk the extends chain from derived to base
        let mut lineage = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(symbol);

        while let Some(current_symbol) = current {
            // stop on cycles
            if !seen.insert(current_symbol) {
                break;
            }

            // record the current symbol
            lineage.push(current_symbol);

            // advance to the base class
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
            // select the class members
            let members = match declaration {
                dir::Declaration::Class { members, .. } => members,
                _ => continue,
            };

            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                // skip non method members
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
                let key = VirtualMethodKey::new(name, signature.mode, signature_type_id);

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
    pub(crate) fn virtual_method_slots_for_class(
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
                // find an existing slot with matching signature
                let slot_index = virtual_slots
                    .iter()
                    .position(|slot: &VirtualMethodDescriptor| {
                        slot.key.name == method.key.name
                            && slot.key.mode == method.key.mode
                            && self.method_signatures_equivalent(
                                slot.key.signature,
                                method.key.signature,
                            )
                    });

                match slot_index {
                    // reuse the slot for overrides
                    Some(index) => {
                        virtual_slots[index] = method;
                    }
                    // append a new slot for fresh methods
                    None => {
                        // reject overrides without a base slot
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
        // report whether any slots exist
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
        let signature_type_id = self.signature_type_id_for_node(node_id)?;

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
