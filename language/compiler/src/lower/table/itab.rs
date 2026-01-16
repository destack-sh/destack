use std::collections::HashSet;

use destack_base::StringId;
use destack_dir::{GlobalSymbolId, LocalNodeId, Member};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;
use crate::lower::table::interface::InterfaceSlot;

impl ModuleLowerer<'_> {
    /// Lower itabs for interface dispatch.
    pub(crate) fn lower_itabs(&mut self) -> LowerResult<()> {
        // collect concrete to interface pairs
        let mut pairs = Vec::new();

        // scan type lineages for concrete symbols
        for (symbol, lineage) in self.types.iter_lineages() {
            // skip non nominal types
            if !matches!(
                symbol.ty(),
                dir::SymbolType::Class | dir::SymbolType::Struct
            ) {
                continue;
            }

            // collect interfaces in declaration order
            let mut ordered_interfaces = Vec::new();
            let mut seen_interfaces = HashSet::new();

            // expand interface lineage
            for interface_symbol in &lineage.implements {
                self.collect_interface_lineage(
                    *interface_symbol,
                    &mut ordered_interfaces,
                    &mut seen_interfaces,
                );
            }

            // record concrete interface pairs
            for interface_symbol in ordered_interfaces {
                pairs.push((symbol, interface_symbol));
            }
        }

        // sort for deterministic output
        pairs.sort_by_key(|(concrete, interface)| (concrete.local_id.id, interface.local_id.id));
        pairs.dedup();

        // generate itabs for each pair
        for (concrete, interface) in pairs {
            self.lower_itab_for_pair(concrete, interface)?;
        }

        Ok(())
    }

    /// Lower a single interface itab for a concrete type.
    fn lower_itab_for_pair(
        &mut self,
        concrete: GlobalSymbolId,
        interface: GlobalSymbolId,
    ) -> LowerResult<()> {
        // collect a diagnostic anchor
        let anchor = self
            .declaration_ids_for_symbol(interface)
            .first()
            .copied()
            .map(|id| {
                id.into_global_any(self.module_id)
                    .into_anchored(Some(self.profile))
            });

        // skip interfaces without declarations
        let Some(anchor) = anchor else {
            return Ok(());
        };

        // collect interface slots in declaration order
        let interface_slots = self.interface_slots_for_symbol(interface)?;

        // resolve concrete instance type
        let instance_type_id = self.types.get_instance_type_id(concrete).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: anchor,
                message: "missing concrete instance type".to_string(),
            }
        })?;

        // resolve concrete layout from cache
        let concrete_mir_type = *self
            .type_lowerer
            .type_cache
            .get(&instance_type_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: anchor,
                message: "concrete layout not lowered".to_string(),
            })?;

        // resolve interface instance type
        let interface_type_id = self.types.get_instance_type_id(interface).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node: anchor,
                message: "missing interface instance type".to_string(),
            }
        })?;

        // lower interface instance type
        let interface_mir_type = self.type_lowerer.lower_type(
            self.types,
            interface_type_id,
            self.module_id,
            anchor,
            &mut self.builder,
        )?;

        // build itab slots with fixed prefix
        let mut slots = Vec::with_capacity(interface_slots.len() + 1);
        slots.push(mir::DispatchSlot::TypeTag);

        // append interface slots
        for slot in interface_slots {
            match slot {
                InterfaceSlot::Field {
                    name, member_id, ..
                } => {
                    // resolve field offset slot
                    let offset =
                        self.interface_field_offset(concrete_mir_type, name, member_id, anchor)?;
                    slots.push(mir::DispatchSlot::FieldOffset {
                        field_name: name,
                        offset,
                    });
                }
                InterfaceSlot::Method {
                    name,
                    signature,
                    member_id,
                    ..
                } => {
                    // resolve interface method slot
                    let target =
                        self.interface_method_target(concrete, name, signature, member_id)?;
                    let interface_method = target;
                    slots.push(mir::DispatchSlot::InterfaceMethod {
                        interface_method,
                        target,
                    });
                }
            }
        }

        // insert the dispatch table
        let table_id = {
            let table = mir::DispatchTable {
                kind: mir::DispatchTableKind::Interface {
                    concrete: concrete_mir_type,
                    interface: interface_mir_type,
                },
                global: None,
                slots,
            };
            self.builder
                .tree_mut()
                .type_table
                .dispatch_tables
                .insert(table)
        };

        // attach the itab to type metadata
        let type_table = &mut self.builder.tree_mut().type_table;
        let metadata = type_table
            .type_metadata_by_id
            .entry(concrete_mir_type)
            .or_default();
        metadata.itabs.push(table_id);

        Ok(())
    }

    /// Resolve the concrete field offset for an interface field.
    fn interface_field_offset(
        &self,
        concrete_mir_type: mir::LocalNodeId<mir::Type>,
        field_name: StringId,
        member_id: LocalNodeId<Member>,
        anchor: destack_dir::AnchoredGlobalNodeId,
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
                    node: member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "interface field missing on concrete type".to_string(),
                })?;

        // resolve the field offset
        let field_offset = layout
            .field(field_index)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "interface field offset missing".to_string(),
            })?
            .offset;

        Ok(field_offset)
    }

    /// Resolve the concrete method target for an interface method.
    fn interface_method_target(
        &self,
        concrete: GlobalSymbolId,
        method_name: StringId,
        signature_type_id: dir::LocalTypeId,
        member_id: LocalNodeId<Member>,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // decide the search order for concrete methods
        let mut symbols = Vec::new();
        if concrete.ty() == dir::SymbolType::Class {
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
                    dir::Declaration::Class { members, .. }
                    | dir::Declaration::Struct { members, .. } => members,
                    _ => continue,
                };

                // scan members for a matching method
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

                    // skip non instance members
                    if self.member_is_static(modifiers.as_ref())
                        || self.member_is_private(modifiers.as_ref())
                    {
                        continue;
                    }

                    // skip constructor style signatures
                    if matches!(
                        signature.mode,
                        Some(dir::FunctionMode::Constructor) | Some(dir::FunctionMode::New)
                    ) {
                        continue;
                    }

                    // match the method name
                    let name = self.member_name_or_error(key.as_ref(), *member_id)?;
                    if name != method_name {
                        continue;
                    }

                    // match the method signature
                    let signature_id = self.method_signature_type_id(*member_id)?;
                    if !self.types_are_equivalent(signature_id, signature_type_id) {
                        continue;
                    }

                    // resolve the method function id
                    let method_symbol = method_symbol.into_global(self.module_id);
                    let function_id = self.method_function_id(*member_id, method_symbol)?;
                    return Ok(function_id);
                }
            }
        }

        // report missing implementation
        Err(LowerError::UnsupportedConstruct {
            node: member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile)),
            message: "missing interface method implementation".to_string(),
        })
    }

    /// Collect interface lineage in base to derived order.
    fn collect_interface_lineage(
        &self,
        interface: GlobalSymbolId,
        order: &mut Vec<GlobalSymbolId>,
        seen: &mut HashSet<GlobalSymbolId>,
    ) {
        // avoid duplicate interfaces
        if !seen.insert(interface) {
            return;
        }

        // visit the base interface first
        if let Some(lineage) = self.types.get_lineage_for_symbol(interface)
            && let Some(base) = lineage.extends
        {
            self.collect_interface_lineage(base, order, seen);
        }

        // append the interface after base types
        order.push(interface);
    }
}
