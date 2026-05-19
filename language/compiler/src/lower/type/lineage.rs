use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Return lineage metadata for a nominal symbol and instance type.
    pub(crate) fn lineage_metadata_for_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        mir_type: mir::LocalNodeId<mir::Type>,
        _anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::TypeLineage> {
        // skip if metadata already exists
        if let Some(lineage) = self
            .builder
            .tree()
            .metadata
            .types
            .lineage(mir_type)
            .cloned()
        {
            return Ok(lineage);
        }

        let parent = None;
        let interfaces = Vec::new();

        // record lineage metadata
        let is_interface = self.symbol_is(symbol, dir::SymbolForm::Interface);
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
            .types
            .set_lineage(mir_type, type_lineage.clone());

        Ok(type_lineage)
    }
}
