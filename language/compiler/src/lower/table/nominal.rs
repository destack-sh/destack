use std::collections::HashMap;

use destack_dir::AnchoredGlobalNodeId;
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

/// Nominal declaration metadata collected for lowering.
#[derive(Debug, Clone)]
pub(crate) struct NominalType {
    /// Nominal symbol for this declaration group.
    pub(crate) symbol: dir::GlobalSymbolId,
    /// Symbol classification for layout and lineage rules.
    pub(crate) kind: dir::SymbolType,
    /// Anchor for diagnostics.
    pub(crate) anchor: AnchoredGlobalNodeId,
    /// Whether any declaration is abstract.
    pub(crate) is_abstract: bool,
    /// Instance type id when available.
    pub(crate) instance_type_id: Option<dir::LocalTypeId>,
}

impl ModuleLowerer<'_> {
    /// Collect nominal declaration info for this module.
    pub(crate) fn collect_nominal_types(&self) -> Vec<NominalType> {
        // gather nominal declarations by symbol
        let mut type_by_symbol: HashMap<dir::GlobalSymbolId, NominalType> = HashMap::new();
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            let symbol = match declaration {
                dir::Declaration::Struct { descriptor, .. }
                | dir::Declaration::Class { descriptor, .. }
                | dir::Declaration::Enum { descriptor, .. }
                | dir::Declaration::Interface { descriptor, .. } => {
                    descriptor.symbol.into_global(self.module_id)
                }
                dir::Declaration::Type {
                    descriptor, kind, ..
                } => {
                    if !matches!(kind, dir::TypeKind::Nominal) {
                        continue;
                    }
                    descriptor.symbol.into_global(self.module_id)
                }
                _ => continue,
            };

            // collect declaration anchor and abstraction status
            let anchor = declaration_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            let is_abstract =
                declaration.descriptor().abstraction == dir::DeclarationAbstraction::Abstract;

            // merge declaration info across partials
            type_by_symbol
                .entry(symbol)
                .and_modify(|info| {
                    info.is_abstract |= is_abstract;
                })
                .or_insert(NominalType {
                    symbol,
                    kind: symbol.ty(),
                    anchor,
                    is_abstract,
                    instance_type_id: None,
                });
        }

        // resolve instance type ids after collecting declaration info
        type_by_symbol
            .into_values()
            .map(|mut info| {
                info.instance_type_id = self.types.get_instance_type_id(info.symbol);
                info
            })
            .collect()
    }

    /// Resolve a lowered mir type for a local type id.
    pub(crate) fn lower_type(
        &mut self,
        type_id: dir::LocalTypeId,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let dir::Type::Reference { symbol, .. } = self.types.get_type(type_id)
            && matches!(
                symbol.ty(),
                dir::SymbolType::Struct | dir::SymbolType::Class
            )
        {
            let _ = self.lower_nominal_layout(*symbol)?;
        }

        if let Some(symbol) = self.types.symbol_for_instance_type(type_id)
            && matches!(
                symbol.ty(),
                dir::SymbolType::Struct | dir::SymbolType::Class
            )
        {
            let _ = self.lower_nominal_layout(symbol)?;
        }

        // use cached types when available
        if let Some(&mir_type) = self.type_lowerer.type_cache.get(&type_id) {
            return Ok(mir_type);
        }

        // lower the type on demand
        self.type_lowerer.lower_type(
            self.types,
            type_id,
            self.module_id,
            anchor,
            &mut self.builder,
        )
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
