use destack_source::AdaptImage;
use std::collections::HashSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, GlobalSymbolId, LocalTypeId, StaticArgument, Type, TypeLiteral};

use super::TypeTable;

/// Select a normalization cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub enum NormalizationMode {
    /// Normalize for assignability and constraint solving.
    Assign,
    /// Normalize for flow narrowing and guard checks.
    Flow,
}

/// Versions captured during normalization.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct NormalizationDependencyVersions {
    /// The type versions captured during normalization.
    pub type_versions: Vec<(LocalTypeId, u64)>,
    /// The symbol versions captured during normalization.
    pub symbol_versions: Vec<(GlobalSymbolId, u64)>,
}

/// Cache entry for alias normalization with static arguments.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct AliasNormalizationEntry {
    /// The alias symbol being normalized.
    pub symbol: GlobalSymbolId,
    /// The normalization mode for this entry.
    pub mode: NormalizationMode,
    /// The relation cache key for this entry.
    pub relation_key: u64,
    /// The static arguments applied to the alias.
    pub arguments: Vec<StaticArgument>,
    /// The dependency versions captured during normalization.
    pub dependency_versions: NormalizationDependencyVersions,
    /// The normalized type id.
    pub normalized_type: LocalTypeId,
}

/// Bucket key for alias normalization entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub struct AliasNormalizationKey {
    /// The alias symbol being normalized.
    pub symbol: GlobalSymbolId,
    /// The normalization mode for this entry.
    pub mode: NormalizationMode,
    /// The relation cache key for this entry.
    pub relation_key: u64,
}

/// Build a bucket key for alias normalization entries.
pub fn alias_normalization_key(
    symbol: GlobalSymbolId,
    mode: NormalizationMode,
    relation_key: u64,
) -> AliasNormalizationKey {
    AliasNormalizationKey {
        symbol,
        mode,
        relation_key,
    }
}

/// In-progress alias normalization key.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct AliasNormalizationInProgressKey {
    /// The alias normalization bucket key.
    pub key: AliasNormalizationKey,
    /// The static arguments applied to the alias.
    pub arguments: Vec<StaticArgument>,
}

/// Cache entry for normalized types keyed by relation.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct NormalizationCacheEntry {
    /// The normalized type id.
    pub normalized_type: LocalTypeId,
    /// The dependency versions captured during normalization.
    pub dependency_versions: NormalizationDependencyVersions,
}

/// Cache entry for shared type rewrites.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct RewriteCacheEntry {
    /// The rewritten type id.
    pub mapped_type: LocalTypeId,
    /// The dependency versions captured during rewriting.
    pub dependency_versions: NormalizationDependencyVersions,
}

/// Dependency tracking scope for normalization.
#[derive(Debug, Default, Clone, AdaptImage)]
pub struct NormalizationDependencyScope {
    /// The types touched during normalization.
    pub type_ids: HashSet<LocalTypeId>,
    /// The symbols touched during normalization.
    pub symbol_ids: HashSet<GlobalSymbolId>,
}

