use std::collections::HashSet;

use indexmap::IndexMap;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Addressability, Arena, EnumBackingType, EnumFieldValue, Extension, GlobalNodeIdAny,
    GlobalSymbolId, Instance, Lineage, LocalExtensionId, LocalInstanceId, LocalLineageId,
    LocalNodeId, LocalNodeIdAny, LocalResolutionId, LocalTypeId, Node, Resolution, StaticArgument,
    StaticParameterKind, SymbolTable, Type,
};

/// Select a normalization cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NormalizationMode {
    /// Normalize for assignability and constraint solving.
    Assign,
    /// Normalize for flow narrowing and guard checks.
    Flow,
}

/// Versions captured during normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationDependencyVersions {
    /// The type versions captured during normalization.
    pub type_versions: Vec<(LocalTypeId, u64)>,
    /// The symbol versions captured during normalization.
    pub symbol_versions: Vec<(GlobalSymbolId, u64)>,
}

/// Cache entry for alias normalization with static arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Cache entry for normalized types keyed by relation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationCacheEntry {
    /// The normalized type id.
    pub normalized_type: LocalTypeId,
    /// The dependency versions captured during normalization.
    pub dependency_versions: NormalizationDependencyVersions,
}

/// Cache entry for shared type rewrites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteCacheEntry {
    /// The rewritten type id.
    pub mapped_type: LocalTypeId,
    /// The dependency versions captured during rewriting.
    pub dependency_versions: NormalizationDependencyVersions,
}

/// Dependency tracking scope for normalization.
#[derive(Debug, Default, Clone)]
pub struct NormalizationDependencyScope {
    /// The types touched during normalization.
    pub type_ids: HashSet<LocalTypeId>,
    /// The symbols touched during normalization.
    pub symbol_ids: HashSet<GlobalSymbolId>,
}

/// TypeTable stores all type-related analysis results for a module. NOT THREAD-SAFE.
/// NOTE #Cleanup #Architecture: revisit TypeTable.*_in_progress markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeTable {
    /// The module id of the type table.
    pub module_id: ModuleId,

    // types
    /// The next type id to allocate.
    pub(crate) next_type_id: u32,
    /// The types.
    pub(crate) types: Arena<Type>,
    /// The version number for each type id.
    pub(crate) type_version_by_id: Vec<u64>,
    /// The version number for symbol to type mappings.
    pub(crate) symbol_version_by_id: IndexMap<GlobalSymbolId, u64>,
    /// The source ids of all types. Index is the type id.
    pub(crate) source_id_by_type_id: Vec<LocalNodeIdAny>,
    /// Whether a type originates from an imported module.
    pub(crate) imported_type_by_id: Vec<bool>,
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
    pub(crate) static_argument_resolution_cache_by_key: IndexMap<u64, Option<Vec<StaticArgument>>>,
    /// Cached normalization results for assignability.
    pub(crate) normalized_assignability_type_by_id: Vec<IndexMap<u64, NormalizationCacheEntry>>,
    /// Cached normalization results for flow.
    pub(crate) normalized_flow_type_by_id: Vec<IndexMap<u64, NormalizationCacheEntry>>,
    /// Cached alias normalization results.
    pub(crate) normalized_alias_by_symbol: Vec<AliasNormalizationEntry>,
    /// Cached rewrite results by cache key.
    pub(crate) rewrite_cache_by_key: IndexMap<u64, IndexMap<LocalTypeId, RewriteCacheEntry>>,
    /// Alias normalization currently in progress.
    pub(crate) normalization_alias_in_progress: HashSet<GlobalSymbolId>,
    /// Assignability pairs currently in progress.
    pub(crate) assignability_in_progress: HashSet<(LocalTypeId, LocalTypeId)>,
    /// Active dependency tracking scopes for normalization.
    #[serde(skip)]
    pub(crate) normalization_dependency_stack: Vec<NormalizationDependencyScope>,
    /// Interned union type ids by element list.
    #[serde(skip)]
    pub(crate) interned_union_by_elements: IndexMap<Vec<LocalTypeId>, LocalTypeId>,
    /// Interned intersection type ids by element list.
    #[serde(skip)]
    pub(crate) interned_intersection_by_elements: IndexMap<Vec<LocalTypeId>, LocalTypeId>,

    // static parameter constraints
    /// Cached constraint types by static parameter symbol.
    pub(crate) static_parameter_constraint_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// Static parameter constraint resolution in progress.
    pub(crate) static_parameter_constraint_in_progress: HashSet<GlobalSymbolId>,
    /// Cached static parameter kinds by symbol.
    pub(crate) static_parameter_kind_by_symbol_id: IndexMap<GlobalSymbolId, StaticParameterKind>,
    /// Static parameter kind inference in progress.
    pub(crate) static_parameter_kind_in_progress: HashSet<GlobalSymbolId>,
    /// Expression type evaluation in progress.
    pub(crate) expression_type_in_progress: HashSet<GlobalNodeIdAny>,
    /// Static argument resolution in progress.
    pub(crate) static_argument_resolution_in_progress: Vec<(GlobalSymbolId, Vec<StaticArgument>)>,

    // node types
    /// The declared type by node id.
    pub(crate) declared_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The inferred type by node id.
    pub(crate) inferred_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The signature type by node id (separate from declared types).
    pub(crate) signature_type_by_node_id: IndexMap<GlobalNodeIdAny, LocalTypeId>,
    /// The addressability by node id.
    pub(crate) addressability_by_node_id: IndexMap<GlobalNodeIdAny, Addressability>,

    // symbol types
    /// The instance type by symbol id (for type declarations: the shape of instances).
    pub(crate) instance_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// The value type by symbol id (the type when used as a value).
    pub(crate) value_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    /// The target type id for alias symbols (the declared alias value type).
    pub(crate) alias_target_type_by_symbol_id: IndexMap<GlobalSymbolId, LocalTypeId>,
    // #Architecture: move enum backing type into Type::Enum (?)
    /// The backing type of enum symbols.
    pub(crate) enum_backing_type_by_symbol_id: IndexMap<GlobalSymbolId, EnumBackingType>,
    /// The resolved enum field values by enum field symbol.
    pub(crate) enum_field_value_by_symbol_id: IndexMap<GlobalSymbolId, EnumFieldValue>,

    // instances (statically parameterised types)
    /// The next instance id to allocate.
    pub(crate) next_instance_id: u32,
    /// The instances.
    pub(crate) instances: Arena<Instance>,
    /// The instance used by node ids.
    pub(crate) instance_by_node_id: IndexMap<GlobalNodeIdAny, LocalInstanceId>,

    // resolutions (types of members like functions/methods)
    /// The next resolution id to allocate.
    pub(crate) next_resolution_id: u32,
    /// The resolutions.
    pub(crate) resolutions: Arena<Resolution>,
    /// The resolution used by node ids.
    pub(crate) resolution_by_node_id: IndexMap<GlobalNodeIdAny, LocalResolutionId>,

    // lineages (resolved heritage for nominal types)
    /// The next lineage id to allocate.
    pub(crate) next_lineage_id: u32,
    /// The lineages.
    pub(crate) lineages: Arena<Lineage>,
    /// The lineage by symbol id (for type declarations: their resolved heritage).
    pub(crate) lineage_by_symbol_id: IndexMap<GlobalSymbolId, LocalLineageId>,

    // extensions (methods and implements added to existing types)
    /// The next extension id to allocate.
    pub(crate) next_extension_id: u32,
    /// The extensions.
    pub(crate) extensions: Arena<Extension>,
    /// Extensions by their declaration symbol (for lookup by extension symbol).
    pub(crate) extension_by_symbol: IndexMap<GlobalSymbolId, LocalExtensionId>,
    /// Extensions indexed by target symbol (for member lookup on types).
    pub(crate) extensions_by_target: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
}

