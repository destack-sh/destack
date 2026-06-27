use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Return lineage for a nominal symbol and instance type.
    pub(crate) fn lineage_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        mir_type: mir::LocalNodeId<mir::Type>,
        _anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::TypeLineage> {
        // reuse existing lineage
        if let Some(lineage) = self.builder.types().lineage(mir_type).cloned() {
            return Ok(lineage);
        }

        let parent = None;
        let interfaces = Vec::new();

        // record lineage
        let is_interface = self.symbol_kind_matches(symbol, dir::SymbolKind::Interface);
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
            .types_mut()
            .set_lineage(mir_type, type_lineage.clone());

        Ok(type_lineage)
    }
}