/// Normalization-owned relation cache tables.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct TypeNormalizationCache {
    /// Cached expression type ids by cache key.
    #[serde(skip)]
    pub(crate) expression_type_id_cache_by_key: IndexMap<u64, LocalTypeId>,
    /// Cached expression type values by cache key.
    #[serde(skip)]
    pub(crate) expression_type_value_cache_by_key: IndexMap<u64, Type>,
    /// Cached type reference results by cache key.
    #[serde(skip)]
    pub(crate) type_reference_cache_by_key: IndexMap<u64, Type>,
    /// Cached resolved static arguments by cache key.
    #[serde(skip)]
    pub(crate) generic_argument_resolution_cache_by_key: IndexMap<u64, Option<Vec<StaticArgument>>>,
    /// Cached normalization results for assignability.
    pub(crate) normalized_assignability_type_by_id: Vec<IndexMap<u64, NormalizationCacheEntry>>,
    /// Cached normalization results for flow.
    pub(crate) normalized_flow_type_by_id: Vec<IndexMap<u64, NormalizationCacheEntry>>,
    /// Cached alias normalization results indexed by alias key.
    pub(crate) normalized_alias_entries_by_key:
        IndexMap<AliasNormalizationKey, Vec<AliasNormalizationEntry>>,
    /// Alias normalization currently in progress.
    pub(crate) normalization_alias_in_progress: Vec<AliasNormalizationInProgressKey>,
    /// Assignability pairs currently in progress.
    pub(crate) assignability_in_progress: HashSet<(LocalTypeId, LocalTypeId)>,
    /// Active dependency tracking scopes for normalization.
    #[serde(skip)]
    pub(crate) normalization_dependency_stack: Vec<NormalizationDependencyScope>,
    /// Expression type evaluation in progress.
    pub(crate) expression_type_in_progress: HashSet<GlobalNodeIdAny>,
    /// Static argument resolution in progress.
    pub(crate) generic_argument_resolution_in_progress: Vec<(GlobalSymbolId, Vec<StaticArgument>)>,
}

/// Rewrite-owned relation cache tables.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct TypeRewriteCache {
    /// Cached rewrite results by cache key.
    pub(crate) rewrite_cache_by_key: IndexMap<u64, IndexMap<LocalTypeId, RewriteCacheEntry>>,
}

/// Interner-owned relation cache tables.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct TypeInternerCache {
    /// Cached literal type ids by literal hash bucket.
    #[serde(skip)]
    pub(crate) literal_type_id_by_hash: IndexMap<u64, Vec<(TypeLiteral, LocalTypeId)>>,
    /// Interned union type ids by element list.
    #[serde(skip)]
    pub(crate) interned_union_by_elements: IndexMap<Vec<LocalTypeId>, LocalTypeId>,
    /// Interned intersection type ids by element list.
    #[serde(skip)]
    pub(crate) interned_intersection_by_elements: IndexMap<Vec<LocalTypeId>, LocalTypeId>,
}

/// Type-relation cache ownership.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct TypeRelationCache {
    /// Normalization-owned relation cache tables.
    pub(crate) normalization: TypeNormalizationCache,
    /// Rewrite-owned relation cache tables.
    pub(crate) rewrite_cache: TypeRewriteCache,
    /// Interner-owned relation cache tables.
    pub(crate) interner: TypeInternerCache,
}

impl TypeNormalizationCache {
    /// Create an empty normalization cache table.
    pub fn new() -> Self {
        Self {
            expression_type_id_cache_by_key: IndexMap::new(),
            expression_type_value_cache_by_key: IndexMap::new(),
            type_reference_cache_by_key: IndexMap::new(),
            generic_argument_resolution_cache_by_key: IndexMap::new(),
            normalized_assignability_type_by_id: Vec::new(),
            normalized_flow_type_by_id: Vec::new(),
            normalized_alias_entries_by_key: IndexMap::new(),
            normalization_alias_in_progress: Vec::new(),
            assignability_in_progress: HashSet::new(),
            normalization_dependency_stack: Vec::new(),
            expression_type_in_progress: HashSet::new(),
            generic_argument_resolution_in_progress: Vec::new(),
        }
    }
}

impl Default for TypeNormalizationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRewriteCache {
    /// Create an empty rewrite cache table.
    pub fn new() -> Self {
        Self {
            rewrite_cache_by_key: IndexMap::new(),
        }
    }
}

impl Default for TypeRewriteCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInternerCache {
    /// Create an empty interner cache table.
    pub fn new() -> Self {
        Self {
            literal_type_id_by_hash: IndexMap::new(),
            interned_union_by_elements: IndexMap::new(),
            interned_intersection_by_elements: IndexMap::new(),
        }
    }
}

impl Default for TypeInternerCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRelationCache {
    /// Create an empty type-relation cache table.
    pub fn new() -> Self {
        Self {
            normalization: TypeNormalizationCache::new(),
            rewrite_cache: TypeRewriteCache::new(),
            interner: TypeInternerCache::new(),
        }
    }
}

