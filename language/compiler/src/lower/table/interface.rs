use std::collections::{HashMap, HashSet};

use destack_base::StringId;
use destack_dir as dir;
use destack_dir::{DynamicKey, GlobalSymbolId, LocalNodeId, Member};

use crate::{LowerError, LowerResult};

use crate::lower::{ModuleLowerer, static_key_to_field_name};

/// A slot in an interface dispatch layout.
#[derive(Debug, Clone)]
pub(crate) enum InterfaceSlot {
    /// A field offset slot for interface property access.
    Field {
        /// The interface field name.
        name: StringId,
        /// The member node for diagnostics.
        member_id: LocalNodeId<Member>,
    },
    /// A method slot for interface method dispatch.
    Method {
        /// The interface method name.
        name: StringId,
        /// The signature type id for the interface method.
        signature: dir::LocalTypeId,
        /// The member node for diagnostics.
        member_id: LocalNodeId<Member>,
    },
}

/// Cache of interface slots keyed by interface symbol.
#[derive(Debug, Default)]
pub(crate) struct InterfaceDispatchCache {
    /// Cached slots for each interface symbol.
    slots_by_interface: HashMap<GlobalSymbolId, Vec<InterfaceSlot>>,
}

impl InterfaceDispatchCache {
    /// Create an empty interface dispatch cache.
    pub(crate) fn new() -> Self {
        Self {
            slots_by_interface: HashMap::new(),
        }
    }

    /// Return cached slots for an interface symbol.
    pub(crate) fn slots(&self, interface: GlobalSymbolId) -> Option<&[InterfaceSlot]> {
        self.slots_by_interface
            .get(&interface)
            .map(|slots| slots.as_slice())
    }

    /// Insert slots for an interface symbol.
    pub(crate) fn insert(&mut self, interface: GlobalSymbolId, slots: Vec<InterfaceSlot>) {
        self.slots_by_interface.insert(interface, slots);
    }
}

