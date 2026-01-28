use std::collections::HashMap;

use destack_dir::{LocalTypeId, TypeRewriter, TypeRewriterOptions, TypeTable, TypeVisitorOptions};

use super::MaterializationMode;

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

    cache.insert(key, type_id);
    let ty = types.get_type(type_id).clone();
    let mapped = rewriter.rewrite_type(types, type_id, &ty);
    cache.insert(key, mapped);
    mapped
}

const MATERIALIZATION_MODE_SHIFT: u64 = 22;

fn materialization_mode_key(mode: MaterializationMode) -> u64 {
    match mode {
        MaterializationMode::Shape => 0,
        MaterializationMode::Validation => 1,
        MaterializationMode::Surface => 2,
    }
}