impl Default for TypeRelationCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeTable {
    /// Initialize relation cache slots for one newly allocated type id.
    pub(super) fn initialize_relation_cache_for_new_type(&mut self) {
        self.relation
            .normalization
            .normalized_assignability_type_by_id
            .push(IndexMap::new());
        self.relation
            .normalization
            .normalized_flow_type_by_id
            .push(IndexMap::new());
    }

    /// Return a cached expression type id for a cache key.
    pub fn get_expression_type_id_cache(&self, key: u64) -> Option<LocalTypeId> {
        self.relation
            .normalization
            .expression_type_id_cache_by_key
            .get(&key)
            .copied()
    }

    /// Store a cached expression type id for a cache key.
    pub fn set_expression_type_id_cache(&mut self, key: u64, type_id: LocalTypeId) {
        self.relation
            .normalization
            .expression_type_id_cache_by_key
            .insert(key, type_id);
    }

    /// Return a cached expression type value for a cache key.
    pub fn get_expression_type_value_cache(&self, key: u64) -> Option<&Type> {
        self.relation
            .normalization
            .expression_type_value_cache_by_key
            .get(&key)
    }

    /// Store a cached expression type value for a cache key.
    pub fn set_expression_type_value_cache(&mut self, key: u64, ty: Type) {
        self.relation
            .normalization
            .expression_type_value_cache_by_key
            .insert(key, ty);
    }

    /// Return a cached type reference result for a cache key.
    pub fn get_type_reference_cache(&self, key: u64) -> Option<&Type> {
        self.relation
            .normalization
            .type_reference_cache_by_key
            .get(&key)
    }

    /// Store a cached type reference result for a cache key.
    pub fn set_type_reference_cache(&mut self, key: u64, ty: Type) {
        self.relation
            .normalization
            .type_reference_cache_by_key
            .insert(key, ty);
    }

    /// Return cached resolved static arguments for a cache key.
    pub fn get_static_argument_resolution_cache(
        &self,
        key: u64,
    ) -> Option<Option<Vec<StaticArgument>>> {
        self.relation
            .normalization
            .generic_argument_resolution_cache_by_key
            .get(&key)
            .cloned()
    }

    /// Store cached resolved static arguments for a cache key.
    pub fn set_static_argument_resolution_cache(
        &mut self,
        key: u64,
        resolved: Option<Vec<StaticArgument>>,
    ) {
        self.relation
            .normalization
            .generic_argument_resolution_cache_by_key
            .insert(key, resolved);
    }

    /// Get a cached normalized type for the chosen mode.
    pub fn get_normalized_type(
        &self,
        mode: NormalizationMode,
        relation_key: u64,
        type_id: LocalTypeId,
    ) -> Option<NormalizationCacheEntry> {
        let cache_index = type_id.0 as usize;
        let cache = match mode {
            NormalizationMode::Assign => self
                .relation
                .normalization
                .normalized_assignability_type_by_id
                .get(cache_index)?,
            NormalizationMode::Flow => self
                .relation
                .normalization
                .normalized_flow_type_by_id
                .get(cache_index)?,
        };

        let entry = cache.get(&relation_key).cloned()?;
        self.dependency_versions_are_valid(&entry.dependency_versions)
            .then_some(entry)
    }

    /// Return the normalized type id when a valid cache entry exists.
    pub fn get_normalized_type_id(
        &self,
        mode: NormalizationMode,
        relation_key: u64,
        type_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        self.get_normalized_type(mode, relation_key, type_id)
            .map(|entry| entry.normalized_type)
    }

    /// Return a normalized type id, or the original type id when uncached.
    pub fn get_normalized_type_id_or(
        &self,
        mode: NormalizationMode,
        relation_key: u64,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        self.get_normalized_type_id(mode, relation_key, type_id)
            .unwrap_or(type_id)
    }

