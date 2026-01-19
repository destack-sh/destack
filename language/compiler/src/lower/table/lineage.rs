use std::collections::HashSet;

use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Record lineage metadata for nominal types.
    pub(crate) fn lower_lineage_metadata(&mut self) -> LowerResult<()> {
        // collect nominal declaration metadata for this module
        let mut nominal_info = self.collect_nominal_types();
        nominal_info.sort_by_key(|info| info.symbol.local_id.id);

        for info in nominal_info {
            // resolve the instance type
            let Some(instance_type_id) = info.instance_type_id else {
                continue;
            };
            let mir_type = self.lower_type(instance_type_id, info.anchor)?;

            // resolve the lineage data for the symbol
            let lineage = self.types.get_lineage_for_symbol(info.symbol);
            let parent_symbol = lineage
                .and_then(|lineage| lineage.extends)
                .filter(|_| info.kind == dir::SymbolType::Class);

            // resolve the parent mir type when present
            let parent = if let Some(parent_symbol) = parent_symbol {
                self.lower_instance_type(parent_symbol, info.anchor)?
            } else {
                None
            };

            // collect implemented interfaces
            let mut interface_symbols = Vec::new();
            let mut seen_interfaces = HashSet::new();
            if let Some(lineage) = lineage {
                // include base interface lineage for interfaces
                if info.kind == dir::SymbolType::Interface
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
                let Some(interface_type) =
                    self.lower_instance_type(interface_symbol, info.anchor)?
                else {
                    continue;
                };
                interfaces.push(interface_type);
            }

            // record lineage metadata
            let is_interface = info.kind == dir::SymbolType::Interface;
            let is_abstract = info.is_abstract || is_interface;
            let type_table = &mut self.builder.tree_mut().type_table;
            let metadata = type_table.type_metadata_by_id.entry(mir_type).or_default();
            metadata.lineage = Some(mir::TypeLineage {
                parent,
                interfaces,
                is_sealed: false,
                is_final: false,
                is_abstract,
                is_interface,
            });
        }

        Ok(())
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
