use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalTypeId, StaticParameterKind, VarianceModifier};

use super::TypeTable;
use super::core::{StaticParameterSymbolKey, static_parameter_symbol_key};

/// Static parameter metadata ownership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericTable {
    /// Cached constraint types by static parameter symbol.
    pub(crate) static_parameter_constraint_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, LocalTypeId>,
    /// Declare-published constraint types by static parameter symbol.
    pub(crate) published_static_parameter_constraint_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, LocalTypeId>,
    /// Static parameter constraint resolution in progress.
    pub(crate) static_parameter_constraint_in_progress: HashSet<StaticParameterSymbolKey>,
    /// Cached static parameter kinds by symbol.
    pub(crate) static_parameter_kind_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, StaticParameterKind>,
    /// Declare-published static parameter kinds by symbol.
    pub(crate) published_static_parameter_kind_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, StaticParameterKind>,
    /// Cached static parameter variances by symbol.
    pub(crate) static_parameter_variance_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, Option<VarianceModifier>>,
    /// Declare-published static parameter variances by symbol.
    pub(crate) published_static_parameter_variance_by_symbol_id:
        IndexMap<StaticParameterSymbolKey, Option<VarianceModifier>>,
    /// Cached static parameter symbols by declaration symbol.
    pub(crate) static_parameter_symbols_by_symbol_id: IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    /// Declare-published static parameter symbols by declaration symbol.
    pub(crate) published_static_parameter_symbols_by_symbol_id:
        IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    /// Static parameter kind inference in progress.
    pub(crate) static_parameter_kind_in_progress: HashSet<StaticParameterSymbolKey>,
}

impl GenericTable {
    /// Create an empty generic metadata table.
    pub fn new() -> Self {
        Self {
            static_parameter_constraint_by_symbol_id: IndexMap::new(),
            published_static_parameter_constraint_by_symbol_id: IndexMap::new(),
            static_parameter_constraint_in_progress: HashSet::new(),
            static_parameter_kind_by_symbol_id: IndexMap::new(),
            published_static_parameter_kind_by_symbol_id: IndexMap::new(),
            static_parameter_variance_by_symbol_id: IndexMap::new(),
            published_static_parameter_variance_by_symbol_id: IndexMap::new(),
            static_parameter_symbols_by_symbol_id: IndexMap::new(),
            published_static_parameter_symbols_by_symbol_id: IndexMap::new(),
            static_parameter_kind_in_progress: HashSet::new(),
        }
    }
}

impl TypeTable {
    /// Cache the constraint type for a static parameter symbol.
    pub fn set_static_parameter_constraint_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        ty: LocalTypeId,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_constraint_by_symbol_id
            .insert(key, ty);
    }

    /// Publish a declared constraint type for a static parameter symbol.
    pub fn publish_static_parameter_constraint_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        ty: LocalTypeId,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_constraint_by_symbol_id
            .insert(key, ty);
    }

    /// Get the cached constraint type for a static parameter symbol.
    pub fn get_static_parameter_constraint_type(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_constraint_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query the declare-published constraint type for a static parameter symbol.
    pub fn query_published_static_parameter_constraint_type(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_constraint_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Mark a static parameter constraint as in progress.
    pub fn mark_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_constraint_in_progress
            .insert(key);
    }

    /// Clear the in progress marker for a static parameter constraint.
    pub fn clear_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_constraint_in_progress
            .remove(&key);
    }

    /// Check whether a static parameter constraint is in progress.
    pub fn is_static_parameter_constraint_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_constraint_in_progress
            .contains(&key)
    }

    /// Cache the inferred kind for a static parameter symbol.
    pub fn set_static_parameter_kind(
        &mut self,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_kind_by_symbol_id
            .insert(key, kind);
    }

    /// Publish one declared static parameter kind for a symbol.
    pub fn publish_static_parameter_kind(
        &mut self,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_kind_by_symbol_id
            .insert(key, kind);
    }

    /// Get the cached static parameter kind for a symbol.
    pub fn get_static_parameter_kind(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<StaticParameterKind> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_kind_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query the declare-published static parameter kind for a symbol.
    pub fn query_published_static_parameter_kind(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<StaticParameterKind> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_kind_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Cache the variance for a static parameter symbol.
    pub fn set_static_parameter_variance(
        &mut self,
        symbol_id: GlobalSymbolId,
        variance: Option<VarianceModifier>,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_variance_by_symbol_id
            .insert(key, variance);
    }

    /// Publish one declared variance for a static parameter symbol.
    pub fn publish_static_parameter_variance(
        &mut self,
        symbol_id: GlobalSymbolId,
        variance: Option<VarianceModifier>,
    ) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_variance_by_symbol_id
            .insert(key, variance);
    }

    /// Get the cached variance for a static parameter symbol.
    pub fn get_static_parameter_variance(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Option<VarianceModifier>> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_variance_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query the declare-published variance for a static parameter symbol.
    pub fn query_published_static_parameter_variance(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Option<VarianceModifier>> {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_variance_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Cache static parameter symbols for a declaration symbol.
    pub fn set_static_parameter_symbols(
        &mut self,
        symbol_id: GlobalSymbolId,
        symbols: Vec<GlobalSymbolId>,
    ) {
        self.generic
            .static_parameter_symbols_by_symbol_id
            .insert(symbol_id, symbols);
    }

    /// Publish declared static parameter symbols for one declaration symbol.
    pub fn publish_static_parameter_symbols(
        &mut self,
        symbol_id: GlobalSymbolId,
        symbols: Vec<GlobalSymbolId>,
    ) {
        self.generic
            .published_static_parameter_symbols_by_symbol_id
            .insert(symbol_id, symbols);
    }

    /// Get cached static parameter symbols for a declaration symbol.
    pub fn get_static_parameter_symbols(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Vec<GlobalSymbolId>> {
        self.generic
            .static_parameter_symbols_by_symbol_id
            .get(&symbol_id)
            .cloned()
    }

    /// Query declare-published static parameter symbols for one declaration symbol.
    pub fn query_published_static_parameter_symbols(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Vec<GlobalSymbolId>> {
        self.generic
            .published_static_parameter_symbols_by_symbol_id
            .get(&symbol_id)
            .cloned()
    }

    /// Mark a static parameter kind as in progress.
    pub fn mark_static_parameter_kind_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic.static_parameter_kind_in_progress.insert(key);
    }

    /// Clear the in progress marker for a static parameter kind.
    pub fn clear_static_parameter_kind_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic.static_parameter_kind_in_progress.remove(&key);
    }

    /// Check whether a static parameter kind is in progress.
    pub fn is_static_parameter_kind_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        let key = static_parameter_symbol_key(symbol_id);
        self.generic
            .static_parameter_kind_in_progress
            .contains(&key)
    }
}