    /// Cache a normalized type for the chosen mode.
    pub fn set_normalized_type(
        &mut self,
        mode: NormalizationMode,
        relation_key: u64,
        type_id: LocalTypeId,
        normalized_type: LocalTypeId,
        dependency_versions: NormalizationDependencyVersions,
    ) {
        let cache = self.required_normalization_cache_mut(mode, type_id);
        cache.insert(
            relation_key,
            NormalizationCacheEntry {
                normalized_type,
                dependency_versions,
            },
        );
    }

    /// Return the required normalization cache slot for one type id.
    fn required_normalization_cache_mut(
        &mut self,
        mode: NormalizationMode,
        type_id: LocalTypeId,
    ) -> &mut IndexMap<u64, NormalizationCacheEntry> {
        let cache_index = type_id.0 as usize;

        // keep normalization cache slots aligned with type ids
        self.ensure_normalization_cache_slot(cache_index);

        match mode {
            NormalizationMode::Assign => {
                &mut self
                    .relation
                    .normalization
                    .normalized_assignability_type_by_id[cache_index]
            }
            NormalizationMode::Flow => {
                &mut self.relation.normalization.normalized_flow_type_by_id[cache_index]
            }
        }
    }

    /// Ensure normalization caches contain one slot for the given type index.
    fn ensure_normalization_cache_slot(&mut self, cache_index: usize) {
        let assign_cache = &mut self
            .relation
            .normalization
            .normalized_assignability_type_by_id;
        if assign_cache.len() <= cache_index {
            assign_cache.resize_with(cache_index + 1, IndexMap::new);
        }

        let flow_cache = &mut self.relation.normalization.normalized_flow_type_by_id;
        if flow_cache.len() <= cache_index {
            flow_cache.resize_with(cache_index + 1, IndexMap::new);
        }
    }

    /// Start a dependency tracking scope for normalization.
    pub fn push_normalization_dependency_scope(&mut self) {
        self.relation
            .normalization
            .normalization_dependency_stack
            .push(NormalizationDependencyScope::default());
    }

    /// Finish a dependency tracking scope for normalization.
    pub fn pop_normalization_dependency_scope(&mut self) -> NormalizationDependencyScope {
        self.relation
            .normalization
            .normalization_dependency_stack
            .pop()
            .unwrap_or_default()
    }

    /// Record a dependency on a type for all active normalization scopes.
    pub fn record_normalization_dependency(&mut self, type_id: LocalTypeId) {
        for scope in &mut self.relation.normalization.normalization_dependency_stack {
            scope.type_ids.insert(type_id);
        }
    }

    /// Capture dependency versions for normalization caches.
    pub fn collect_dependency_versions(
        &self,
        dependencies: NormalizationDependencyScope,
    ) -> NormalizationDependencyVersions {
        let mut type_versions: Vec<(LocalTypeId, u64)> = dependencies
            .type_ids
            .into_iter()
            .map(|type_id| (type_id, self.type_version(type_id)))
            .collect();
        type_versions.sort_by_key(|(type_id, _)| type_id.0);

        let mut symbol_versions: Vec<(GlobalSymbolId, u64)> = dependencies
            .symbol_ids
            .into_iter()
            .map(|symbol_id| (symbol_id, self.symbol_version(symbol_id)))
            .collect();
        symbol_versions.sort_by_key(|(symbol_id, _)| *symbol_id);

        NormalizationDependencyVersions {
            type_versions,
            symbol_versions,
        }
    }

    /// Record a dependency on a symbol mapping for all active normalization scopes.
    pub fn record_normalization_symbol_dependency(&mut self, symbol_id: GlobalSymbolId) {
        for scope in &mut self.relation.normalization.normalization_dependency_stack {
            scope.symbol_ids.insert(symbol_id);
        }
    }

