use destack_dir::AnchoredGlobalNodeId;
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve a lowered mir type for a local type id.
    pub(crate) fn lower_type(
        &mut self,
        type_id: dir::LocalTypeId,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // ensure nominal layouts for reference types
        if let dir::Type::Reference { symbol, .. } = self.types.get_type(type_id)
            && matches!(
                symbol.ty(),
                dir::SymbolType::Struct | dir::SymbolType::Class
            )
        {
            self.lower_nominal_layout(*symbol)?;
        }

        // ensure nominal layouts for instance types
        if let Some(symbol) = self.types.symbol_for_instance_type(type_id)
            && matches!(
                symbol.ty(),
                dir::SymbolType::Struct | dir::SymbolType::Class
            )
        {
            self.lower_nominal_layout(symbol)?;
        }

        // lower the type on demand
        let mir_type = if let Some(mir_type) = self.type_lowerer.cached_type(type_id) {
            mir_type
        } else {
            self.type_lowerer.lower_type(
                self.types,
                type_id,
                self.module_id,
                anchor,
                &mut self.builder,
            )?
        };

        Ok(mir_type)
    }

    /// Resolve a lowered mir type for a nominal symbol.
    pub(crate) fn lower_instance_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // resolve the instance type id
        let Some(instance_type_id) = self.types.get_instance_type_id(symbol) else {
            return Ok(None);
        };

        // lower the instance type
        let mir_type = self.lower_type(instance_type_id, anchor)?;
        Ok(Some(mir_type))
    }
}