impl TypeTable {
    /// Create a new TypeTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            // types
            next_type_id: 0,
            types: Arena::new(),
            type_version_by_id: Vec::new(),
            symbol_version_by_id: IndexMap::new(),
            source_id_by_type_id: Vec::new(),
            imported_type_by_id: Vec::new(),
            expression_type_id_cache_by_key: IndexMap::new(),
            expression_type_value_cache_by_key: IndexMap::new(),
            type_reference_cache_by_key: IndexMap::new(),
            static_argument_resolution_cache_by_key: IndexMap::new(),
            normalized_assignability_type_by_id: Vec::new(),
            normalized_flow_type_by_id: Vec::new(),
            normalized_alias_by_symbol: Vec::new(),
            rewrite_cache_by_key: IndexMap::new(),
            normalization_alias_in_progress: HashSet::new(),
            assignability_in_progress: HashSet::new(),
            normalization_dependency_stack: Vec::new(),
            interned_union_by_elements: IndexMap::new(),
            interned_intersection_by_elements: IndexMap::new(),

            // static parameter constraints
            static_parameter_constraint_by_symbol_id: IndexMap::new(),
            static_parameter_constraint_in_progress: HashSet::new(),
            static_parameter_kind_by_symbol_id: IndexMap::new(),
            static_parameter_kind_in_progress: HashSet::new(),
            static_argument_resolution_in_progress: Vec::new(),
            expression_type_in_progress: HashSet::new(),