    /// Return a cached rewrite entry when dependencies still match.
    pub fn get_rewrite_cached_type(
        &mut self,
        cache_key: u64,
        type_id: LocalTypeId,
    ) -> Option<RewriteCacheEntry> {
        let entry = self
            .relation
            .rewrite_cache
            .rewrite_cache_by_key
            .get(&cache_key)?
            .get(&type_id)?
            .clone();
        if self.dependency_versions_are_valid(&entry.dependency_versions) {
            return Some(entry);
        }

        if let Some(cache) = self
            .relation
            .rewrite_cache
            .rewrite_cache_by_key
            .get_mut(&cache_key)
        {
            cache.swap_remove(&type_id);
        }
        None
    }

    /// Cache a rewritten type for reuse.
    pub fn set_rewrite_cached_type(
        &mut self,
        cache_key: u64,
        type_id: LocalTypeId,
        mapped_type: LocalTypeId,
        dependency_versions: NormalizationDependencyVersions,
    ) {
        self.relation
            .rewrite_cache
            .rewrite_cache_by_key
            .entry(cache_key)
            .or_default()
            .insert(
                type_id,
                RewriteCacheEntry {
                    mapped_type,
                    dependency_versions,
                },
            );
    }

    /// Intern a union type by its element list.
    pub fn intern_union_type(
        &mut self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
    ) -> LocalTypeId {
        if let Some(type_id) = self
            .relation
            .interner
            .interned_union_by_elements
            .get(&elements)
            .copied()
        {
            return type_id;
        }

        let type_id = self.insert_type_from_any(
            Type::Union {
                elements: elements.clone(),
            },
            self.get_type_source(source_type_id),
        );
        self.relation
            .interner
            .interned_union_by_elements
            .insert(elements, type_id);
        type_id
    }

    /// Intern an intersection type by its element list.
    pub fn intern_intersection_type(
        &mut self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
    ) -> LocalTypeId {
        if let Some(type_id) = self
            .relation
            .interner
            .interned_intersection_by_elements
            .get(&elements)
            .copied()
        {
            return type_id;
        }

        let type_id = self.insert_type_from_any(
            Type::Intersection {
                elements: elements.clone(),
            },
            self.get_type_source(source_type_id),
        );
        self.relation
            .interner
            .interned_intersection_by_elements
            .insert(elements, type_id);
        type_id
    }

    /// Validate dependency versions against current table state.
    fn dependency_versions_are_valid(
        &self,
        dependencies: &NormalizationDependencyVersions,
    ) -> bool {
        dependencies
            .type_versions
            .iter()
            .all(|(type_id, version)| self.type_version(*type_id) == *version)
            && dependencies
                .symbol_versions
                .iter()
                .all(|(symbol_id, version)| self.symbol_version(*symbol_id) == *version)
    }