impl ModuleLowerer<'_> {
    /// Predeclare interface dispatch layouts for this module.
    pub(crate) fn predeclare_interface_dispatch(&mut self) -> LowerResult<()> {
        // scan interface declarations for dispatch layouts
        for (_declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            // skip non interface declarations
            let dir::Declaration::Interface { descriptor, .. } = declaration else {
                continue;
            };

            // resolve the interface symbol
            let interface_symbol = descriptor.symbol.into_global(self.module_id);

            // skip cached interfaces
            if self.interface_dispatch.slots(interface_symbol).is_some() {
                continue;
            }

            // collect slots and cache
            let slots = self.collect_interface_slots(interface_symbol)?;
            self.interface_dispatch.insert(interface_symbol, slots);
        }

        Ok(())
    }

    /// Return cached interface slots or compute them on demand.
    pub(crate) fn interface_slots_for_symbol(
        &mut self,
        interface: GlobalSymbolId,
    ) -> LowerResult<Vec<InterfaceSlot>> {
        // return cached slots when available
        if let Some(slots) = self.interface_dispatch.slots(interface) {
            let slots: Vec<InterfaceSlot> = slots.to_vec();
            return Ok(slots);
        }

        // compute slots for the interface symbol
        let slots = self.collect_interface_slots(interface)?;
        self.interface_dispatch.insert(interface, slots.clone());
        Ok(slots)
    }

    /// Collect interface member slots in declaration order.
    fn collect_interface_slots(
        &mut self,
        interface: GlobalSymbolId,
    ) -> LowerResult<Vec<InterfaceSlot>> {
        // seed the collection state
        let mut slots = Vec::new();
        let mut seen_fields = HashMap::new();
        let mut seen_methods = HashMap::new();
        let mut visited = HashSet::new();

        // collect members across the interface lineage
        self.collect_interface_slots_recursive(
            interface,
            &mut slots,
            &mut seen_fields,
            &mut seen_methods,
            &mut visited,
        )?;

        Ok(slots)
    }

    /// Collect interface slots with inheritance ordering.
    fn collect_interface_slots_recursive(
        &mut self,
        interface: GlobalSymbolId,
        slots: &mut Vec<InterfaceSlot>,
        seen_fields: &mut HashMap<StringId, dir::LocalTypeId>,
        seen_methods: &mut HashMap<StringId, Vec<dir::LocalTypeId>>,
        visited: &mut HashSet<GlobalSymbolId>,
    ) -> LowerResult<()> {
        // avoid cycles in interface inheritance
        if !visited.insert(interface) {
            return Ok(());
        }

        // visit base interface first
        if let Some(lineage) = self.types.get_lineage_for_symbol(interface)
            && let Some(base) = lineage.extends
        {
            self.collect_interface_slots_recursive(
                base,
                slots,
                seen_fields,
                seen_methods,
                visited,
            )?;
        }

        // collect local interface members
        let declaration_ids = self.declaration_ids_for_symbol(interface);
        for declaration_id in declaration_ids {
            // load interface members from the declaration
            let declaration = self.dir_tree.get(declaration_id);
            let members = match declaration {
                dir::Declaration::Interface { members, .. } => members,
                _ => continue,
            };

            // scan interface members
            for member_id in members {
                self.collect_interface_member_slots(*member_id, slots, seen_fields, seen_methods)?;
            }
        }

        Ok(())
    }

    /// Collect slots for a single interface member.
    fn collect_interface_member_slots(
        &mut self,
        member_id: LocalNodeId<Member>,
        slots: &mut Vec<InterfaceSlot>,
        seen_fields: &mut HashMap<StringId, dir::LocalTypeId>,
        seen_methods: &mut HashMap<StringId, Vec<dir::LocalTypeId>>,
    ) -> LowerResult<()> {
        // load member data
        let member = self.dir_tree.get(member_id);

        // handle member slots by kind
        match member {
            dir::Member::Field {
                modifiers,
                key,
                value,
                ..
            } => {
                // reject index signatures for native lowering
                if matches!(key, Some(DynamicKey::NamedExpression { .. })) {
                    return Err(LowerError::UnsupportedConstruct {
                        node: member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "index signatures are not supported for native lowering"
                            .to_string(),
                    });
                }

                // skip non instance fields
                if self.member_is_static(modifiers.as_ref())
                    || self.member_is_private(modifiers.as_ref())
                {
                    return Ok(());
                }

                // resolve the field name
                let field_name = self.interface_field_name(member_id, key)?;

                // resolve the field type
                let field_type = value
                    .and_then(|value_id| {
                        self.types.get_declared_or_inferred_type_id(
                            value_id.into_global_any(self.module_id),
                        )
                    })
                    .ok_or_else(|| LowerError::MissingType {
                        node: member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    })?;

                // reuse matching field slots when possible
                if let Some(existing_type) = seen_fields.get(&field_name) {
                    if !self.types_are_equivalent(*existing_type, field_type) {
                        return Err(LowerError::UnsupportedConstruct {
                            node: member_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                            message: "interface field type mismatch".to_string(),
                        });
                    }

                    return Ok(());
                }

                // record the field slot
                seen_fields.insert(field_name, field_type);
                slots.push(InterfaceSlot::Field {
                    name: field_name,
                    member_id,
                });
            }
            dir::Member::Method {
                modifiers,
                key,
                signature,
                ..
            } => {
                // skip non instance methods
                if self.member_is_static(modifiers.as_ref())
                    || self.member_is_private(modifiers.as_ref())
                {
                    return Ok(());
                }

                // resolve the method name
                let method_name =
                    self.member_dispatch_name_or_error(key.as_ref(), signature.mode, member_id)?;

                // resolve the method signature type
                let signature_type_id = self.method_signature_type_id(member_id)?;

                // reuse matching method slots when possible
                let entries = seen_methods.entry(method_name).or_default();
                if entries
                    .iter()
                    .any(|existing| self.types_are_equivalent(*existing, signature_type_id))
                {
                    return Ok(());
                }

                // record the method slot
                entries.push(signature_type_id);
                slots.push(InterfaceSlot::Method {
                    name: method_name,
                    signature: signature_type_id,
                    member_id,
                });
            }
            _ => {}
        }

        Ok(())
    }

    /// Compute the field name for an interface member key.
    fn interface_field_name(
        &mut self,
        member_id: LocalNodeId<Member>,
        key: &Option<DynamicKey>,
    ) -> LowerResult<StringId> {
        // resolve the static key for the field
        let Some(key) = key.as_ref().and_then(|key| {
            self.compiler.static_key_from_dynamic_key(
                self.profile,
                *key,
                self.dir_tree,
                self.symbols,
                self.types,
            )
        }) else {
            return Err(LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "interface field must have a static name".to_string(),
            });
        };

        Ok(static_key_to_field_name(&key, &mut self.builder))
    }
}
