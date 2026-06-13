use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::check::PlaceAccess;
use crate::{CompilerError, CompilerResult};

/// Inferred types and values keyed by source identity.
#[derive(Debug)]
pub(in crate::check) struct InputTable {
    /// Node types keyed by source node.
    node_types: IndexMap<dir::GlobalNodeIdAny, dir::GlobalTypeId>,
    /// Symbol types keyed by source symbol.
    symbol_types: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Symbol static values keyed by source symbol.
    symbol_values: IndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Active static guard predicates keyed by guarded source node.
    /// Only nodes walked under one or more @if guards have entries.
    node_conditions: IndexMap<dir::GlobalNodeIdAny, SmallVec<[dir::GlobalTypeId; 2]>>,
    /// Place accesses keyed by written place expression node.
    /// Only assignment targets have entries; everything else reads.
    place_accesses: IndexMap<dir::GlobalNodeIdAny, PlaceAccess>,
}

impl InputTable {
    /// Create an empty input table.
    pub(in crate::check) fn new() -> Self {
        Self {
            node_types: IndexMap::new(),
            symbol_types: IndexMap::new(),
            symbol_values: IndexMap::new(),
            node_conditions: IndexMap::new(),
            place_accesses: IndexMap::new(),
        }
    }

    /// Record how syntax accesses one place expression.
    pub(in crate::check) fn set_place_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        access: PlaceAccess,
    ) {
        self.place_accesses.insert(node, access);
    }

    /// Return how syntax accesses one expression.
    pub(in crate::check) fn place_access(&self, node: dir::GlobalNodeIdAny) -> PlaceAccess {
        self.place_accesses
            .get(&node)
            .copied()
            .unwrap_or(PlaceAccess::Read)
    }

    /// Record the active static guard predicates of one source node.
    pub(in crate::check) fn set_node_condition(
        &mut self,
        node: dir::GlobalNodeIdAny,
        predicates: SmallVec<[dir::GlobalTypeId; 2]>,
    ) {
        self.node_conditions.insert(node, predicates);
    }

    /// Return the active static guard predicates of one source node.
    pub(in crate::check) fn node_condition(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> &[dir::GlobalTypeId] {
        self.node_conditions
            .get(&node)
            .map_or(&[], |predicates| predicates.as_slice())
    }

    /// Record the type of one source node.
    pub(in crate::check) fn set_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let previous = self.node_types.insert(node, ty);

        if previous.is_some_and(|previous| previous != ty) {
            return Err(CompilerError::Internal {
                message: format!("check node {node:?} received two types"),
            });
        }

        Ok(())
    }

    /// Record the type of one source symbol.
    pub(in crate::check) fn set_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let previous = self.symbol_types.insert(symbol, ty);

        if previous.is_some_and(|previous| previous != ty) {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two types"),
            });
        }

        Ok(())
    }

    /// Record the static value of one source symbol as a singleton type.
    pub(in crate::check) fn set_symbol_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let previous = self.symbol_values.insert(symbol, value);

        if previous.is_some_and(|previous| previous != value) {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two static values"),
            });
        }

        Ok(())
    }

    /// Return the type of one source node.
    pub(in crate::check) fn node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        self.node_types.get(&node).copied()
    }

    /// Return the type of one source symbol.
    pub(in crate::check) fn symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.symbol_types.get(&symbol).copied()
    }

    /// Return the static value of one source symbol.
    pub(in crate::check) fn symbol_value(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.symbol_values.get(&symbol).copied()
    }

    /// Iterate node types declared in one module.
    pub(in crate::check) fn node_types_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalNodeIdAny, dir::GlobalTypeId)> + '_ {
        self.node_types
            .iter()
            .filter_map(move |(node, ty)| (node.module_id == module).then_some((*node, *ty)))
    }

    /// Iterate symbol types declared in one module.
    pub(in crate::check) fn symbol_types_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, dir::GlobalTypeId)> + '_ {
        self.symbol_types
            .iter()
            .filter_map(move |(symbol, ty)| (symbol.module_id == module).then_some((*symbol, *ty)))
    }

    /// Iterate symbol static values declared in one module.
    pub(in crate::check) fn symbol_values_in(
        &self,
        module: ModuleId,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, dir::GlobalTypeId)> + '_ {
        self.symbol_values
            .iter()
            .filter_map(move |(symbol, value)| {
                (symbol.module_id == module).then_some((*symbol, *value))
            })
    }
}
