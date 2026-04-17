use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use destack_dir::{
    GlobalSymbolId, LocalNodeIdAny, LocalTypeId, NormalizationMode, PrimitiveType, ScalarLiteral,
    StaticArgument, StaticExpression, StaticProperty, StringId, SymbolKind, SymbolSpace,
    SymbolTable, SymbolType, Type, TypeElement, TypeLiteral, TypeRewriter, TypeRewriterOptions,
    TypeTable,
};
use destack_workspace::Module;

use super::{
    REWRITER_TAG_INFER_SUBSTITUTION, SymbolTypeView, TypeContext, TypeRewriteCache,
    TypeWalkContext, TypeWalkKey, rewrite_type_with_cache,
};
use crate::Compiler;

/// Substitutions captured from conditional infer patterns.
#[derive(Debug, Clone)]
pub(crate) struct InferSubstitutions {
    cache_key: u64,
    /// Inferred bindings keyed by infer name.
    by_name: HashMap<StringId, LocalTypeId>,
}

impl InferSubstitutions {
    /// Create an empty substitution set with a fresh cache key.
    fn empty() -> Self {
        Self::from_map(HashMap::new())
    }

    /// Return the cache key for these substitutions.
    fn cache_key(&self) -> u64 {
        self.cache_key
    }

    /// Get the substitution for an inferred binding name.
    fn get(&self, name: &StringId) -> Option<LocalTypeId> {
        self.by_name.get(name).copied()
    }

    /// Build a substitution set from a map.
    fn from_map(by_name: HashMap<StringId, LocalTypeId>) -> Self {
        Self {
            cache_key: infer_substitution_cache_key(&by_name),
            by_name,
        }
    }

    /// Iterate over substitution entries.
    fn into_entries(self) -> impl Iterator<Item = (StringId, LocalTypeId)> {
        self.by_name.into_iter()
    }
}

/// Builder for immutable substitution sets.
struct InferSubstitutionsBuilder<'a> {
    _compiler: std::marker::PhantomData<&'a Compiler>,
    by_name: HashMap<StringId, LocalTypeId>,
}

impl<'a> InferSubstitutionsBuilder<'a> {
    /// Create a new builder with an empty substitution set.
    fn new() -> Self {
        Self {
            _compiler: std::marker::PhantomData,
            by_name: HashMap::new(),
        }
    }

    /// Get the substitution for an inferred binding name.
    fn get(&self, name: &StringId) -> Option<LocalTypeId> {
        self.by_name.get(name).copied()
    }

    /// Insert a substitution entry, returning whether the map changed.
    fn insert(&mut self, name: StringId, ty: LocalTypeId) -> bool {
        let changed = self.by_name.get(&name).copied() != Some(ty);
        if changed {
            self.by_name.insert(name, ty);
        }
        changed
    }

    /// Finish the builder and return an immutable substitution set.
    fn build(self) -> InferSubstitutions {
        InferSubstitutions::from_map(self.by_name)
    }
}

/// Build a deterministic cache key for inferred substitutions.
fn infer_substitution_cache_key(substitutions: &HashMap<StringId, LocalTypeId>) -> u64 {
    // keep empty substitutions stable and cheap
    if substitutions.is_empty() {
        return 0x8d3c_4fb1_5e0a_1d79;
    }

    // sort by infer binding name for deterministic hashing
    let mut entries = substitutions
        .iter()
        .map(|(name, ty)| (*name, *ty))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(name, _)| *name);

    // hash the ordered entries into one cache key
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    entries.len().hash(&mut hasher);
    for (name, type_id) in entries {
        name.hash(&mut hasher);
        type_id.hash(&mut hasher);
    }

    hasher.finish()
}

/// Merge mode for inferred bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferMergeMode {
    /// Union inferred candidates (covariant positions).
    Union,
    /// Intersect inferred candidates (contravariant positions).
    Intersection,
}

/// Rewrite types by substituting inferred bindings.
struct InferSubstitutionRewriter<'a> {
    /// The compiler instance.
    compiler: &'a Compiler,
    /// The current module.
    module: &'a Module,
    /// The inferred substitutions to apply.
    substitutions: &'a InferSubstitutions,
    /// The symbol table for the current module.
    symbols: &'a SymbolTable,
    /// The cached mapped type ids.
    cache: TypeRewriteCache,
    /// The cache key for rewrites.
    cache_key: u64,
    /// The rewriter options.
    options: TypeRewriterOptions,
}

impl<'a> InferSubstitutionRewriter<'a> {
    /// Create a rewriter for inferred bindings.
    fn new(
        compiler: &'a Compiler,
        module: &'a Module,
        substitutions: &'a InferSubstitutions,
        symbols: &'a SymbolTable,
    ) -> Self {
        let walk_context = TypeWalkContext::new(TypeWalkKey::BASE)
            .with_rewriter_tag(REWRITER_TAG_INFER_SUBSTITUTION);
        let base_key = walk_context.rewriter_options().cache_key();
        let cache_key = base_key ^ substitutions.cache_key();
        let rewrite_options = TypeRewriterOptions::new(cache_key);
        Self {
            compiler,
            module,
            substitutions,
            symbols,
            cache: TypeRewriteCache::new(),
            cache_key,
            options: rewrite_options,
        }
    }
}

