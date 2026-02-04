use destack_dir::{
    LocalNodeIdAny, LocalTypeId, Type, TypeRewriter, TypeRewriterOptions, TypeTable,
};

use super::{
    REWRITER_TAG_READONLY, TypeRewriteCache, TypeWalkContext, TypeWalkKey, rewrite_type_with_cache,
};
/// The mode used when materializing types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MaterializationMode {
    /// Materialize a shape without validating bounds.
    Shape,
    /// Materialize types for validation and assignability checks.
    Validation,
    /// Materialize surface types for exports and substitutions.
    Surface,
}

/// Placeholder to keep the shape mode reachable during refactors.
const _SHAPE_MODE: MaterializationMode = MaterializationMode::Shape;

/// Materialize readonly modifiers by rewriting types.
pub(crate) struct ReadonlyMaterializer {
    /// The source node to attribute new types to.
    source_id: LocalNodeIdAny,
    /// The cached mapped type ids.
    cache: TypeRewriteCache,
    /// The cache key for rewrites.
    cache_key: u64,
    /// The rewriter options.
    options: TypeRewriterOptions,
}

impl ReadonlyMaterializer {
    /// Create a readonly materializer for the given source.
    pub(crate) fn new(source_id: LocalNodeIdAny) -> Self {
        let walk_context =
            TypeWalkContext::new(TypeWalkKey::BASE).with_rewriter_tag(REWRITER_TAG_READONLY);
        let walk_context = walk_context.with_context_key(source_id.cache_key());
        let rewrite_options = walk_context.rewriter_options();
        let cache_key = rewrite_options.cache_key();
        Self {
            source_id,
            cache: TypeRewriteCache::new(),
            cache_key,
            options: rewrite_options,
        }
    }

    /// Materialize readonly modifiers at the surface only.
    pub(crate) fn apply_shallow(&self, types: &mut TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        self.apply_readonly_flags(type_id, types)
    }

    /// Apply readonly flags to an already rewritten type.
    fn apply_readonly_flags(&self, mapped_id: LocalTypeId, types: &mut TypeTable) -> LocalTypeId {
        let ty = types.get_type(mapped_id).clone();
        match ty {
            Type::Object {
                mut fields,
                call_signatures,
                construct_signatures,
                mut index_signatures,
            } => {
                // mark object fields and index signatures as readonly
                let mut changed = false;
                for field in fields.iter_mut() {
                    if !field.is_readonly {
                        field.is_readonly = true;
                        changed = true;
                    }
                }
                for signature in index_signatures.iter_mut() {
                    if !signature.is_readonly {
                        signature.is_readonly = true;
                        changed = true;
                    }
                }
                if !changed {
                    return mapped_id;
                }
                types.insert_type_from_any(
                    Type::Object {
                        fields,
                        call_signatures,
                        construct_signatures,
                        index_signatures,
                    },
                    self.source_id,
                )
            }
            Type::Tuple {
                mut elements,
                is_readonly,
            } => {
                // mark tuple elements as readonly
                let mut changed = false;
                if !is_readonly {
                    changed = true;
                }
                for element in elements.iter_mut() {
                    if !element.is_readonly {
                        element.is_readonly = true;
                        changed = true;
                    }
                }
                if !changed {
                    return mapped_id;
                }
                types.insert_type_from_any(
                    Type::Tuple {
                        elements,
                        is_readonly: true,
                    },
                    self.source_id,
                )
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                if is_readonly {
                    return mapped_id;
                }
                types.insert_type_from_any(
                    Type::ArraySized {
                        element,
                        count,
                        is_readonly: true,
                    },
                    self.source_id,
                )
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                if is_readonly {
                    return mapped_id;
                }
                types.insert_type_from_any(
                    Type::Array {
                        element,
                        is_readonly: true,
                    },
                    self.source_id,
                )
            }
            _ => mapped_id,
        }
    }
}

impl TypeRewriter for ReadonlyMaterializer {
    fn options(&self) -> &TypeRewriterOptions {
        &self.options
    }

    fn rewrite_any(
        &mut self,
        _types: &mut TypeTable,
        type_id: LocalTypeId,
        ty: &Type,
    ) -> Option<LocalTypeId> {
        // avoid rewriting through nominal references
        if matches!(ty, Type::Reference { .. } | Type::Import { .. }) {
            return Some(type_id);
        }

        None
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, type_id);

        // apply readonly flags after rewrites
        let mapped = self.apply_readonly_flags(mapped, types);
        cache.insert((self.cache_key, type_id), mapped);
        self.cache = cache;
        mapped
    }
}