            // node types
            declared_type_by_node_id: IndexMap::new(),
            inferred_type_by_node_id: IndexMap::new(),
            signature_type_by_node_id: IndexMap::new(),
            addressability_by_node_id: IndexMap::new(),
            // symbol types
            instance_type_by_symbol_id: IndexMap::new(),
            value_type_by_symbol_id: IndexMap::new(),
            alias_target_type_by_symbol_id: IndexMap::new(),
            enum_backing_type_by_symbol_id: IndexMap::new(),
            enum_field_value_by_symbol_id: IndexMap::new(),
            // instances
            next_instance_id: 0,
            instances: Arena::new(),
            instance_by_node_id: IndexMap::new(),
            // resolutions
            next_resolution_id: 0,
            resolutions: Arena::new(),
            resolution_by_node_id: IndexMap::new(),
            // lineages
            next_lineage_id: 0,
            lineages: Arena::new(),
            lineage_by_symbol_id: IndexMap::new(),
            // extensions
            next_extension_id: 0,
            extensions: Arena::new(),
            extension_by_symbol: IndexMap::new(),
            extensions_by_target: IndexMap::new(),
        }
    }

    /// Insert a type derived from some source node.
    pub fn insert_type_from<T: Node>(&mut self, ty: Type, node_id: LocalNodeId<T>) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.type_version_by_id.push(1);
        self.source_id_by_type_id.push(node_id.into_any());
        self.imported_type_by_id.push(false);
        self.normalized_assignability_type_by_id
            .push(IndexMap::new());
        self.normalized_flow_type_by_id.push(IndexMap::new());
        type_id
    }

    /// Insert a type derived from some source node (any node type).
    pub fn insert_type_from_any(&mut self, ty: Type, node_id: LocalNodeIdAny) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.type_version_by_id.push(1);
        self.source_id_by_type_id.push(node_id);
        self.imported_type_by_id.push(false);
        self.normalized_assignability_type_by_id
            .push(IndexMap::new());
        self.normalized_flow_type_by_id.push(IndexMap::new());
        type_id
    }

    /// Insert a type that originates from an imported module.
    pub fn insert_imported_type_from_any(
        &mut self,
        ty: Type,
        node_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        let type_id = LocalTypeId::new(self.next_type_id);
        self.next_type_id += 1;
        self.types.allocate(ty);
        self.type_version_by_id.push(1);
        self.source_id_by_type_id.push(node_id);
        self.imported_type_by_id.push(true);
        self.normalized_assignability_type_by_id
            .push(IndexMap::new());
        self.normalized_flow_type_by_id.push(IndexMap::new());
        type_id
    }

    /// Insert a type derived from another type id.
    pub fn insert_type_from_type(&mut self, ty: Type, source_type_id: LocalTypeId) -> LocalTypeId {
        let source_id = self.get_type_source(source_type_id);
        self.insert_type_from_any(ty, source_id)
    }

    /// Get a type by its id.
    pub fn get_type(&self, type_id: LocalTypeId) -> &Type {
        self.types.get(type_id.0)
    }

    /// Return a cached expression type id for a cache key.
    pub fn get_expression_type_id_cache(&self, key: u64) -> Option<LocalTypeId> {
        self.expression_type_id_cache_by_key.get(&key).copied()
    }

    /// Store a cached expression type id for a cache key.
    pub fn set_expression_type_id_cache(&mut self, key: u64, type_id: LocalTypeId) {
        self.expression_type_id_cache_by_key.insert(key, type_id);
    }

    /// Return a cached expression type value for a cache key.
    pub fn get_expression_type_value_cache(&self, key: u64) -> Option<&Type> {
        self.expression_type_value_cache_by_key.get(&key)
    }

    /// Store a cached expression type value for a cache key.
    pub fn set_expression_type_value_cache(&mut self, key: u64, ty: Type) {
        self.expression_type_value_cache_by_key.insert(key, ty);
    }

    /// Return a cached type reference result for a cache key.
    pub fn get_type_reference_cache(&self, key: u64) -> Option<&Type> {
        self.type_reference_cache_by_key.get(&key)
    }

    /// Store a cached type reference result for a cache key.
    pub fn set_type_reference_cache(&mut self, key: u64, ty: Type) {
        self.type_reference_cache_by_key.insert(key, ty);
    }

    /// Return cached resolved static arguments for a cache key.
    pub fn get_static_argument_resolution_cache(
        &self,
        key: u64,
    ) -> Option<Option<Vec<StaticArgument>>> {
        self.static_argument_resolution_cache_by_key
            .get(&key)
            .cloned()
    }

    /// Store cached resolved static arguments for a cache key.
    pub fn set_static_argument_resolution_cache(
        &mut self,
        key: u64,
        resolved: Option<Vec<StaticArgument>>,
    ) {
        self.static_argument_resolution_cache_by_key
            .insert(key, resolved);
    }

    /// Iterate over all type ids.
    pub fn iter_type_ids(&self) -> impl Iterator<Item = LocalTypeId> + '_ {
        (0..self.types.len()).map(|id| LocalTypeId::new(id as u32))
    }

    /// Get a mutable type by its id without bumping its version.
    pub fn get_type_mut(&mut self, type_id: LocalTypeId) -> &mut Type {
        self.types.get_mut(type_id.0)
    }

    /// Update a type and bump its version.
    pub fn update_type(&mut self, type_id: LocalTypeId, ty: Type) {
        *self.types.get_mut(type_id.0) = ty;
        self.bump_type_version(type_id);
    }

    /// Return the current version for a type id.
    pub fn type_version(&self, type_id: LocalTypeId) -> u64 {
        self.type_version_by_id[type_id.0 as usize]
    }

    /// Bump the version for a type id.
    pub fn bump_type_version(&mut self, type_id: LocalTypeId) {
        if let Some(version) = self.type_version_by_id.get_mut(type_id.0 as usize) {
            *version = version.wrapping_add(1);
        }
    }

    /// Return the current version for a symbol mapping.
    pub fn symbol_version(&self, symbol_id: GlobalSymbolId) -> u64 {
        self.symbol_version_by_id
            .get(&symbol_id)
            .copied()
            .unwrap_or(0)
    }

    /// Bump the version for a symbol mapping.
    pub fn bump_symbol_version(&mut self, symbol_id: GlobalSymbolId) {
        let version = self.symbol_version_by_id.entry(symbol_id).or_insert(0);
        *version = version.wrapping_add(1);
    }

    /// Get the source id for a type.
    pub fn get_type_source(&self, type_id: LocalTypeId) -> LocalNodeIdAny {
        self.source_id_by_type_id[type_id.0 as usize]
    }

    /// Return true when a type originated from an imported module.
    pub fn is_imported_type(&self, type_id: LocalTypeId) -> bool {
        self.imported_type_by_id[type_id.0 as usize]
    }

    /// Get a cached normalized type for the chosen mode.
    pub fn normalized_type(
        &self,
        mode: NormalizationMode,
        relation_key: u64,
        type_id: LocalTypeId,
    ) -> Option<NormalizationCacheEntry> {
        let cache_index = type_id.0 as usize;
        let cache = match mode {
            NormalizationMode::Assign => {
                self.normalized_assignability_type_by_id.get(cache_index)?
            }
            NormalizationMode::Flow => self.normalized_flow_type_by_id.get(cache_index)?,
        };

        let entry = cache.get(&relation_key).cloned()?;
        self.dependency_versions_are_valid(&entry.dependency_versions)
            .then_some(entry)
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
        let cache_index = type_id.0 as usize;
        match mode {
            NormalizationMode::Assign => {
                if self.normalized_assignability_type_by_id.len() <= cache_index {
                    self.normalized_assignability_type_by_id
                        .resize_with(cache_index + 1, IndexMap::new);
                }
                self.normalized_assignability_type_by_id[cache_index].insert(
                    relation_key,
                    NormalizationCacheEntry {
                        normalized_type,
                        dependency_versions,
                    },
                );
            }
            NormalizationMode::Flow => {
                if self.normalized_flow_type_by_id.len() <= cache_index {
                    self.normalized_flow_type_by_id
                        .resize_with(cache_index + 1, IndexMap::new);
                }
                self.normalized_flow_type_by_id[cache_index].insert(
                    relation_key,
                    NormalizationCacheEntry {
                        normalized_type,
                        dependency_versions,
                    },
                );
            }
        }
    }

    /// Start a dependency tracking scope for normalization.
    pub fn push_normalization_dependency_scope(&mut self) {
        self.normalization_dependency_stack
            .push(NormalizationDependencyScope::default());
    }

    /// Finish a dependency tracking scope for normalization.
    pub fn pop_normalization_dependency_scope(&mut self) -> NormalizationDependencyScope {
        self.normalization_dependency_stack
            .pop()
            .unwrap_or_default()
    }

    /// Record a dependency on a type for all active normalization scopes.
    pub fn record_normalization_dependency(&mut self, type_id: LocalTypeId) {
        for scope in &mut self.normalization_dependency_stack {
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
        for scope in &mut self.normalization_dependency_stack {
            scope.symbol_ids.insert(symbol_id);
        }
    }

    /// Return a cached rewrite entry when dependencies still match.
    pub fn rewrite_cached_type(
        &mut self,
        cache_key: u64,
        type_id: LocalTypeId,
    ) -> Option<RewriteCacheEntry> {
        let entry = self
            .rewrite_cache_by_key
            .get(&cache_key)?
            .get(&type_id)?
            .clone();
        if self.dependency_versions_are_valid(&entry.dependency_versions) {
            return Some(entry);
        }

        if let Some(cache) = self.rewrite_cache_by_key.get_mut(&cache_key) {
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
        self.rewrite_cache_by_key
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
        if let Some(type_id) = self.interned_union_by_elements.get(&elements).copied() {
            return type_id;
        }

        let type_id = self.insert_type_from_any(
            Type::Union {
                elements: elements.clone(),
            },
            self.get_type_source(source_type_id),
        );
        self.interned_union_by_elements.insert(elements, type_id);
        type_id
    }

    /// Intern an intersection type by its element list.
    pub fn intern_intersection_type(
        &mut self,
        elements: Vec<LocalTypeId>,
        source_type_id: LocalTypeId,
    ) -> LocalTypeId {
        if let Some(type_id) = self
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
        self.interned_intersection_by_elements
            .insert(elements, type_id);
        type_id
    }

    /// Validate dependency versions against current table state.
    fn dependency_versions_are_valid(
        &self,
        dependencies: &NormalizationDependencyVersions,
    ) -> bool {
        dependencies.type_versions.iter().all(|(type_id, version)| {
            self.type_version_by_id
                .get(type_id.0 as usize)
                .is_some_and(|current| current == version)
        }) && dependencies
            .symbol_versions
            .iter()
            .all(|(symbol_id, version)| self.symbol_version(*symbol_id) == *version)
    }

    /// Mark an alias normalization as in progress.
    pub fn mark_normalization_alias_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        self.normalization_alias_in_progress.insert(symbol_id);
    }

    /// Clear the alias normalization in progress marker.
    pub fn clear_normalization_alias_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        self.normalization_alias_in_progress.remove(&symbol_id);
    }

    /// Check whether an alias normalization is in progress.
    pub fn is_normalization_alias_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        self.normalization_alias_in_progress.contains(&symbol_id)
    }

    /// Get a cached normalized alias reference.
    pub fn normalized_alias_reference(
        &mut self,
        symbol_id: GlobalSymbolId,
        mode: NormalizationMode,
        relation_key: u64,
        arguments: &[StaticArgument],
    ) -> Option<AliasNormalizationEntry> {
        let entry_index = self.normalized_alias_by_symbol.iter().position(|entry| {
            entry.symbol == symbol_id
                && entry.mode == mode
                && entry.relation_key == relation_key
                && entry.arguments == arguments
        })?;
        let entry = self.normalized_alias_by_symbol[entry_index].clone();
        if self.dependency_versions_are_valid(&entry.dependency_versions) {
            return Some(entry);
        }

        self.normalized_alias_by_symbol.swap_remove(entry_index);
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
        if self.normalized_alias_by_symbol.iter().any(|entry| {
            entry.symbol == symbol_id
                && entry.mode == mode
                && entry.relation_key == relation_key
                && entry.arguments == arguments
        }) {
            return;
        }
        self.normalized_alias_by_symbol
            .push(AliasNormalizationEntry {
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
        self.assignability_in_progress
            .insert((target_id, source_id))
    }

    /// Clear assignability in progress for a target and source pair.
    pub fn clear_assignability_in_progress(
        &mut self,
        target_id: LocalTypeId,
        source_id: LocalTypeId,
    ) {
        self.assignability_in_progress
            .remove(&(target_id, source_id));
    }

    /// Get the number of types in the table.
    pub fn type_count(&self) -> u32 {
        self.next_type_id
    }

    /// Set the declared type for a node (from type annotation).
    pub fn set_declared_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.declared_type_by_node_id.insert(node_id, ty);
    }

    /// Get the declared type for a node.
    pub fn get_declared_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.declared_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the declared type id for a node.
    pub fn get_declared_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.declared_type_by_node_id.get(&node_id).copied()
    }

    /// Iterate over declared type ids keyed by node id.
    pub fn iter_declared_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.declared_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Set the inferred type for a node.
    pub fn set_inferred_type(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.inferred_type_by_node_id.insert(node_id, ty);
    }

    /// Clear all cached inferred types.
    pub fn clear_inferred_types(&mut self) {
        self.inferred_type_by_node_id.clear();
    }

    /// Get the inferred type for a node.
    pub fn get_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.inferred_type_by_node_id
            .get(&node_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the inferred type id for a node.
    pub fn get_inferred_type_id(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.inferred_type_by_node_id.get(&node_id).copied()
    }

    /// Iterate over inferred type ids keyed by node id.
    pub fn iter_inferred_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.inferred_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Get declared or fallback to inferred type for a node.
    pub fn get_declared_or_inferred_type(&self, node_id: GlobalNodeIdAny) -> Option<&Type> {
        self.get_declared_type(node_id)
            .or_else(|| self.get_inferred_type(node_id))
    }

    /// Get declared or fallback to inferred type id for a node.
    pub fn get_declared_or_inferred_type_id(
        &self,
        node_id: GlobalNodeIdAny,
    ) -> Option<LocalTypeId> {
        self.get_declared_type_id(node_id)
            .or_else(|| self.get_inferred_type_id(node_id))
    }

    /// Set the signature type for a declaration or member node.
    pub fn set_signature_type_for_node(&mut self, node_id: GlobalNodeIdAny, ty: LocalTypeId) {
        self.signature_type_by_node_id.insert(node_id, ty);
    }

    /// Get the signature type id for a declaration or member node.
    pub fn get_signature_type_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalTypeId> {
        self.signature_type_by_node_id.get(&node_id).copied()
    }

    /// Iterate over signature type ids keyed by node id.
    pub fn iter_signature_type_ids(
        &self,
    ) -> impl Iterator<Item = (GlobalNodeIdAny, LocalTypeId)> + '_ {
        self.signature_type_by_node_id
            .iter()
            .map(|(node_id, type_id)| (*node_id, *type_id))
    }

    /// Set the addressability for a node.
    pub fn set_addressability_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        addressability: Addressability,
    ) {
        self.addressability_by_node_id
            .insert(node_id, addressability);
    }

    /// Get the addressability for a node.
    pub fn get_addressability_for_node(&self, node_id: GlobalNodeIdAny) -> Option<Addressability> {
        self.addressability_by_node_id.get(&node_id).copied()
    }

    /// Copy node-local analysis data from a source node to a target node.
    /// Copies declared types, inferred types, signature types, addressability, instances, and resolutions.
    pub fn copy_node_analysis(&mut self, source: GlobalNodeIdAny, target: GlobalNodeIdAny) {
        // declared type
        if let Some(declared_type) = self.declared_type_by_node_id.get(&source).copied() {
            self.declared_type_by_node_id.insert(target, declared_type);
        }

        // inferred type
        if let Some(inferred_type) = self.inferred_type_by_node_id.get(&source).copied() {
            self.inferred_type_by_node_id.insert(target, inferred_type);
        }

        // signature type
        if let Some(signature_type) = self.signature_type_by_node_id.get(&source).copied() {
            self.signature_type_by_node_id
                .insert(target, signature_type);
        }

        // addressability
        if let Some(addressability) = self.addressability_by_node_id.get(&source).copied() {
            self.addressability_by_node_id
                .insert(target, addressability);
        }

        // instance
        if let Some(instance_id) = self.instance_by_node_id.get(&source).copied() {
            self.instance_by_node_id.insert(target, instance_id);
        }

        // resolution
        if let Some(resolution_id) = self.resolution_by_node_id.get(&source).copied() {
            self.resolution_by_node_id.insert(target, resolution_id);
        }
    }

    /// Set the instance type for a symbol (what type instances of this type have).
    pub fn set_instance_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.instance_type_by_symbol_id.insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the instance type for a symbol.
    pub fn get_instance_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.instance_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the instance type id for a symbol.
    pub fn get_instance_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.instance_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Find the symbol that owns an instance type id.
    pub fn symbol_for_instance_type(
        &self,
        instance_type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        self.instance_type_by_symbol_id
            .iter()
            .find_map(|(symbol, ty_id)| (*ty_id == instance_type_id).then_some(*symbol))
    }

    /// Cache the constraint type for a static parameter symbol.
    pub fn set_static_parameter_constraint_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        ty: LocalTypeId,
    ) {
        // cache the constraint type id
        self.static_parameter_constraint_by_symbol_id
            .insert(symbol_id, ty);
    }

    /// Get the cached constraint type for a static parameter symbol.
    pub fn get_static_parameter_constraint_type(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // fetch the cached constraint type id
        self.static_parameter_constraint_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Mark a static parameter constraint as in progress.
    pub fn mark_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        // record constraint resolution as in progress
        self.static_parameter_constraint_in_progress
            .insert(symbol_id);
    }

    /// Clear the in progress marker for a static parameter constraint.
    pub fn clear_static_parameter_constraint_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        // clear the in progress marker
        self.static_parameter_constraint_in_progress
            .remove(&symbol_id);
    }

    /// Check whether a static parameter constraint is in progress.
    pub fn is_static_parameter_constraint_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        // check whether resolution is in progress
        self.static_parameter_constraint_in_progress
            .contains(&symbol_id)
    }

    /// Cache the inferred kind for a static parameter symbol.
    pub fn set_static_parameter_kind(
        &mut self,
        symbol_id: GlobalSymbolId,
        kind: StaticParameterKind,
    ) {
        // cache the inferred kind
        self.static_parameter_kind_by_symbol_id
            .insert(symbol_id, kind);
    }

    /// Get the cached static parameter kind for a symbol.
    pub fn get_static_parameter_kind(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> Option<StaticParameterKind> {
        // fetch the cached kind
        self.static_parameter_kind_by_symbol_id
            .get(&symbol_id)
            .copied()
    }

    /// Mark a static parameter kind as in progress.
    pub fn mark_static_parameter_kind_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        // record kind inference as in progress
        self.static_parameter_kind_in_progress.insert(symbol_id);
    }

    /// Clear the in progress marker for a static parameter kind.
    pub fn clear_static_parameter_kind_in_progress(&mut self, symbol_id: GlobalSymbolId) {
        // clear the in progress marker
        self.static_parameter_kind_in_progress.remove(&symbol_id);
    }

    /// Check whether a static parameter kind is in progress.
    pub fn is_static_parameter_kind_in_progress(&self, symbol_id: GlobalSymbolId) -> bool {
        // check whether inference is in progress
        self.static_parameter_kind_in_progress.contains(&symbol_id)
    }

    /// Mark static argument resolution as in progress.
    pub fn mark_static_argument_resolution_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        arguments: Vec<StaticArgument>,
    ) {
        // record static argument resolution as in progress
        self.static_argument_resolution_in_progress
            .push((symbol_id, arguments));
    }

    /// Clear the in progress marker for static argument resolution.
    pub fn clear_static_argument_resolution_in_progress(
        &mut self,
        symbol_id: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) {
        // clear the in progress marker
        if let Some(index) = self
            .static_argument_resolution_in_progress
            .iter()
            .position(|(symbol, stored)| *symbol == symbol_id && stored == arguments)
        {
            self.static_argument_resolution_in_progress
                .swap_remove(index);
        }
    }

    /// Check whether static argument resolution is in progress.
    pub fn is_static_argument_resolution_in_progress(
        &self,
        symbol_id: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) -> bool {
        // check whether resolution is in progress
        self.static_argument_resolution_in_progress
            .iter()
            .any(|(symbol, stored)| *symbol == symbol_id && stored == arguments)
    }

    /// Mark expression type evaluation as in progress.
    pub fn mark_expression_type_in_progress(&mut self, node_id: GlobalNodeIdAny) {
        // record evaluation as in progress
        self.expression_type_in_progress.insert(node_id);
    }

    /// Clear the in progress marker for expression type evaluation.
    pub fn clear_expression_type_in_progress(&mut self, node_id: GlobalNodeIdAny) {
        // clear the in progress marker
        self.expression_type_in_progress.remove(&node_id);
    }

    /// Check whether expression type evaluation is in progress.
    pub fn is_expression_type_in_progress(&self, node_id: GlobalNodeIdAny) -> bool {
        // check whether evaluation is in progress
        self.expression_type_in_progress.contains(&node_id)
    }

    /// Set the value type for a symbol (what type this symbol has when used as a value).
    pub fn set_value_type(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.value_type_by_symbol_id.insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the value type for a symbol.
    pub fn get_value_type(&self, symbol_id: GlobalSymbolId) -> Option<&Type> {
        self.value_type_by_symbol_id
            .get(&symbol_id)
            .map(|ty| self.types.get(ty.0))
    }

    /// Get the value type id for a symbol.
    pub fn get_value_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.value_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Get the declared or inferred type id for a symbol in this module.
    pub fn get_type_id_for_symbol(
        &self,
        symbols: &SymbolTable,
        symbol_id: GlobalSymbolId,
    ) -> Option<LocalTypeId> {
        // prefer cached value types
        if let Some(value_type_id) = self.get_value_type_id(symbol_id) {
            return Some(value_type_id);
        }

        // skip non local symbols
        if symbol_id.module_id != self.module_id {
            return None;
        }

        // fall back to declared or inferred declaration types
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        self.get_declared_or_inferred_type_id(primary_declaration)
    }

    /// Set the declared target type id for an alias symbol.
    pub fn set_alias_target_type_id(&mut self, symbol_id: GlobalSymbolId, ty: LocalTypeId) {
        self.alias_target_type_by_symbol_id.insert(symbol_id, ty);
        self.bump_symbol_version(symbol_id);
    }

    /// Get the declared target type id for an alias symbol.
    pub fn get_alias_target_type_id(&self, symbol_id: GlobalSymbolId) -> Option<LocalTypeId> {
        self.alias_target_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Set the enum backing type for a symbol.
    pub fn set_enum_backing_type(
        &mut self,
        symbol_id: GlobalSymbolId,
        backing_type: EnumBackingType,
    ) {
        self.enum_backing_type_by_symbol_id
            .insert(symbol_id, backing_type);
    }

    /// Get the enum backing type for a symbol.
    pub fn get_enum_backing_type(&self, symbol_id: GlobalSymbolId) -> Option<EnumBackingType> {
        self.enum_backing_type_by_symbol_id.get(&symbol_id).copied()
    }

    /// Set the enum field value for a symbol.
    pub fn set_enum_field_value(&mut self, symbol_id: GlobalSymbolId, value: EnumFieldValue) {
        self.enum_field_value_by_symbol_id.insert(symbol_id, value);
    }

    /// Get the enum field value for a symbol.
    pub fn get_enum_field_value(&self, symbol_id: GlobalSymbolId) -> Option<EnumFieldValue> {
        self.enum_field_value_by_symbol_id.get(&symbol_id).copied()
    }

    /// Insert a new instance.
    pub fn insert_instance(&mut self, instance: Instance) -> LocalInstanceId {
        let instance_id = LocalInstanceId::new(self.next_instance_id);
        self.next_instance_id += 1;
        self.instances.allocate(instance);
        instance_id
    }

    /// Get an instance by its id.
    pub fn get_instance(&self, instance_id: LocalInstanceId) -> &Instance {
        self.instances.get(instance_id.0)
    }

    /// Get a mutable instance by its id.
    pub fn get_instance_mut(&mut self, instance_id: LocalInstanceId) -> &mut Instance {
        self.instances.get_mut(instance_id.0)
    }

    /// Set the instance used by a node id.
    pub fn set_instance_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        instance_id: LocalInstanceId,
    ) {
        self.instance_by_node_id.insert(node_id, instance_id);
    }

    /// Get the instance used by a node id.
    pub fn get_instance_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalInstanceId> {
        self.instance_by_node_id.get(&node_id).copied()
    }

    /// Find an existing instance by symbol and static arguments.
    pub fn find_instance(
        &self,
        symbol_id: GlobalSymbolId,
        static_arguments: &[StaticArgument],
    ) -> Option<LocalInstanceId> {
        for (index, instance) in self.instances.iter().enumerate() {
            if instance.symbol_id == symbol_id && instance.static_arguments == static_arguments {
                return Some(LocalInstanceId::new(index as u32));
            }
        }

        None
    }

    /// Insert a new resolution.
    pub fn insert_resolution(&mut self, resolution: Resolution) -> LocalResolutionId {
        let resolution_id = LocalResolutionId::new(self.next_resolution_id);
        self.next_resolution_id += 1;
        self.resolutions.allocate(resolution);
        resolution_id
    }

    /// Get a resolution by its id.
    pub fn get_resolution(&self, resolution_id: LocalResolutionId) -> &Resolution {
        self.resolutions.get(resolution_id.0)
    }

    /// Get a mutable resolution by its id.
    pub fn get_resolution_mut(&mut self, resolution_id: LocalResolutionId) -> &mut Resolution {
        self.resolutions.get_mut(resolution_id.0)
    }

    /// Set the resolution used by a node id.
    pub fn set_resolution_for_node(
        &mut self,
        node_id: GlobalNodeIdAny,
        resolution_id: LocalResolutionId,
    ) {
        self.resolution_by_node_id.insert(node_id, resolution_id);
    }

    /// Get the resolution used by a node id.
    pub fn get_resolution_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalResolutionId> {
        self.resolution_by_node_id.get(&node_id).copied()
    }

    /// Insert a new lineage.
    pub fn insert_lineage(&mut self, lineage: Lineage) -> LocalLineageId {
        let lineage_id = LocalLineageId::new(self.next_lineage_id);
        self.next_lineage_id += 1;
        self.lineages.allocate(lineage);
        lineage_id
    }

    /// Get a lineage by its id.
    pub fn get_lineage(&self, lineage_id: LocalLineageId) -> &Lineage {
        self.lineages.get(lineage_id.0)
    }

    /// Get a mutable lineage by its id.
    pub fn get_lineage_mut(&mut self, lineage_id: LocalLineageId) -> &mut Lineage {
        self.lineages.get_mut(lineage_id.0)
    }

    /// Set the lineage for a symbol (type declaration).
    pub fn set_lineage_for_symbol(
        &mut self,
        symbol_id: GlobalSymbolId,
        lineage_id: LocalLineageId,
    ) {
        self.lineage_by_symbol_id.insert(symbol_id, lineage_id);
    }

    /// Get the lineage id for a symbol.
    pub fn get_lineage_id_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<LocalLineageId> {
        self.lineage_by_symbol_id.get(&symbol_id).copied()
    }

    /// Get the lineage for a symbol directly.
    pub fn get_lineage_for_symbol(&self, symbol_id: GlobalSymbolId) -> Option<&Lineage> {
        self.lineage_by_symbol_id
            .get(&symbol_id)
            .map(|id| self.lineages.get(id.0))
    }

    /// Insert a new extension.
    pub fn insert_extension(&mut self, extension: Extension) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.next_extension_id);
        self.next_extension_id += 1;

        // index by target symbol for member lookup
        let target = extension.target;
        self.extensions_by_target
            .entry(target)
            .or_default()
            .push(extension_id);

        // index by extension symbol
        self.extension_by_symbol
            .insert(extension.symbol, extension_id);

        self.extensions.allocate(extension);
        extension_id
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.extensions.get(extension_id.0)
    }

    /// Get an extension id by its symbol.
    pub fn get_extension_id_for_symbol(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.extension_by_symbol.get(&extension_symbol).copied()
    }

    /// Get a mutable extension by its id.
    pub fn get_extension_mut(&mut self, extension_id: LocalExtensionId) -> &mut Extension {
        self.extensions.get_mut(extension_id.0)
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn get_extensions_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extensions_by_target.get(&target_symbol)
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> {
        (0..self.next_extension_id).map(|i| {
            let id = LocalExtensionId::new(i);
            (id, self.extensions.get(i))
        })
    }

    /// Iterate over all lineages with their associated symbol ids.
    pub fn iter_lineages(&self) -> impl Iterator<Item = (GlobalSymbolId, &Lineage)> {
        self.lineage_by_symbol_id
            .iter()
            .map(|(symbol_id, lineage_id)| (*symbol_id, self.lineages.get(lineage_id.0)))
    }
}