impl TypeRewriter for InferSubstitutionRewriter<'_> {
    fn options(&self) -> &TypeRewriterOptions {
        &self.options
    }

    fn rewrite_any(
        &mut self,
        _types: &mut TypeTable,
        type_id: LocalTypeId,
        ty: &Type,
    ) -> Option<LocalTypeId> {
        // substitute explicit infer bindings by name
        if let Type::Infer { name, .. } = ty {
            return Some(self.substitutions.get(name).unwrap_or(type_id));
        }

        // substitute local infer binding references without static arguments
        if let Type::Reference {
            symbol,
            static_arguments,
        } = ty
            && static_arguments.is_none()
            && let Some(mapped) = self.compiler.infer_binding_substitution(
                self.module,
                *symbol,
                self.substitutions,
                self.symbols,
            )
        {
            return Some(mapped);
        }

        None
    }

    fn rewrite_type_id(&mut self, types: &mut TypeTable, type_id: LocalTypeId) -> LocalTypeId {
        let mut cache = std::mem::take(&mut self.cache);
        let mapped = rewrite_type_with_cache(self, types, &mut cache, self.cache_key, type_id);
        self.cache = cache;
        mapped
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return the distributive static parameter symbol for a conditional left side.
    pub(crate) fn conditional_left_distributive_symbol(
        &self,
        ctx: SymbolTypeView<'_>,
        type_id: LocalTypeId,
    ) -> Option<GlobalSymbolId> {
        // only naked static parameters distribute
        match ctx.types.get_type(type_id) {
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                if static_arguments.is_none() && self.symbol_is_static_parameter(ctx, *symbol) {
                    Some(*symbol)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Infer substitutions for conditional types with `infer` bindings.
    pub(crate) fn infer_conditional_type_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> Option<InferSubstitutions> {
        // collect substitutions for each matching branch
        let mut visited = HashSet::new();
        self.infer_conditional_type_substitutions_inner(
            &mut ctx.reborrow(),
            distributive,
            left,
            right,
            source_id,
            &mut visited,
        )
    }

    /// Infer substitutions for conditional type matching with recursion guard.
    fn infer_conditional_type_substitutions_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // stop on recursion cycles
        if !visited.insert((left, right)) {
            return Some(InferSubstitutions::empty());
        }

        // infer substitution matching normalizes aliases eagerly
        // infer from matching reference arguments before normalization
        let left_type = ctx.types.get_type(left).clone();
        let right_type = ctx.types.get_type(right).clone();
        if let (
            Type::Reference {
                symbol,
                static_arguments: left_arguments,
            },
            Type::Reference {
                symbol: right_symbol,
                static_arguments: right_arguments,
            },
        ) = (&left_type, &right_type)
            && symbol == right_symbol
            && let (Some(left_arguments), Some(right_arguments)) =
                (left_arguments.as_deref(), right_arguments.as_deref())
            && let Some(inferred) = self.infer_conditional_substitutions_for_reference_arguments(
                &mut ctx.reborrow(),
                left_arguments,
                right_arguments,
                distributive,
                source_id,
                visited,
            )
        {
            return Some(inferred);
        }

        // unwrap alias references before matching
        let left = self.unwrap_normalization_alias_reference(left, ctx.types);
        let right = self.unwrap_normalization_alias_reference(right, ctx.types);

        // normalize alias references with concrete arguments before matching
        // conditional inference uses assign normalization mode
        let left = self.normalize_type(&mut ctx.reborrow(), left, NormalizationMode::Assign);
        let right = self.normalize_type(&mut ctx.reborrow(), right, NormalizationMode::Assign);

        // read the current type shapes
        let left_type = ctx.types.get_type(left).clone();
        let right_type = ctx.types.get_type(right).clone();

        // handle infer bindings and trivial matches early
        match (&left_type, &right_type) {
            (_, Type::Infer { name, .. }) => {
                let mut substitutions = InferSubstitutionsBuilder::new();
                substitutions.insert(*name, left);
                return Some(substitutions.build());
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                _,
            ) => {
                return None;
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                },
                _,
            ) => {
                return self.infer_substitutions_for_any_left(
                    &mut ctx.reborrow(),
                    distributive,
                    left,
                    right,
                    source_id,
                );
            }
            _ => {}
        }

        // infer defaults for never inputs
        if matches!(
            left_type,
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            }
        ) {
            return self.infer_substitutions_for_never_left(&mut ctx.reborrow(), right, source_id);
        }

        // resolve static parameter constraints for left and right sides
        {
            if let Type::Reference { symbol, .. } = &left_type
                && self.symbol_is_static_parameter(ctx.symbol_type_view(), *symbol)
            {
                let constraint_id =
                    self.static_parameter_constraint_type(&mut ctx.reborrow(), *symbol, source_id);
                if let Some(constraint_id) = constraint_id {
                    return self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        constraint_id,
                        right,
                        source_id,
                        visited,
                    );
                }
            }

            if let Type::Reference { symbol, .. } = &right_type
                && self.symbol_is_static_parameter(ctx.symbol_type_view(), *symbol)
            {
                let constraint_id =
                    self.static_parameter_constraint_type(&mut ctx.reborrow(), *symbol, source_id);
                if let Some(constraint_id) = constraint_id {
                    return self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left,
                        constraint_id,
                        source_id,
                        visited,
                    );
                }
            }
        }

        // short-circuit identical type aliases
        if let (
            Type::Reference { symbol, .. },
            Type::Reference {
                symbol: right_symbol,
                ..
            },
        ) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
            && right_symbol.ty() == SymbolType::TypeAlias
            && symbol == right_symbol
        {
            return Some(InferSubstitutions::empty());
        }

        // compare instance types for identical references
        if let (
            Type::Reference {
                symbol,
                static_arguments,
            },
            Type::Reference {
                symbol: right_symbol,
                static_arguments: right_arguments,
            },
        ) = (&left_type, &right_type)
            && symbol == right_symbol
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && right_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(left_instance_id) =
                self.apparent_instance_type(&mut ctx.reborrow(), source_id, *symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                left_instance_id,
                right,
                source_id,
                visited,
            );
        }

        // expand left instance types when available
        if let (
            Type::Reference {
                symbol,
                static_arguments,
            },
            _,
        ) = (&left_type, &right_type)
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(left_instance_id) =
                self.apparent_instance_type(&mut ctx.reborrow(), source_id, *symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                left_instance_id,
                right,
                source_id,
                visited,
            );
        }

        // expand right instance types when available
        if let (
            _,
            Type::Reference {
                symbol,
                static_arguments,
            },
        ) = (&left_type, &right_type)
            && static_arguments
                .as_ref()
                .is_none_or(|arguments| arguments.is_empty())
            && let Some(right_instance_id) =
                self.apparent_instance_type(&mut ctx.reborrow(), source_id, *symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                left,
                right_instance_id,
                source_id,
                visited,
            );
        }

        // normalize alias references before structural matching
        if let (Type::Reference { symbol, .. }, _) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_left =
                self.normalize_type(&mut ctx.reborrow(), left, NormalizationMode::Flow);
            if normalized_left != left {
                return self.infer_conditional_type_substitutions_inner(
                    &mut ctx.reborrow(),
                    distributive,
                    normalized_left,
                    right,
                    source_id,
                    visited,
                );
            }
        }

        // normalize alias references before structural matching
        if let (_, Type::Reference { symbol, .. }) = (&left_type, &right_type)
            && symbol.ty() == SymbolType::TypeAlias
        {
            let normalized_right =
                self.normalize_type(&mut ctx.reborrow(), right, NormalizationMode::Flow);
            if normalized_right != right {
                return self.infer_conditional_type_substitutions_inner(
                    &mut ctx.reborrow(),
                    distributive,
                    left,
                    normalized_right,
                    source_id,
                    visited,
                );
            }
        }

        // expose callable value shapes for matching
        if let (Type::Reference { symbol, .. }, _) = (&left_type, &right_type)
            && let Some(function_id) = ctx.types.get_value_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                function_id,
                right,
                source_id,
                visited,
            );
        }

        // expose callable value shapes for matching
        if let (_, Type::Reference { symbol, .. }) = (&left_type, &right_type)
            && let Some(function_id) = ctx.types.get_value_type_id(*symbol)
        {
            return self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                left,
                function_id,
                source_id,
                visited,
            );
        }

        // structural matching across type shapes
        match (left_type, right_type) {
            (Type::Conditional { .. }, _) | (_, Type::Conditional { .. }) => {
                Some(InferSubstitutions::empty())
            }
            (Type::Union { elements }, _) => {
                let mut combined = InferSubstitutionsBuilder::new();
                for element_id in elements {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        element_id,
                        right,
                        source_id,
                        visited,
                    );
                    match inferred {
                        Some(inferred) => {
                            self.merge_infer_substitutions(
                                &mut ctx.reborrow(),
                                &mut combined,
                                inferred,
                                InferMergeMode::Union,
                            );
                        }
                        None => {
                            if !distributive {
                                return None;
                            }
                        }
                    }
                }
                Some(combined.build())
            }
            (_, Type::Union { elements }) => {
                let mut combined = InferSubstitutionsBuilder::new();
                // require at least one matching union branch
                let mut matched_any = false;
                for element_id in elements {
                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left,
                        element_id,
                        source_id,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            &mut ctx.reborrow(),
                            &mut combined,
                            inferred,
                            InferMergeMode::Union,
                        );
                        matched_any = true;
                    }
                }
                if !matched_any {
                    return None;
                }
                Some(combined.build())
            }
            (Type::Intersection { elements }, _) => {
                let mut combined = InferSubstitutionsBuilder::new();
                for element_id in elements {
                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        element_id,
                        right,
                        source_id,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            &mut ctx.reborrow(),
                            &mut combined,
                            inferred,
                            InferMergeMode::Union,
                        );
                    }
                }
                Some(combined.build())
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                },
                Type::TemplateLiteral { strings, spans },
            ) => {
                let value = self.repository.strings.get(string_id).to_string();
                self.infer_template_substitutions_from_string(
                    &mut ctx.reborrow(),
                    &value,
                    &strings,
                    &spans,
                    source_id,
                )
            }
            (
                Type::TemplateLiteral {
                    strings: left_strings,
                    spans: left_spans,
                },
                Type::TemplateLiteral {
                    strings: right_strings,
                    spans: right_spans,
                },
            ) if left_spans.is_empty() => {
                // treat spanless templates as string literals for matching
                let mut value = String::new();
                for string_id in left_strings {
                    let fragment = self.repository.strings.get(string_id);
                    value.push_str(fragment.as_ref());
                }
                self.infer_template_substitutions_from_string(
                    &mut ctx.reborrow(),
                    &value,
                    &right_strings,
                    &right_spans,
                    source_id,
                )
            }
            (
                Type::TemplateLiteral {
                    strings: left_strings,
                    spans: left_spans,
                },
                Type::TemplateLiteral {
                    strings: right_strings,
                    spans: right_spans,
                },
            ) => self.infer_template_substitutions_from_template(
                &mut ctx.reborrow(),
                distributive,
                &left_strings,
                &left_spans,
                &right_strings,
                &right_spans,
                source_id,
                visited,
            ),
            (_, Type::Intersection { elements }) => {
                let mut combined = InferSubstitutionsBuilder::new();
                for element_id in elements {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left,
                        element_id,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                Some(combined.build())
            }
            (
                Type::Array { element, .. },
                Type::Array {
                    element: right_element,
                    ..
                },
            ) => {
                let Some(left_element) = element else {
                    return Some(InferSubstitutions::empty());
                };
                let Some(right_element) = right_element else {
                    return Some(InferSubstitutions::empty());
                };
                self.infer_conditional_type_substitutions_inner(
                    &mut ctx.reborrow(),
                    distributive,
                    left_element,
                    right_element,
                    source_id,
                    visited,
                )
            }
            (
                Type::ArraySized { element, .. },
                Type::ArraySized {
                    element: right_element,
                    ..
                },
            ) => self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                element,
                right_element,
                source_id,
                visited,
            ),
            (
                Type::Tuple {
                    elements,
                    is_readonly,
                },
                Type::Tuple {
                    elements: right_elements,
                    is_readonly: right_is_readonly,
                },
            ) => {
                if is_readonly != right_is_readonly {
                    return None;
                }
                if elements.len() != right_elements.len() {
                    return None;
                }
                let mut combined = InferSubstitutionsBuilder::new();
                for (left_element, right_element) in elements.iter().zip(right_elements.iter()) {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left_element.ty,
                        right_element.ty,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                Some(combined.build())
            }
            (
                Type::Function {
                    static_parameters,
                    this_parameter,
                    dynamic_parameters,
                    return_type,
                    ..
                },
                Type::Function {
                    static_parameters: right_static_parameters,
                    this_parameter: right_this_parameter,
                    dynamic_parameters: right_dynamic_parameters,
                    return_type: right_return_type,
                    ..
                },
            ) => {
                // ensure static parameter counts match
                if static_parameters.len() != right_static_parameters.len() {
                    return None;
                }

                // infer static parameter substitutions
                let mut combined = InferSubstitutionsBuilder::new();
                for (left_parameter, right_parameter) in
                    static_parameters.iter().zip(right_static_parameters.iter())
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        *left_parameter,
                        *right_parameter,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                // infer this-parameter substitutions
                let inferred = match (this_parameter, right_this_parameter) {
                    (Some(left_parameter), Some(right_parameter)) => self
                        .infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            left_parameter,
                            right_parameter,
                            source_id,
                            visited,
                        ),
                    (None, None) => Some(InferSubstitutions::empty()),
                    _ => None,
                }?;
                self.merge_infer_substitutions(
                    &mut ctx.reborrow(),
                    &mut combined,
                    inferred,
                    InferMergeMode::Intersection,
                );

                // check dynamic parameter matching
                let mut matched_dynamic = false;
                if dynamic_parameters.len() == right_dynamic_parameters.len() {
                    for (left_parameter, right_parameter) in dynamic_parameters
                        .iter()
                        .zip(right_dynamic_parameters.iter())
                    {
                        let inferred = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            *left_parameter,
                            *right_parameter,
                            source_id,
                            visited,
                        )?;
                        self.merge_infer_substitutions(
                            &mut ctx.reborrow(),
                            &mut combined,
                            inferred,
                            InferMergeMode::Intersection,
                        );
                    }
                    matched_dynamic = true;
                } else if right_dynamic_parameters.len() == 1 {
                    let only = right_dynamic_parameters[0];
                    let tuple_type = ctx.types.insert_type_from_any(
                        Type::Tuple {
                            elements: dynamic_parameters
                                .iter()
                                .map(|parameter| TypeElement::new(*parameter))
                                .collect(),
                            is_readonly: false,
                        },
                        source_id,
                    );

                    if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        tuple_type,
                        only,
                        source_id,
                        visited,
                    ) {
                        self.merge_infer_substitutions(
                            &mut ctx.reborrow(),
                            &mut combined,
                            inferred,
                            InferMergeMode::Intersection,
                        );
                        matched_dynamic = true;
                    } else {
                        // fall back to assignability for variadic patterns
                        matched_dynamic = self
                            .is_type_assignable(&mut ctx.reborrow(), only, tuple_type)
                            .is_assignable();
                    }
                }

                if !matched_dynamic {
                    return None;
                }

                // infer return type substitutions
                if let (Some(left_return), Some(right_return)) = (return_type, right_return_type)
                    && self.type_contains_infer(right_return, ctx.types, &mut HashSet::new())
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left_return,
                        right_return,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                Some(combined.build())
            }
            (
                Type::Function { .. },
                Type::Object {
                    call_signatures,
                    construct_signatures,
                    ..
                },
            ) => {
                // match function types against callable object patterns
                let mut combined = InferSubstitutionsBuilder::new();
                for right_signature in call_signatures
                    .iter()
                    .chain(construct_signatures.iter())
                    .copied()
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left,
                        right_signature,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                Some(combined.build())
            }
            (
                Type::Object {
                    call_signatures,
                    construct_signatures,
                    ..
                },
                Type::Function { .. },
            ) => {
                // match callable objects against function patterns
                let mut combined = InferSubstitutionsBuilder::new();
                for left_signature in call_signatures
                    .iter()
                    .chain(construct_signatures.iter())
                    .copied()
                {
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        left_signature,
                        right,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
                Some(combined.build())
            }
            (
                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                },
                Type::Object {
                    fields: right_fields,
                    call_signatures: right_calls,
                    construct_signatures: right_constructs,
                    index_signatures: right_indexes,
                },
            ) => {
                let mut combined = InferSubstitutionsBuilder::new();

                // match required fields in the pattern
                for right_field in right_fields {
                    if let Some(left_field) =
                        fields.iter().find(|field| field.key == right_field.key)
                    {
                        let inferred = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            left_field.ty,
                            right_field.ty,
                            source_id,
                            visited,
                        )?;
                        self.merge_infer_substitutions(
                            &mut ctx.reborrow(),
                            &mut combined,
                            inferred,
                            InferMergeMode::Union,
                        );
                    } else if !right_field.is_optional {
                        return None;
                    }
                }

                // match callable signatures in the pattern
                if !right_calls.is_empty() && call_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_calls {
                    let mut matched_signature = false;
                    for left_signature in call_signatures.iter() {
                        if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            *left_signature,
                            right_signature,
                            source_id,
                            visited,
                        ) {
                            self.merge_infer_substitutions(
                                &mut ctx.reborrow(),
                                &mut combined,
                                inferred,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                // match constructor signatures in the pattern
                if !right_constructs.is_empty() && construct_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_constructs {
                    let mut matched_signature = false;
                    for left_signature in construct_signatures.iter() {
                        if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            *left_signature,
                            right_signature,
                            source_id,
                            visited,
                        ) {
                            self.merge_infer_substitutions(
                                &mut ctx.reborrow(),
                                &mut combined,
                                inferred,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                // match index signatures in the pattern
                if !right_indexes.is_empty() && index_signatures.is_empty() {
                    return None;
                }

                for right_signature in right_indexes {
                    let mut matched_signature = false;
                    for left_signature in index_signatures.iter() {
                        let inferred_key = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            left_signature.key_type,
                            right_signature.key_type,
                            source_id,
                            visited,
                        );
                        let inferred_value = self.infer_conditional_type_substitutions_inner(
                            &mut ctx.reborrow(),
                            distributive,
                            left_signature.value_type,
                            right_signature.value_type,
                            source_id,
                            visited,
                        );

                        if let (Some(inferred_key), Some(inferred_value)) =
                            (inferred_key, inferred_value)
                        {
                            self.merge_infer_substitutions(
                                &mut ctx.reborrow(),
                                &mut combined,
                                inferred_key,
                                InferMergeMode::Union,
                            );
                            self.merge_infer_substitutions(
                                &mut ctx.reborrow(),
                                &mut combined,
                                inferred_value,
                                InferMergeMode::Union,
                            );
                            matched_signature = true;
                        }
                    }
                    if !matched_signature {
                        return None;
                    }
                }

                Some(combined.build())
            }
            (
                Type::TypeLiteral {
                    value:
                        TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                        | TypeLiteral::ScalarLiteral(ScalarLiteral::Float(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Number),
                },
            )
            | (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                },
            ) => Some(InferSubstitutions::empty()),
            (
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)),
                },
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                },
            ) => Some(InferSubstitutions::empty()),
            (Type::TypeLiteral { value }, Type::TypeLiteral { value: right_value }) => {
                if value == right_value {
                    return Some(InferSubstitutions::empty());
                }
                None
            }
            (
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                },
                _,
            ) => Some(InferSubstitutions::empty()),
            _ => None,
        }
    }

    /// Infer substitutions for conditional patterns when the left side is `any`.
    fn infer_substitutions_for_any_left(
        &self,
        ctx: &mut TypeContext<'_>,
        distributive: bool,
        left: LocalTypeId,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> Option<InferSubstitutions> {
        // handle union patterns by merging successful branches
        if let Type::Union { elements } = ctx.types.get_type(right).clone() {
            let mut combined = InferSubstitutionsBuilder::new();
            let mut matched = false;

            for element_id in elements {
                let mut branch_visited = HashSet::new();
                if let Some(inferred) = self.infer_conditional_type_substitutions_inner(
                    &mut ctx.reborrow(),
                    distributive,
                    left,
                    element_id,
                    source_id,
                    &mut branch_visited,
                ) {
                    matched = true;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
            }

            if matched {
                return Some(combined.build());
            }
            return None;
        }

        // infer template literal spans as string-compatible for `any`
        if let Type::TemplateLiteral { spans, .. } = ctx.types.get_type(right) {
            let spans = spans.clone();
            let substitutions = self.infer_template_literal_substitutions_for_any(
                &mut ctx.reborrow(),
                &spans,
                source_id,
            );
            return Some(substitutions);
        }

        // collect infer names from the pattern
        let mut names = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_infer_names_from_type(right, ctx.types, &mut visited, &mut names);

        // default to an empty substitution when there are no infer names
        if names.is_empty() {
            return Some(InferSubstitutions::empty());
        }

        // map infer bindings to any for `any` patterns
        let any_type = ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Any,
            },
            source_id,
        );
        let mut substitutions = InferSubstitutionsBuilder::new();
        for name in names {
            substitutions.insert(name, any_type);
        }

        Some(substitutions.build())
    }

    /// Infer substitutions from matching reference static arguments.
    fn infer_conditional_substitutions_for_reference_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        left_arguments: &[StaticArgument],
        right_arguments: &[StaticArgument],
        distributive: bool,
        source_id: LocalNodeIdAny,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // infer substitutions for each aligned argument
        let mut combined = InferSubstitutionsBuilder::new();
        if left_arguments.len() != right_arguments.len() {
            return None;
        }

        for (left_argument, right_argument) in left_arguments.iter().zip(right_arguments.iter()) {
            let left_ty = self.convert_static_argument_type(left_argument, source_id, ctx.types);
            let right_ty = self.convert_static_argument_type(right_argument, source_id, ctx.types);
            let inferred = self.infer_conditional_type_substitutions_inner(
                &mut ctx.reborrow(),
                distributive,
                left_ty,
                right_ty,
                source_id,
                visited,
            )?;
            self.merge_infer_substitutions(
                &mut ctx.reborrow(),
                &mut combined,
                inferred,
                InferMergeMode::Union,
            );
        }

        Some(combined.build())
    }

    /// Collect infer bindings from a pattern type.
    fn collect_infer_names_from_type(
        &self,
        type_id: LocalTypeId,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // stop on recursion cycles
        if !visited.insert(type_id) {
            return;
        }

        // walk infer-bearing type shapes
        match types.get_type(type_id) {
            Type::Infer { name, .. } => {
                names.insert(*name);
            }
            Type::Reference {
                static_arguments, ..
            }
            | Type::Import {
                static_arguments, ..
            } => {
                if let Some(arguments) = static_arguments {
                    for argument in arguments {
                        self.collect_infer_names_from_static_argument(
                            argument, types, visited, names,
                        );
                    }
                }
            }
            Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*right, types, visited, names);
                self.collect_infer_names_from_type(*then_type, types, visited, names);
                self.collect_infer_names_from_type(*else_type, types, visited, names);
            }
            Type::Mapped {
                parameter, value, ..
            } => {
                self.collect_infer_names_from_type(parameter.constraint, types, visited, names);
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_infer_names_from_type(key_remap, types, visited, names);
                }
                self.collect_infer_names_from_type(*value, types, visited, names);
            }
            Type::Index { left, index } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*index, types, visited, names);
            }
            Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.collect_infer_names_from_type(*span, types, visited, names);
                }
            }
            Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.collect_infer_names_from_type(*element, types, visited, names);
                }
            }
            Type::ArraySized { element, .. } => {
                self.collect_infer_names_from_type(*element, types, visited, names);
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    self.collect_infer_names_from_type(element.ty, types, visited, names);
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.collect_infer_names_from_type(field.ty, types, visited, names);
                }
                for signature in call_signatures {
                    self.collect_infer_names_from_type(*signature, types, visited, names);
                }
                for signature in construct_signatures {
                    self.collect_infer_names_from_type(*signature, types, visited, names);
                }
                for signature in index_signatures {
                    self.collect_infer_names_from_type(signature.key_type, types, visited, names);
                    self.collect_infer_names_from_type(signature.value_type, types, visited, names);
                }
            }
            Type::Function {
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
                ..
            } => {
                for parameter in static_parameters {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                if let Some(parameter) = this_parameter {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                for parameter in dynamic_parameters {
                    self.collect_infer_names_from_type(*parameter, types, visited, names);
                }
                if let Some(return_type) = return_type {
                    self.collect_infer_names_from_type(*return_type, types, visited, names);
                }
            }
            Type::Readonly { target_type: right }
            | Type::KeyOf { target_type: right }
            | Type::Must { target_type: right }
            | Type::AsComptime { target_type: right }
            | Type::Not { target_type: right }
            | Type::ValueOf { right, .. }
            | Type::ReferenceOf { right, .. }
            | Type::PointerOf { right, .. } => {
                self.collect_infer_names_from_type(*right, types, visited, names);
            }
            Type::In { left, right }
            | Type::Extends { left, right }
            | Type::Implements { left, right } => {
                self.collect_infer_names_from_type(*left, types, visited, names);
                self.collect_infer_names_from_type(*right, types, visited, names);
            }
            Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.collect_infer_names_from_type(*target, types, visited, names);
                }
            }
            Type::Union { elements } | Type::Intersection { elements } => {
                for element in elements {
                    self.collect_infer_names_from_type(*element, types, visited, names);
                }
            }
            Type::Value { value } => {
                self.collect_infer_names_from_type(*value, types, visited, names);
            }
            Type::TypeLiteral { .. }
            | Type::InferVar { .. }
            | Type::Unevaluated(_)
            | Type::This
            | Type::Error => {}
        }
    }

    /// Collect infer bindings from a static argument.
    fn collect_infer_names_from_static_argument(
        &self,
        argument: &StaticArgument,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // inspect static argument expressions
        match argument {
            StaticArgument::Unevaluated { .. } => {}
            StaticArgument::Evaluated { value, .. } => {
                self.collect_infer_names_from_static_expression(value, types, visited, names);
            }
        }
    }

    /// Collect infer bindings from a static expression.
    fn collect_infer_names_from_static_expression(
        &self,
        expression: &StaticExpression,
        types: &TypeTable,
        visited: &mut HashSet<LocalTypeId>,
        names: &mut HashSet<StringId>,
    ) {
        // walk static expression shapes
        match expression {
            StaticExpression::Type { ty } => {
                self.collect_infer_names_from_type(*ty, types, visited, names);
            }
            StaticExpression::Declaration {
                static_arguments, ..
            } => {
                if let Some(arguments) = static_arguments {
                    for argument in arguments {
                        self.collect_infer_names_from_static_argument(
                            argument, types, visited, names,
                        );
                    }
                }
            }
            StaticExpression::ArrayExpression { elements }
            | StaticExpression::TupleExpression { elements } => {
                for element in elements {
                    self.collect_infer_names_from_static_expression(element, types, visited, names);
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                for property in properties {
                    match property {
                        StaticProperty::Unevaluated { .. } => {}
                        StaticProperty::Field { value, .. } => {
                            self.collect_infer_names_from_static_expression(
                                value, types, visited, names,
                            );
                        }
                        StaticProperty::Method { body, .. } => {
                            self.collect_infer_names_from_static_expression(
                                body, types, visited, names,
                            );
                        }
                        StaticProperty::Spread { value, .. } => {
                            self.collect_infer_names_from_static_expression(
                                value, types, visited, names,
                            );
                        }
                    }
                }
            }
            StaticExpression::ScalarLiteral { .. }
            | StaticExpression::TypeLiteral { .. }
            | StaticExpression::Unevaluated { .. } => {}
        }
    }

    /// Infer substitutions for `never` inputs.
    fn infer_substitutions_for_never_left(
        &self,
        ctx: &mut TypeContext<'_>,
        right: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> Option<InferSubstitutions> {
        // read the target pattern
        let right_type = ctx.types.get_type(right).clone();

        // infer defaults for template literal spans
        if let Type::TemplateLiteral { spans, .. } = right_type {
            let substitutions =
                self.infer_template_literal_substitutions_for_never(&spans, source_id, ctx.types);
            return Some(substitutions);
        }

        // merge inferred defaults across union branches
        if let Type::Union { elements } = right_type {
            let mut combined = InferSubstitutionsBuilder::new();
            for element_id in elements {
                if let Some(inferred) = self.infer_substitutions_for_never_left(
                    &mut ctx.reborrow(),
                    element_id,
                    source_id,
                ) {
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
            }
            return Some(combined.build());
        }

        // merge inferred defaults across intersection branches
        if let Type::Intersection { elements } = right_type {
            let mut combined = InferSubstitutionsBuilder::new();
            for element_id in elements {
                if let Some(inferred) = self.infer_substitutions_for_never_left(
                    &mut ctx.reborrow(),
                    element_id,
                    source_id,
                ) {
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut combined,
                        inferred,
                        InferMergeMode::Intersection,
                    );
                }
            }
            return Some(combined.build());
        }

        // default infer bindings to never
        let mut infer_names = HashSet::new();
        self.collect_infer_names_from_type(right, ctx.types, &mut HashSet::new(), &mut infer_names);
        let mut substitutions = InferSubstitutionsBuilder::new();
        let never_id = ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            },
            source_id,
        );

        for name in infer_names {
            substitutions.insert(name, never_id);
        }

        Some(substitutions.build())
    }

    /// Infer substitutions by matching a template literal type against a string literal.
    pub(crate) fn infer_template_substitutions_from_string(
        &self,
        ctx: &mut TypeContext<'_>,
        value: &str,
        strings: &[StringId],
        spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
    ) -> Option<InferSubstitutions> {
        // match template literal parts to the string
        let span_values = self.match_template_literal_to_string(strings, value)?;

        // require span counts to match
        if span_values.len() != spans.len() {
            return None;
        }

        // collect inferred substitutions
        let mut substitutions = InferSubstitutionsBuilder::new();
        let mut visited = HashSet::new();

        // apply each span constraint
        for (span_ty_id, span_value) in spans.iter().zip(span_values.iter()) {
            // resolve span type
            let span_ty = ctx.types.get_type(*span_ty_id).clone();
            match span_ty {
                Type::Infer { name, constraint } => {
                    // enforce span constraints for constrained inference
                    if let Some(constraint_id) = constraint
                        && !self.template_span_matches_string(
                            &mut ctx.reborrow(),
                            constraint_id,
                            span_value,
                            &mut visited,
                        )
                    {
                        return None;
                    }

                    // infer literal type for the span
                    let inferred_ty = self.template_infer_literal_type(
                        constraint, span_value, source_id, ctx.types,
                    )?;
                    if let Some(existing) = substitutions.get(&name)
                        && !self.inferred_type_ids_equivalent(
                            &mut ctx.reborrow(),
                            existing,
                            inferred_ty,
                        )
                    {
                        return None;
                    }

                    substitutions.insert(name, inferred_ty);
                }
                _ => {
                    // enforce non infer span constraints
                    if !self.template_span_matches_string(
                        &mut ctx.reborrow(),
                        *span_ty_id,
                        span_value,
                        &mut visited,
                    ) {
                        return None;
                    }
                }
            }
        }

        Some(substitutions.build())
    }

    /// Infer substitutions for template literals when the input is any.
    fn infer_template_literal_substitutions_for_any(
        &self,
        ctx: &mut TypeContext<'_>,
        spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
    ) -> InferSubstitutions {
        // default to string spans when unconstrained
        let string_id = ctx.types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            source_id,
        );

        // collect inferred substitutions
        let mut substitutions = InferSubstitutionsBuilder::new();

        // collect inferred bindings from each span
        for span_ty_id in spans {
            let span_ty = ctx.types.get_type(*span_ty_id).clone();
            let Type::Infer { name, constraint } = span_ty else {
                continue;
            };

            let inferred_ty = constraint.unwrap_or(string_id);
            if let Some(existing) = substitutions.get(&name) {
                if !self.inferred_type_ids_equivalent(&mut ctx.reborrow(), existing, inferred_ty) {
                    let union_id =
                        self.union_type_from_list(vec![existing, inferred_ty], existing, ctx.types);
                    substitutions.insert(name, union_id);
                }
            } else {
                substitutions.insert(name, inferred_ty);
            }
        }

        substitutions.build()
    }

    /// Infer substitutions for template literals when the input is never.
    fn infer_template_literal_substitutions_for_never(
        &self,
        spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> InferSubstitutions {
        let never_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Never,
            },
            source_id,
        );

        let mut substitutions = InferSubstitutionsBuilder::new();
        for span_ty_id in spans {
            let span_ty = types.get_type(*span_ty_id).clone();
            let Type::Infer { name, .. } = span_ty else {
                continue;
            };

            substitutions.insert(name, never_id);
        }

        substitutions.build()
    }

    /// Infer substitutions by matching a template literal type against another template literal.
    pub(crate) fn infer_template_substitutions_from_template(
        &self,
        ctx: &mut TypeContext<'_>,
        distributive: bool,
        left_strings: &[StringId],
        left_spans: &[LocalTypeId],
        right_strings: &[StringId],
        right_spans: &[LocalTypeId],
        source_id: LocalNodeIdAny,
        visited: &mut HashSet<(LocalTypeId, LocalTypeId)>,
    ) -> Option<InferSubstitutions> {
        // require aligned literal parts
        if left_strings.len() != right_strings.len() || left_spans.len() != right_spans.len() {
            return None;
        }

        // ensure literal parts match exactly
        for (left_string, right_string) in left_strings.iter().zip(right_strings.iter()) {
            if left_string != right_string {
                return None;
            }
        }

        // collect inferred substitutions
        let mut substitutions = InferSubstitutionsBuilder::new();

        // apply each span mapping
        for (left_span, right_span) in left_spans.iter().zip(right_spans.iter()) {
            // resolve right span type
            let right_ty = ctx.types.get_type(*right_span).clone();
            match right_ty {
                Type::Infer { name, .. } => {
                    // map inferred spans directly
                    if let Some(existing) = substitutions.get(&name)
                        && !self.inferred_type_ids_equivalent(
                            &mut ctx.reborrow(),
                            existing,
                            *left_span,
                        )
                    {
                        return None;
                    }

                    substitutions.insert(name, *left_span);
                }
                _ => {
                    // recursively infer nested substitutions
                    let inferred = self.infer_conditional_type_substitutions_inner(
                        &mut ctx.reborrow(),
                        distributive,
                        *left_span,
                        *right_span,
                        source_id,
                        visited,
                    )?;
                    self.merge_infer_substitutions(
                        &mut ctx.reborrow(),
                        &mut substitutions,
                        inferred,
                        InferMergeMode::Union,
                    );
                }
            }
        }

        Some(substitutions.build())
    }

    /// Merge inferred substitutions by unioning divergent bindings.
    fn merge_infer_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        base: &mut InferSubstitutionsBuilder<'_>,
        other: InferSubstitutions,
        mode: InferMergeMode,
    ) {
        // merge inferred bindings across branches
        for (name, ty) in other.into_entries() {
            let Some(existing) = base.get(&name) else {
                base.insert(name, ty);
                continue;
            };

            if self.inferred_type_ids_equivalent(&mut ctx.reborrow(), existing, ty) {
                continue;
            }

            let merged = match mode {
                InferMergeMode::Union => {
                    self.union_type_from_list(vec![existing, ty], existing, ctx.types)
                }
                InferMergeMode::Intersection => {
                    self.intersection_type_from_list(vec![existing, ty], existing, ctx.types)
                }
            };
            base.insert(name, merged);
        }
    }

    /// Check whether two inferred type ids are merge-compatible.
    fn inferred_type_ids_equivalent(
        &self,
        ctx: &mut TypeContext<'_>,
        left: LocalTypeId,
        right: LocalTypeId,
    ) -> bool {
        // fast path: identical ids are equivalent
        if left == right {
            return true;
        }

        // compare normalized shapes for conditional infer merge
        let left = self.normalize_type(&mut ctx.reborrow(), left, NormalizationMode::Assign);
        let right = self.normalize_type(&mut ctx.reborrow(), right, NormalizationMode::Assign);
        if left == right {
            return true;
        }

        // compare merge-compatibility for repeated infer spans
        let left_assignable = self.is_type_assignable(&mut ctx.reborrow(), left, right);
        let right_assignable = self.is_type_assignable(&mut ctx.reborrow(), right, left);
        left_assignable.is_assignable() && right_assignable.is_assignable()
    }

    /// Apply inferred bindings to a type id.
    pub(crate) fn substitute_infer_types(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        substitutions: &InferSubstitutions,
    ) -> LocalTypeId {
        let mut rewriter =
            InferSubstitutionRewriter::new(self, ctx.module, substitutions, ctx.symbols);
        rewriter.rewrite_type_id(ctx.types, type_id)
    }

    /// Resolve inferred types for a local infer binding symbol.
    fn infer_binding_substitution(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        substitutions: &InferSubstitutions,
        symbols: &SymbolTable,
    ) -> Option<LocalTypeId> {
        // only local symbols can be conditional infer bindings
        if symbol.module_id != module.id {
            return None;
        }

        // only local type symbols without declarations are infer bindings
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if symbol_entry.space != SymbolSpace::Type || symbol_entry.kind != SymbolKind::Local {
            return None;
        }

        // skip declared symbols that shadow infer names
        if symbol_entry.primary_declaration.is_some() {
            return None;
        }

        // resolve the inferred binding by name
        let name = symbol_entry.name()?;
        substitutions.get(&name)
    }
}
