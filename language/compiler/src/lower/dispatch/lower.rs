use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{LowerError, LowerResult};

impl ModuleLowerer<'_> {
    /// Create dispatch storage and slot maps.
    pub(crate) fn declare_dispatch(&mut self) -> LowerResult<()> {
        if self.dispatch_declared {
            return Ok(());
        }

        // collect class symbols in declaration order
        let mut class_symbols = Vec::new();
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Class(_) = declaration else {
                continue;
            };
            class_symbols.push(self.require_symbol_for_node(declaration_id)?);
        }

        // deduplicate and sort for determinism
        class_symbols.sort_by_key(|symbol| symbol.local_id.id);
        class_symbols.dedup();

        // compute the set of classes that require vtable headers
        let vtable_layout_symbols = self.collect_vtable_layout_symbols()?;
        self.vtable_layout_symbols = Some(vtable_layout_symbols);

        // build vtable globals and method slots
        for symbol in class_symbols {
            let slots = self.virtual_method_slots_for_class(symbol)?;
            let vtable_layout_symbols =
                self.vtable_layout_symbols
                    .as_ref()
                    .ok_or_else(|| LowerError::Internal {
                        anchor: (self.module_id).into(),
                        module: self.module_id,
                        message: "missing vtable layout symbols".to_string(),
                    })?;

            if !vtable_layout_symbols.contains(&symbol) {
                continue;
            }

            self.vtable_class_symbols.push(symbol);

            // resolve the declaration id for this class symbol
            let declaration_id = self.declaration_ids_for_symbol(symbol).first().copied();
            let Some(declaration_id) = declaration_id else {
                return Err(LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: format!("missing class declaration for vtable symbol {symbol:?}"),
                }
                .into());
            };
            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));

            let vtable_global =
                self.create_vtable_global(symbol, slots.len() as u64 + 2, anchor)?;
            self.insert_vtable_global(symbol, vtable_global)?;

            for (index, slot) in slots.iter().enumerate() {
                let dispatch_slot = index as u32 + 2;
                self.insert_virtual_method_slot(
                    symbol,
                    slot.key(),
                    dispatch_slot,
                    slot.member_id(),
                )?;
            }
        }

        // precompute dynamic members for dynamic constraints
        let mut constraint_symbols = Vec::new();
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let dir::Declaration::Interface(_) = declaration else {
                continue;
            };
            constraint_symbols.push(self.require_symbol_for_node(declaration_id)?);
        }

        // deduplicate and sort for determinism
        constraint_symbols.sort_by_key(|symbol| symbol.local_id.id);
        constraint_symbols.dedup();

        for symbol in constraint_symbols {
            self.lower_dynamic_members(symbol)?;
        }

        // precompute dynamic table pairs
        let mut pairs = self.collect_dynamic_pairs();
        pairs.sort_by_key(|(concrete, constraint)| (concrete.local_id.id, constraint.local_id.id));
        pairs.dedup();

        self.dynamic_table_pairs = pairs.clone();

        for pair in pairs {
            // create the static dynamic table backing store
            let (concrete, constraint) = pair;
            let declaration_id = self.declaration_ids_for_symbol(constraint).first().copied();
            let Some(declaration_id) = declaration_id else {
                return Err(LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: "dynamic constraint declaration missing for table global".to_string(),
                }
                .into());
            };
            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            let slots = self.lower_dynamic_members(constraint)?;
            let dynamic_table_global = self.create_dynamic_table_global(
                concrete,
                constraint,
                mir::DynamicTable::storage_len(slots.len()) as u64,
                anchor,
            )?;
            self.insert_dynamic_table_global(pair, dynamic_table_global)?;
        }

        // publish the guard after every declaration is complete
        self.dispatch_declared = true;

        Ok(())
    }

    /// Write dispatch tables after function bodies are lowered.
    pub(crate) fn emit_dispatch(&mut self) -> LowerResult<()> {
        self.emit_vtables()?;
        self.emit_dynamic_tables()?;

        Ok(())
    }
}
