use destack_source::AdaptImage;
use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GenericParameterKind, GlobalSymbolId, LocalTypeId, VarianceModifier};

use super::TypeTable;
use super::core::{GenericParameterSymbolKey, generic_parameter_symbol_key};

/// Generic parameter metadata ownership.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct GenericTable {
    /// Cached constraint types by generic parameter symbol.
    pub(crate) generic_parameter_constraint_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, LocalTypeId>,
    /// Declare-published constraint types by static parameter symbol.
    pub(crate) published_static_parameter_constraint_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, LocalTypeId>,
    /// Static parameter constraint resolution in progress.
    pub(crate) generic_parameter_constraint_in_progress: HashSet<GenericParameterSymbolKey>,
    /// Cached generic parameter kinds by symbol.
    pub(crate) generic_parameter_kind_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, GenericParameterKind>,
    /// Declare-published static parameter kinds by symbol.
    pub(crate) published_static_parameter_kind_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, GenericParameterKind>,
    /// Cached generic parameter variances by symbol.
    pub(crate) generic_parameter_variance_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, Option<VarianceModifier>>,
    /// Declare-published static parameter variances by symbol.
    pub(crate) published_static_parameter_variance_by_symbol_id:
        IndexMap<GenericParameterSymbolKey, Option<VarianceModifier>>,
    /// Cached generic parameter symbols by declaration symbol.
    pub(crate) generic_parameter_symbols_by_symbol_id:
        IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    /// Declare-published static parameter symbols by declaration symbol.
    pub(crate) published_static_parameter_symbols_by_symbol_id:
        IndexMap<GlobalSymbolId, Vec<GlobalSymbolId>>,
    /// Static parameter kind inference in progress.
    pub(crate) generic_parameter_kind_in_progress: HashSet<GenericParameterSymbolKey>,
}

impl GenericTable {
    /// Create an empty generic metadata table.
    pub fn new() -> Self {
        Self {
            generic_parameter_constraint_by_symbol_id: IndexMap::new(),
            published_static_parameter_constraint_by_symbol_id: IndexMap::new(),
            generic_parameter_constraint_in_progress: HashSet::new(),
            generic_parameter_kind_by_symbol_id: IndexMap::new(),
            published_static_parameter_kind_by_symbol_id: IndexMap::new(),
            generic_parameter_variance_by_symbol_id: IndexMap::new(),
            published_static_parameter_variance_by_symbol_id: IndexMap::new(),
            generic_parameter_symbols_by_symbol_id: IndexMap::new(),
            published_static_parameter_symbols_by_symbol_id: IndexMap::new(),
            generic_parameter_kind_in_progress: HashSet::new(),
        }
    }
}

impl Default for GenericTable {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    /// Cache the constraint type for a static parameter symbol.
    pub fn set_static_parameter_constraint_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        ty: LocalTypeId,
    ) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_constraint_by_symbol_id
            .insert(key, ty);
    }

    /// Store one artifact constraint type for a static parameter symbol.
    pub fn set_artifact_static_parameter_constraint_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        ty: LocalTypeId,
    ) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_constraint_by_symbol_id
            .insert(key, ty);
    }

    /// Get the cached constraint type for a static parameter symbol.
    pub fn get_static_parameter_constraint_type(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_constraint_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query one artifact constraint type for a static parameter symbol.
    pub fn query_artifact_static_parameter_constraint_type(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_constraint_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Mark a static parameter constraint as in progress.
    pub fn mark_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_constraint_in_progress
            .insert(key);
    }

    /// Clear the in progress marker for a static parameter constraint.
    pub fn clear_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_constraint_in_progress
            .remove(&key);
    }

    /// Check whether a static parameter constraint is in progress.
    pub fn is_static_parameter_constraint_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_constraint_in_progress
            .contains(&key)
    }

    /// Cache the inferred kind for a static parameter symbol.
    pub fn set_static_parameter_kind(
        &mut self,
        symbol_id: GlobalSymbolId,
        kind: GenericParameterKind,
    ) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_kind_by_symbol_id
            .insert(key, kind);
    }

    /// Store one artifact static parameter kind for a symbol.
    pub fn set_artifact_static_parameter_kind(
        &mut self,
        symbol_id: GlobalSymbolId,
        kind: GenericParameterKind,
    ) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_kind_by_symbol_id
            .insert(key, kind);
    }

    /// Get the cached static parameter kind for a symbol.
    pub fn get_static_parameter_kind(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<GenericParameterKind> {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_kind_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query one artifact static parameter kind for a symbol.
    pub fn query_artifact_static_parameter_kind(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<GenericParameterKind> {
        let key = generic_parameter_symbol_key(symbol_id);
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
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_variance_by_symbol_id
            .insert(key, variance);
    }

    /// Store one artifact variance for a static parameter symbol.
    pub fn set_artifact_static_parameter_variance(
        &mut self,
        symbol_id: GlobalSymbolId,
        variance: Option<VarianceModifier>,
    ) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .published_static_parameter_variance_by_symbol_id
            .insert(key, variance);
    }

    /// Get the cached variance for a static parameter symbol.
    pub fn get_static_parameter_variance(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Option<VarianceModifier>> {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_variance_by_symbol_id
            .get(&key)
            .copied()
    }

    /// Query one artifact variance for a static parameter symbol.
    pub fn query_artifact_static_parameter_variance(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<Option<VarianceModifier>> {
        let key = generic_parameter_symbol_key(symbol_id);
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
            .generic_parameter_symbols_by_symbol_id
            .insert(symbol_id, symbols);
    }

    /// Store artifact static parameter symbols for one declaration symbol.
    pub fn set_artifact_static_parameter_symbols(
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
            .generic_parameter_symbols_by_symbol_id
            .get(&symbol_id)
            .cloned()
    }

    /// Query artifact static parameter symbols for one declaration symbol.
    pub fn query_artifact_static_parameter_symbols(
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
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic.generic_parameter_kind_in_progress.insert(key);
    }

    /// Clear the in progress marker for a static parameter kind.
    pub fn clear_static_parameter_kind_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic.generic_parameter_kind_in_progress.remove(&key);
    }

    /// Check whether a static parameter kind is in progress.
    pub fn is_static_parameter_kind_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        let key = generic_parameter_symbol_key(symbol_id);
        self.generic
            .generic_parameter_kind_in_progress
            .contains(&key)
    }
}