    /// Mark an alias normalization as in progress.
    pub fn mark_normalization_alias_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: Vec<StaticArgument>,
    ) {
        self.relation
            .normalization
            .normalization_alias_in_progress
            .push(AliasNormalizationInProgressKey {
                key: alias_normalization_key(symbol_id, mode, relation_key),
                arguments,
            });
    }

    /// Clear the alias normalization in progress marker.
    pub fn clear_normalization_alias_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: &[StaticArgument],
    ) {
        let key = alias_normalization_key(symbol_id, mode, relation_key);
        if let Some(index) = self
            .relation
            .normalization
            .normalization_alias_in_progress
            .iter()
            .position(|entry| entry.key == key && entry.arguments == arguments)
        {
            self.relation
                .normalization
                .normalization_alias_in_progress
                .remove(index);
        }
    }

    /// Check whether an alias normalization is in progress.
    pub fn is_normalization_alias_in_progress(
        &self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: &[StaticArgument],
    ) -> bool {
        let key = alias_normalization_key(symbol_id, mode, relation_key);
        self.relation
            .normalization
            .normalization_alias_in_progress
            .iter()
            .any(|entry| entry.key == key && entry.arguments == arguments)
    }

    /// Return the current alias normalization stack depth.
    pub fn normalization_alias_in_progress_depth(&self) -> usize {
        self.relation
            .normalization
            .normalization_alias_in_progress
            .len()
    }

    /// Get a cached normalized alias reference.
    pub fn get_normalized_alias_reference(
        &mut self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: &[StaticArgument],
    ) -> Option<AliasNormalizationEntry> {
        let key = alias_normalization_key(symbol_id, mode, relation_key);
        let entry = self
            .relation
            .normalization
            .normalized_alias_entries_by_key
            .get(&key)?
            .iter()
            .find(|entry| entry.arguments == arguments)
            .cloned()?;

        if self.dependency_versions_are_valid(&entry.dependency_versions) {
            return Some(entry);
        }

        let mut remove_bucket = false;
        if let Some(entries) = self
            .relation
            .normalization
            .normalized_alias_entries_by_key
            .get_mut(&key)
            && let Some(index) = entries
                .iter()
                .position(|entry| entry.arguments == arguments)
        {
            entries.swap_remove(index);
            remove_bucket = entries.is_empty();
        }

        if remove_bucket {
            self.relation
                .normalization
                .normalized_alias_entries_by_key
                .swap_remove(&key);
        }

        None
    }

    /// Cache a normalized alias reference.
    pub fn set_normalized_alias_reference(
        &mut self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: Vec<StaticArgument>,
        normalized_type: LocalTypeId,
        dependency_versions: NormalizationDependencyVersions,
    ) {
        let key = alias_normalization_key(symbol_id, mode, relation_key);
        let entries = self
            .relation
            .normalization
            .normalized_alias_entries_by_key
            .entry(key)
            .or_default();
        if entries.iter().any(|entry| entry.arguments == arguments) {
            return;
        }

        entries.push(AliasNormalizationEntry {
            symbol: symbol_id,
            mode,
            relation_key,
            arguments,
            dependency_versions,
            normalized_type,
        });
    }

    /// Mark assignability for a target and source pair.
    pub fn mark_assignability_in_progress(
        &mut self,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) -> bool {
        self.relation
            .normalization
            .assignability_in_progress
            .insert((target_id, source_id))
    }

    /// Clear assignability in progress for a target and source pair.
    pub fn clear_assignability_in_progress(
        &mut self,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) {
        self.relation
            .normalization
            .assignability_in_progress
            .remove(&(target_id, source_id));
    }

    /// Mark static argument resolution as in progress.
    pub fn mark_static_argument_resolution_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
    ) {
        self.relation
            .normalization
            .generic_argument_resolution_in_progress
            .push((symbol_id, arguments));
    }

    /// Clear the in progress marker for static argument resolution.
    pub fn clear_static_argument_resolution_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) {
        if let Some(index) = self
            .relation
            .normalization
            .generic_argument_resolution_in_progress
            .iter()
            .position(|(symbol, stored)| *symbol == symbol_id && stored == arguments)
        {
            self.relation
                .normalization
                .generic_argument_resolution_in_progress
                .swap_remove(index);
        }
    }

    /// Check whether static argument resolution is in progress.
    pub fn is_static_argument_resolution_in_progress(
        &self,
        symbol_id: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) -> bool {
        self.relation
            .normalization
            .generic_argument_resolution_in_progress
            .iter()
            .any(|(symbol, stored)| *symbol == symbol_id && stored == arguments)
    }

    /// Mark expression type evaluation as in progress.
    pub fn mark_expression_type_in_progress(&mut self, node_id: GlobalNodeIdAny) {
        self.relation
            .normalization
            .expression_type_in_progress
            .insert(node_id);
    }

    /// Clear the in progress marker for expression type evaluation.
    pub fn clear_expression_type_in_progress(&mut self, node_id: GlobalNodeIdAny) {
        self.relation
            .normalization
            .expression_type_in_progress
            .remove(&node_id);
    }

    /// Check whether expression type evaluation is in progress.
    pub fn is_expression_type_in_progress(&self, node_id: GlobalNodeIdAny) -> bool {
        self.relation
            .normalization
            .expression_type_in_progress
            .contains(&node_id)
    }
}
