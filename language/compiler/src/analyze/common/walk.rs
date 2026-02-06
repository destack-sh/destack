use std::collections::HashMap;

use destack_dir::{
    LocalTypeId, Type, TypeRewriter, TypeRewriterOptions, TypeTable, TypeVisitor,
    TypeVisitorOptions,
};

use super::MaterializationMode;

/// Collect referenced type ids while walking a type graph.
pub(crate) struct TypeCollector<'a> {
    /// The types collected during the walk.
    collected: &'a mut Vec<LocalTypeId>,
    /// The visitor options.
    options: TypeVisitorOptions,
}

impl<'a> TypeCollector<'a> {
    /// Create a visitor that records referenced type ids.
    pub(crate) fn new(pending: &'a mut Vec<LocalTypeId>, options: TypeVisitorOptions) -> Self {
        Self {
            collected: pending,
            options,
        }
    }
}

impl TypeVisitor for TypeCollector<'_> {
    fn options(&self) -> &TypeVisitorOptions {
        &self.options
    }

    fn visit_type_id(&mut self, _types: &TypeTable, id: LocalTypeId) {
        self.collected.push(id);
    }
}

/// A cache key for type walking and rewriting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeWalkKey(u64);

impl TypeWalkKey {
    /// The default key for walking without contextual modes.
    pub(crate) const BASE: Self = Self(0);

    /// Add materialization mode bits to the key.
    pub(crate) fn with_materialization_mode(self, mode: MaterializationMode) -> Self {
        let key = self.0 | (materialization_mode_key(mode) << MATERIALIZATION_MODE_SHIFT);
        Self(key)
    }

    /// Add a rewriter tag to the cache key.
    pub(crate) fn with_rewriter_tag(self, tag: u64) -> Self {
        let key = self.0 | (tag << REWRITER_TAG_SHIFT);
        Self(key)
    }

    /// Combine an explicit context key into the cache key.
    pub(crate) fn with_context_key(self, context_key: u64) -> Self {
        Self(self.0 ^ context_key)
    }

    /// Return the raw key value.
    pub(crate) fn value(self) -> u64 {
        self.0
    }
}

/// Context for shared type walking and rewriting helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeWalkContext {
    key: TypeWalkKey,
}

impl TypeWalkContext {
    /// Create a new walk context from a cache key.
    pub(crate) fn new(key: TypeWalkKey) -> Self {
        Self { key }
    }

    /// Create a walk context for a materialization mode.
    pub(crate) fn for_materialization(mode: MaterializationMode) -> Self {
        Self::new(TypeWalkKey::BASE.with_materialization_mode(mode))
    }

    /// Extend the walk context with a rewriter tag.
    pub(crate) fn with_rewriter_tag(self, tag: u64) -> Self {
        Self {
            key: self.key.with_rewriter_tag(tag),
        }
    }

    /// Extend the walk context with an explicit context key.
    pub(crate) fn with_context_key(self, context_key: u64) -> Self {
        Self {
            key: self.key.with_context_key(context_key),
        }
    }

    /// Return rewriter options for this walk context.
    pub(crate) fn rewriter_options(self) -> TypeRewriterOptions {
        TypeRewriterOptions::new(self.key.value())
    }

    /// Return visitor options for this walk context.
    pub(crate) fn visitor_options(self) -> TypeVisitorOptions {
        TypeVisitorOptions::new(self.key.value())
    }
}

/// Cache storage for type rewrites keyed by mode.
pub(crate) type TypeRewriteCache = HashMap<(u64, LocalTypeId), LocalTypeId>;

/// Rewrite a type id using a shared cache.
pub(crate) fn rewrite_type_with_cache<V: TypeRewriter + ?Sized>(
    rewriter: &mut V,
    types: &mut TypeTable,
    cache: &mut TypeRewriteCache,
    cache_key: u64,
    type_id: LocalTypeId,
) -> LocalTypeId {
    let key = (cache_key, type_id);
    if let Some(mapped) = cache.get(&key).copied() {
        return mapped;
    }

    if let Some(entry) = types.rewrite_cached_type(cache_key, type_id) {
        for (dependency_id, _) in &entry.dependency_versions.type_versions {
            types.record_normalization_dependency(*dependency_id);
        }
        for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
            types.record_normalization_symbol_dependency(*dependency_id);
        }
        cache.insert(key, entry.mapped_type);
        return entry.mapped_type;
    }

    types.push_normalization_dependency_scope();
    types.record_normalization_dependency(type_id);
    if let Type::Reference { symbol, .. } = types.get_type(type_id) {
        types.record_normalization_symbol_dependency(*symbol);
    }

    cache.insert(key, type_id);
    let ty = types.get_type(type_id).clone();
    let mapped = rewriter.rewrite_type(types, type_id, &ty);
    cache.insert(key, mapped);

    let dependencies = types.pop_normalization_dependency_scope();
    let dependency_versions = types.collect_dependency_versions(dependencies);
    types.set_rewrite_cached_type(cache_key, type_id, mapped, dependency_versions);
    mapped
}

const MATERIALIZATION_MODE_SHIFT: u64 = 22;
const REWRITER_TAG_SHIFT: u64 = 32;

pub(crate) const REWRITER_TAG_READONLY: u64 = 1;
pub(crate) const REWRITER_TAG_INFER_SUBSTITUTION: u64 = 2;
pub(crate) const REWRITER_TAG_INFER_MATERIALIZER: u64 = 3;
pub(crate) const REWRITER_TAG_LITERAL_WIDENING: u64 = 4;
pub(crate) const REWRITER_TAG_STATIC_ARGUMENT: u64 = 5;
pub(crate) const REWRITER_TAG_ASSOCIATED_ALIAS: u64 = 6;

fn materialization_mode_key(mode: MaterializationMode) -> u64 {
    match mode {
        MaterializationMode::Shape => 0,
        MaterializationMode::Validation => 1,
        MaterializationMode::Surface => 2,
    }
}
