use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{LowerError, LowerResult};

impl ModuleLowerer<'_> {
    /// Declare deterministic vtable and itab ids for lowering.
    pub(crate) fn declare_dispatch(&mut self) -> LowerResult<()> {
        if self.dispatch_declared {
            return Ok(());
        }
        self.dispatch_declared = true;

        // collect class symbols in declaration order
        let mut class_symbols = Vec::new();
        for (_declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Class(declaration) = declaration else {
                continue;
            };
            class_symbols.push(declaration.symbol.into_global(self.module_id));
        }

        // deduplicate and sort for determinism
        class_symbols.sort_by_key(|symbol| symbol.local_id.id);
        class_symbols.dedup();

        // compute the set of classes that require vtable headers
        let vtable_layout_symbols = self.collect_vtable_layout_symbols()?;
        self.vtable_layout_symbols = Some(vtable_layout_symbols);

        // build vtable globals and slot ids
        for symbol in class_symbols {
            let slots = self.virtual_method_slots_for_class(symbol)?;
            let vtable_layout_symbols =
                self.vtable_layout_symbols
                    .as_ref()
                    .ok_or_else(|| LowerError::Internal {
                        module: self.module_id,
                        message: "missing vtable layout symbols".to_string(),
                    })?;

            if !vtable_layout_symbols.contains(&symbol) {
                continue;
            }

            let vtable_id = mir::VtableId::new(self.vtable_class_symbols.len() as u32);
            self.insert_vtable_id(symbol, vtable_id)?;
            self.vtable_class_symbols.push(symbol);

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

            let vtable_global =
                self.create_vtable_global(symbol, slots.len() as u64 + 2, anchor)?;
            self.insert_vtable_global(symbol, vtable_global)?;

            for (index, slot) in slots.iter().enumerate() {
                let slot_id = index as u32 + 2;
                self.insert_virtual_method_slot(symbol, slot.key(), slot_id, slot.member_id())?;
            }
        }

        // precompute interface slots for interface dispatch
        let mut interface_symbols = Vec::new();
        for (_declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Interface(declaration) = declaration else {
                continue;
            };
            interface_symbols.push(declaration.symbol.into_global(self.module_id));
        }

        // deduplicate and sort for determinism
        interface_symbols.sort_by_key(|symbol| symbol.local_id.id);
        interface_symbols.dedup();

        for symbol in interface_symbols {
            self.lower_interface_slots(symbol)?;
        }

        // precompute interface pair ids for itab lowering
        let mut pairs = self.collect_interface_pairs();
        pairs.sort_by_key(|(concrete, interface)| (concrete.local_id.id, interface.local_id.id));
        pairs.dedup();

        self.interface_itab_pairs = pairs.clone();
        self.interface_itab_ids.clear();

        for (index, pair) in pairs.iter().enumerate() {
            let id = mir::ItabId::new(index as u32);
            self.insert_interface_itab_id(*pair, id)?;
        }

        Ok(())
    }

    /// Emit dispatch tables after body lowering.
    pub(crate) fn emit_dispatch(&mut self) -> LowerResult<(Vec<mir::VtableId>, Vec<mir::ItabId>)> {
        let vtables = self.emit_vtables()?;
        let itabs = self.emit_itabs()?;
        Ok((vtables, itabs))
    }
}
