use std::collections::HashSet;
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Return lineage metadata for a nominal symbol and instance type.
    pub(crate) fn lineage_metadata_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::TypeLineage> {
        // skip if metadata already exists
        if let Some(lineage) = self
            .builder
            .tree()
            .metadata
            .layout
            .lineage(mir_type)
            .cloned()
        {
            return Ok(lineage);
        }

        // resolve the lineage data for the symbol
        let lineage = self.types.get_lineage_for_symbol(symbol);
        let parent_symbol = lineage
            .and_then(|lineage| lineage.extends)
            .filter(|_| symbol.ty() == dir::SymbolType::Class);

        // resolve the parent mir type when present
        let parent = if let Some(parent_symbol) = parent_symbol {
            self.lower_instance_type(parent_symbol, anchor)?
        } else {
            None
        };

        // collect implemented interfaces
        let mut interface_symbols = Vec::new();
        let mut seen_interfaces = HashSet::new();
        if let Some(lineage) = lineage {
            // include base interface lineage for interfaces
            if symbol.ty() == dir::SymbolType::Interface
                && let Some(base) = lineage.extends
            {
                self.collect_interface_lineage_symbols(
                    base,
                    &mut interface_symbols,
                    &mut seen_interfaces,
                );
            }

            // include implemented interfaces for nominal types
            for interface_symbol in &lineage.implements {
                self.collect_interface_lineage_symbols(
                    *interface_symbol,
                    &mut interface_symbols,
                    &mut seen_interfaces,
                );
            }
        }

        // lower interface instance types for metadata
        let mut interfaces = Vec::new();
        for interface_symbol in interface_symbols {
            // skip interfaces that cannot be lowered
            let Some(interface_type) = self.lower_instance_type(interface_symbol, anchor)? else {
                continue;
            };
            interfaces.push(interface_type);
        }

        // record lineage metadata
        let is_interface = symbol.ty() == dir::SymbolType::Interface;
        let is_abstract = is_interface
            || self
                .declaration_ids_for_symbol(symbol)
                .iter()
                .any(|declaration_id| match self.dir_tree.get(*declaration_id) {
                    dir::Declaration::Class(declaration) => declaration.is_abstract,
                    _ => false,
                });
        let type_lineage = mir::TypeLineage {
            parent,
            interfaces,
            is_sealed: false,
            is_final: false,
            is_abstract,
            is_interface,
        };
        self.builder
            .tree_mut()
            .metadata
            .layout
            .set_lineage(mir_type, type_lineage.clone());

        Ok(type_lineage)
    }

    /// Collect interface lineage in base to derived order.
    pub(crate) fn collect_interface_lineage_symbols(
        &self,
        interface: dir::GlobalSymbolId,
        order: &mut Vec<dir::GlobalSymbolId>,
        seen: &mut HashSet<dir::GlobalSymbolId>,
    ) {
        if !seen.insert(interface) {
            return;
        }

        // visit the base
        if let Some(lineage) = self.types.get_lineage_for_symbol(interface)
            && let Some(base) = lineage.extends
        {
            self.collect_interface_lineage_symbols(base, order, seen);
        };

        // append the interface after base types
        order.push(interface);
    }
}
