use std::collections::{HashMap, HashSet};

use destack_artifact::ArtifactKey;
use destack_dir::{
    GenericParameterKind, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Member, NodeTree, NodeType,
    NormalizationMode, PrimitiveType, ScalarLiteral, StaticArgument, Symbol, SymbolSpace,
    SymbolTable, SymbolType, Type, TypeElement, TypeField, TypeIndexSignature, TypeLiteral,
    TypeTable, WellKnownSymbol,
};
use destack_workspace::workspace::{Module, ProfileId};

use super::{CanonicalSymbolMode, ModuleSymbolView, RelationMode, TypeContext, TypeRewriteCache};
use crate::timing::tags;
use crate::{AnalyzeError, Compiler, CompilerContext};

const MAX_ALIAS_NORMALIZATION_DEPTH: usize = 128;

/// Return whether one normalized union element safely subsumes another.
fn union_element_subsumes(super_type: &Type, sub_type: &Type) -> bool {
    matches!(
        (super_type, sub_type),
        (
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
            },
        ) | (
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(_)),
            },
        )
    )
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Normalize a reference symbol id to a type space symbol when possible.
    pub(crate) fn normalize_reference_symbol_id(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        self.normalize_reference_symbol_id_for_artifact(
            view,
            symbol,
            destack_artifact::ArtifactKey::dir_declared,
        )
    }

    /// Normalize a reference symbol id to a type space symbol when possible from one exact DIR artifact family.
    pub(crate) fn normalize_reference_symbol_id_for_artifact(
        &self,
        view: ModuleSymbolView<'_>,
        symbol: GlobalSymbolId,
        artifact_key: fn(destack_source::ModuleId, ProfileId) -> ArtifactKey,
    ) -> GlobalSymbolId {
        self.require_remote_artifact_dir(
            view.compiler_context,
            view.module.id,
            symbol.module_id,
            view.profile,
            artifact_key,
        )
        .and_then(|()| {
            let base_dir =
                self.require_artifact_dir_base(view.compiler_context.revision(), symbol.module_id)?;
            let owner_module_handle = if symbol.module_id == view.module.id {
                None
            } else {
                Some(view.compiler_context.module(symbol.module_id))
            };
            let owner_module = owner_module_handle.as_deref().unwrap_or(view.module);
            let symbol_entry = base_dir.symbols.get_symbol(symbol.local_id).clone();

            Ok(self.normalize_reference_symbol_id_with_symbols(
                view,
                owner_module,
                symbol,
                &base_dir.symbols,
                symbol_entry,
            ))
        })
        .unwrap_or(symbol)
    }

    /// Normalize a reference symbol id using a symbol table snapshot.
    fn normalize_reference_symbol_id_with_symbols(
        &self,
        view: ModuleSymbolView<'_>,
        owner_module: &Module,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
        symbol_entry: Symbol,
    ) -> GlobalSymbolId {
        // normalize to the declared symbol type
        let normalized =
            GlobalSymbolId::new(owner_module.id, symbol.local_id.with_type(symbol_entry.ty));

        // preserve static parameter identity across symbol space normalization
        if symbol_entry.is_static_parameter() {
            return normalized;
        }

        if matches!(
            symbol_entry.space,
            SymbolSpace::Type | SymbolSpace::TypeValue
        ) {
            return normalized;
        }

        // map well known values to their type symbols
        for well_known in WellKnownSymbol::all() {
            if self.is_well_known_symbol(view.profile, symbol, well_known)
                && let Some(type_symbol) = self.get_well_known_type_symbol(view.profile, well_known)
            {
                return type_symbol;
            }
        }

        // skip when the symbol has no name key
        let Some(key) = symbol_entry.key else {
            return normalized;
        };

        // prefer type space symbols from the same scope
        let scope = symbols.get_scope_by_id(symbol_entry.scope.0);
        for (candidate_key, candidate_id) in symbols.active_named_symbols(scope) {
            if candidate_key != key {
                continue;
            }
            let candidate_entry = symbols.get_symbol(candidate_id);
            if !matches!(
                candidate_entry.space,
                SymbolSpace::Type | SymbolSpace::TypeValue
            ) {
                continue;
            }

            return GlobalSymbolId::new(
                owner_module.id,
                candidate_id.with_type(candidate_entry.ty),
            );
        }

        // gather global and ambient type symbols by key
        let mut candidates = Vec::new();
        if let Some(group) = self.get_global_symbol_group(
            view.compiler_context.revision(),
            view.module.id,
            view.profile,
            key,
            SymbolSpace::Type,
        ) {
            candidates.extend(group);
        }
        if let Some(group) = self.get_global_symbol_group(
            view.compiler_context.revision(),
            view.module.id,
            view.profile,
            key,
            SymbolSpace::TypeValue,
        ) {
            candidates.extend(group);
        }
        if let Some(ambient) =
            self.get_library_symbol_sources_for_merge(view.profile, key, SymbolSpace::Type)
        {
            candidates.extend(ambient);
        }
        if let Some(ambient) =
            self.get_library_symbol_sources_for_merge(view.profile, key, SymbolSpace::TypeValue)
        {
            candidates.extend(ambient);
        }

        // canonicalize candidates: normalize symbol typing, dedupe, then prefer local-module symbols
        let mut normalized_candidates = candidates
            .into_iter()
            .map(|candidate| {
                let candidate_dir =
                    self.artifact_dir_base(candidate.module_id)
                        .unwrap_or_else(|| {
                            panic!(
                                "missing committed base dir artifact for {:?}",
                                candidate.module_id
                            )
                        });
                let candidate_entry = candidate_dir.symbols.get_symbol(candidate.local_id);
                GlobalSymbolId::new(
                    candidate.module_id,
                    candidate.local_id.with_type(candidate_entry.ty),
                )
            })
            .collect::<Vec<_>>();
        normalized_candidates.sort_unstable();
        normalized_candidates.dedup();
        normalized_candidates
            .sort_unstable_by_key(|candidate| (candidate.module_id != view.module.id, *candidate));
        if let Some(candidate) = normalized_candidates.into_iter().next() {
            return candidate;
        }

        normalized
    }

    /// Normalize a type id for the given mode.
    pub(crate) fn normalize_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
    ) -> LocalTypeId {
        self.normalize_type_with_relation(&mut ctx.reborrow(), type_id, mode, RelationMode::ASSIGN)
    }

    /// Normalize a type id for the given mode and relation.
    pub(crate) fn normalize_type_with_relation(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_NORMALIZE);

        let mut visited = Vec::new();
        self.normalize_type_inner(
            &mut ctx.reborrow(),
            type_id,
            mode,
            relation_mode,
            &mut visited,
        )
    }

    /// Normalize a type for assignability checks.
    pub(crate) fn normalize_type_for_assignability(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
    ) -> LocalTypeId {
        // normalize the apparent return type first
        let normalized = self.normalize_apparent_type(
            &mut ctx.reborrow(),
            type_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );

        // stop when normalization already expanded the type
        if normalized != type_id {
            return normalized;
        }

        // expand alias targets when the return type stays wrapped
        let Type::Reference {
            symbol,
            generic_arguments,
        } = ctx.types.get_type(type_id).clone()
        else {
            return normalized;
        };
        if symbol.ty() != SymbolType::TypeAlias {
            return normalized;
        }
        let Some(arguments) = generic_arguments.as_ref() else {
            return normalized;
        };

        // load the alias target for substitution
        let source_id = ctx.types.get_type_source(type_id);
        let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut ctx.reborrow(), symbol, source_id)
        else {
            return normalized;
        };

        // resolve static arguments for substitution
        let resolved_arguments = self
            .resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                Some(arguments.as_slice()),
                false,
            )
            .ok()
            .flatten();
        let arguments = resolved_arguments.as_deref().unwrap_or(arguments);

        // substitute parameters into the alias target
        let substitutions = self.build_type_parameter_substitutions_for_symbol(
            &mut ctx.reborrow(),
            symbol,
            source_id,
            arguments,
        );
        if substitutions.is_empty() {
            return self.normalize_type(
                &mut ctx.reborrow(),
                alias_target_id,
                NormalizationMode::Assign,
            );
        }

        // apply canonical alias instantiation before normalization
        let mut materialize_cache = TypeRewriteCache::new();
        let mut substitute_cache = HashMap::new();
        let instantiated = self.instantiate_type_with_substitutions(
            &mut ctx.reborrow(),
            source_id,
            None,
            alias_target_id,
            &substitutions,
            &mut materialize_cache,
            &mut substitute_cache,
        );
        self.normalize_type(&mut ctx.reborrow(), instantiated, NormalizationMode::Assign)
    }

    /// Normalize a type id with a recursion guard.
    pub(crate) fn normalize_type_inner(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // normalization caches assign relation mode only
        // reuse cached normalization when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) = ctx.types.get_normalized_type(mode, relation_key, type_id)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                ctx.types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                ctx.types
                    .record_normalization_symbol_dependency(*dependency_id);
            }
            return entry.normalized_type;
        }

        // report alias-driven normalization cycles instead of silently
        // preserving the recursive type
        if visited.contains(&type_id) {
            ctx.types.record_normalization_dependency(type_id);

            if ctx.types.normalization_alias_in_progress_depth() > 0 {
                let source_id = ctx.types.get_type_source(type_id);
                let node = ctx
                    .types
                    .get_type_source(type_id)
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::RecursiveTypeInstantiation { node });

                return ctx.types.insert_type_from_any(Type::Error, source_id);
            }

            return type_id;
        }

        // collect dependencies for this normalization pass
        ctx.types.push_normalization_dependency_scope();
        ctx.types.record_normalization_dependency(type_id);
        visited.push(type_id);

        // keep the source id for any normalized replacement
        let source_id = ctx.types.get_type_source(type_id);
        let ty = ctx.types.get_type(type_id).clone();

        // normalize based on structural shape
        let mut should_cache = relation_mode.is_cacheable();
        let normalized_id = match ty {
            Type::Union { elements } => self.normalize_union_type(
                &mut ctx.reborrow(),
                type_id,
                &elements,
                mode,
                relation_mode,
                visited,
            ),
            Type::Intersection { elements } => self.normalize_intersection_type(
                &mut ctx.reborrow(),
                type_id,
                &elements,
                mode,
                relation_mode,
                visited,
            ),
            Type::Reference {
                symbol,
                generic_arguments,
            } => {
                // normalize reference ctx.symbols to their declared type
                let normalized_symbol =
                    self.normalize_reference_symbol_id(ctx.module_symbol_view(), symbol);
                if normalized_symbol != symbol {
                    let normalized = Type::Reference {
                        symbol: normalized_symbol,
                        generic_arguments,
                    };
                    let normalized_id = ctx.types.insert_type_from_any(normalized, source_id);
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        normalized_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                } else {
                    let symbol = normalized_symbol;

                    // follow import targets while preserving alias identity
                    let symbol = self.canonical_symbol_id(
                        ctx.module_symbol_view(),
                        symbol,
                        CanonicalSymbolMode::PreserveAliases,
                    );

                    // rewrite well known references to canonical shapes
                    if let Some(well_known) = self.well_known_array_kind(ctx.profile, symbol)
                        && let Some(normalized) = self.normalize_well_known_type_reference(
                            &mut ctx.reborrow(),
                            source_id,
                            well_known,
                            generic_arguments.as_deref(),
                        )
                    {
                        let normalized_id = ctx.types.insert_type_from_any(normalized, source_id);
                        self.normalize_type_inner(
                            &mut ctx.reborrow(),
                            normalized_id,
                            mode,
                            relation_mode,
                            visited,
                        )
                    }
                    // expand type aliases with static arguments
                    else if symbol.ty() == SymbolType::TypeAlias {
                        // keep abstract associated aliases symbolic here
                        if self.alias_reference_is_opaque_for_normalization(
                            ctx.compiler_context,
                            ctx.module,
                            ctx.profile,
                            ctx.tree,
                            ctx.symbols,
                            symbol,
                        ) {
                            type_id
                        } else {
                            let arguments = generic_arguments.as_deref().unwrap_or(&[]);
                            let expanded = self.normalize_type_alias_reference_with_arguments(
                                &mut ctx.reborrow(),
                                source_id,
                                symbol,
                                arguments,
                                mode,
                                relation_mode,
                                visited,
                            );
                            if let Some(expanded) = expanded {
                                self.normalize_type_inner(
                                    &mut ctx.reborrow(),
                                    expanded,
                                    mode,
                                    relation_mode,
                                    visited,
                                )
                            } else {
                                let unwrapped =
                                    self.unwrap_normalization_alias_reference(type_id, ctx.types);
                                if unwrapped != type_id {
                                    self.normalize_type_inner(
                                        &mut ctx.reborrow(),
                                        unwrapped,
                                        mode,
                                        relation_mode,
                                        visited,
                                    )
                                } else {
                                    type_id
                                }
                            }
                        }
                    } else {
                        let unwrapped =
                            self.unwrap_normalization_alias_reference(type_id, ctx.types);
                        if unwrapped != type_id {
                            self.normalize_type_inner(
                                &mut ctx.reborrow(),
                                unwrapped,
                                mode,
                                relation_mode,
                                visited,
                            )
                        } else {
                            type_id
                        }
                    }
                }
            }
            Type::Array {
                element,
                is_readonly,
            } => {
                // normalize the optional element type
                let original_element = element;
                let normalized_element = original_element.map(|element_id| {
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        element_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                });

                if normalized_element == original_element {
                    type_id
                } else {
                    let normalized = Type::Array {
                        element: normalized_element,
                        is_readonly,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ArraySized {
                element,
                count,
                is_readonly,
            } => {
                // normalize the array element type
                let original_element = element;
                let normalized_element = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_element,
                    mode,
                    relation_mode,
                    visited,
                );

                // normalize the static count type
                let original_count = count;
                let normalized_count = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_count,
                    mode,
                    relation_mode,
                    visited,
                );

                if normalized_element == original_element && normalized_count == original_count {
                    type_id
                } else {
                    let normalized = Type::ArraySized {
                        element: normalized_element,
                        count: normalized_count,
                        is_readonly,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Tuple {
                elements,
                is_readonly,
            } => {
                // normalize tuple element ctx.types
                let mut normalized_elements = Vec::with_capacity(elements.len());
                let mut did_change = false;
                for element in elements {
                    let (normalized, element_changed) = self.normalize_tuple_element(
                        &mut ctx.reborrow(),
                        element,
                        mode,
                        relation_mode,
                        visited,
                    );
                    if element_changed {
                        did_change = true;
                    }
                    normalized_elements.push(normalized);
                }

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Tuple {
                        elements: normalized_elements,
                        is_readonly,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                // track object member changes
                let mut did_change = false;

                // normalize object fields
                let mut normalized_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let normalized_ty = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        field.ty,
                        mode,
                        relation_mode,
                        visited,
                    );
                    if normalized_ty != field.ty {
                        did_change = true;
                    }
                    normalized_fields.push(TypeField {
                        ty: normalized_ty,
                        ..field
                    });
                }

                // normalize callable signatures
                let normalized_calls = self.normalize_type_list(
                    &mut ctx.reborrow(),
                    &call_signatures,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // normalize construct signatures
                let normalized_constructs = self.normalize_type_list(
                    &mut ctx.reborrow(),
                    &construct_signatures,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );

                // normalize index signatures
                let mut normalized_indexes = Vec::with_capacity(index_signatures.len());
                for signature in index_signatures {
                    let normalized_key = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        signature.key_type,
                        mode,
                        relation_mode,
                        visited,
                    );
                    let normalized_value = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        signature.value_type,
                        mode,
                        relation_mode,
                        visited,
                    );

                    if normalized_key != signature.key_type
                        || normalized_value != signature.value_type
                    {
                        did_change = true;
                    }

                    normalized_indexes.push(TypeIndexSignature {
                        key_type: normalized_key,
                        value_type: normalized_value,
                        ..signature
                    });
                }

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Object {
                        fields: normalized_fields,
                        call_signatures: normalized_calls,
                        construct_signatures: normalized_constructs,
                        index_signatures: normalized_indexes,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                generic_parameters,
                this_parameter,
                parameters,
                return_type,
            } => {
                // track function member changes
                let mut did_change = false;
                // normalize type parameter and parameter lists
                let normalized_static = self.normalize_type_list(
                    &mut ctx.reborrow(),
                    &generic_parameters,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                let normalized_dynamic = self.normalize_type_list(
                    &mut ctx.reborrow(),
                    &parameters,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // normalize the this parameter when present
                let normalized_this = match this_parameter {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            &mut ctx.reborrow(),
                            type_id,
                            mode,
                            relation_mode,
                            visited,
                        );
                        if normalized != type_id {
                            did_change = true;
                        }
                        Some(normalized)
                    }
                    None => None,
                };
                // normalize the return type when present
                let normalized_return = match return_type {
                    Some(type_id) => {
                        let normalized = self.normalize_type_inner(
                            &mut ctx.reborrow(),
                            type_id,
                            mode,
                            relation_mode,
                            visited,
                        );
                        if normalized != type_id {
                            did_change = true;
                        }
                        Some(normalized)
                    }
                    None => None,
                };

                if !did_change {
                    type_id
                } else {
                    let normalized = Type::Function {
                        asynchrony,
                        cardinality,
                        generic_parameters: normalized_static,
                        this_parameter: normalized_this,
                        parameters: normalized_dynamic,
                        return_type: normalized_return,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Conditional {
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
            } => self.normalize_conditional_type(
                &mut ctx.reborrow(),
                type_id,
                source_id,
                distributive_symbol,
                left,
                right,
                then_type,
                else_type,
                mode,
                relation_mode,
                visited,
            ),
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => self.normalize_mapped_type(
                &mut ctx.reborrow(),
                source_id,
                parameter,
                modifiers,
                value,
                mode,
                relation_mode,
                visited,
            ),
            Type::Index { left, index } => self.normalize_index_type(
                &mut ctx.reborrow(),
                source_id,
                left,
                index,
                mode,
                relation_mode,
                visited,
            ),
            Type::TemplateLiteral { strings, spans } => {
                // normalize template literal spans
                let mut did_change = false;
                let normalized_spans = self.normalize_type_list(
                    &mut ctx.reborrow(),
                    &spans,
                    mode,
                    relation_mode,
                    visited,
                    &mut did_change,
                );
                // collapse template literals containing never
                if normalized_spans.iter().any(|span_id| {
                    matches!(
                        ctx.types.get_type(*span_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        }
                    )
                }) {
                    ctx.types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        },
                        source_id,
                    )
                } else if !did_change {
                    type_id
                } else {
                    let normalized = Type::TemplateLiteral {
                        strings,
                        spans: normalized_spans,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Unevaluated(_) => {
                // keep unevaluated types symbolic so later substitution and obligation steps can resolve them
                type_id
            }
            Type::Import {
                target,
                qualifier,
                generic_arguments,
            } => {
                let resolved = self.query_import_type_reference(
                    &mut ctx.reborrow(),
                    source_id,
                    target,
                    qualifier.as_ref(),
                    generic_arguments.as_deref(),
                );
                if let Some(resolved) = resolved {
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        resolved,
                        mode,
                        relation_mode,
                        visited,
                    )
                } else {
                    // keep unresolved import normalization out of the cache:
                    // the symbolic import may become concrete once remote artifacts arrive
                    should_cache = false;
                    type_id
                }
            }
            Type::Infer { name, constraint } => {
                // normalize the inference constraint when present
                let original_constraint = constraint;
                let constraint = original_constraint.map(|type_id| {
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        type_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                });

                if constraint == original_constraint {
                    type_id
                } else {
                    let normalized = Type::Infer { name, constraint };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                // normalize the predicate target when present
                let original_target = target;
                let target = original_target.map(|type_id| {
                    self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        type_id,
                        mode,
                        relation_mode,
                        visited,
                    )
                });
                if target == original_target {
                    type_id
                } else {
                    let normalized = Type::Predicate {
                        asserts,
                        subject,
                        target,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::KeyOf { target_type } => self.normalize_keyof_type(
                &mut ctx.reborrow(),
                source_id,
                Some(type_id),
                target_type,
                mode,
                relation_mode,
                visited,
            ),
            Type::Readonly { target_type } => {
                // materialize readonly modifiers during normalization
                let normalized_right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    target_type,
                    mode,
                    relation_mode,
                    visited,
                );
                let deep_readonly = ctx.options.deep_readonly;
                self.materialize_readonly_type(
                    source_id,
                    normalized_right,
                    ctx.types,
                    deep_readonly,
                )
            }
            Type::Must { target_type } => {
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    target_type,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == target_type {
                    type_id
                } else {
                    let normalized = Type::Must { target_type: right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::AsComptime { target_type } => {
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    target_type,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == target_type {
                    type_id
                } else {
                    let normalized = Type::AsComptime { target_type: right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Not { target_type } => {
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    target_type,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == target_type {
                    type_id
                } else {
                    let normalized = Type::Not { target_type: right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::In { left, right } => {
                // normalize binary operands
                let original_left = left;
                let original_right = right;
                let left = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_left,
                    mode,
                    relation_mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );

                // reduce decidable type operators to boolean literals
                let normalized_id = self.normalize_decidable_type_operator(
                    &mut ctx.reborrow(),
                    source_id,
                    true,
                    left,
                    right,
                    mode,
                    relation_mode,
                );
                if let Some(normalized_id) = normalized_id {
                    normalized_id
                } else if left == original_left && right == original_right {
                    type_id
                } else {
                    let normalized = Type::In { left, right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Extends { left, right } => {
                let original_left = left;
                let original_right = right;
                let left = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_left,
                    mode,
                    relation_mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );

                let normalized_id = self.normalize_decidable_type_operator(
                    &mut ctx.reborrow(),
                    source_id,
                    false,
                    left,
                    right,
                    mode,
                    relation_mode,
                );
                if let Some(normalized_id) = normalized_id {
                    normalized_id
                } else if left == original_left && right == original_right {
                    type_id
                } else {
                    let normalized = Type::Extends { left, right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Implements { left, right } => {
                let original_left = left;
                let original_right = right;
                let left = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_left,
                    mode,
                    relation_mode,
                    visited,
                );
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );

                let normalized_id = self.normalize_decidable_type_operator(
                    &mut ctx.reborrow(),
                    source_id,
                    false,
                    left,
                    right,
                    mode,
                    relation_mode,
                );
                if let Some(normalized_id) = normalized_id {
                    normalized_id
                } else if left == original_left && right == original_right {
                    type_id
                } else {
                    let normalized = Type::Implements { left, right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                // normalize value of target
                let original_right = right;
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::ValueOf {
                        mutability,
                        variance,
                        right,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                // normalize reference of target
                let original_right = right;
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::ReferenceOf {
                        mutability,
                        variance,
                        right,
                    };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::PointerOf { mutability, right } => {
                // normalize pointer target
                let original_right = right;
                let right = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_right,
                    mode,
                    relation_mode,
                    visited,
                );
                if right == original_right {
                    type_id
                } else {
                    let normalized = Type::PointerOf { mutability, right };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::Value { value } => {
                // normalize type value target
                let original_value = value;
                let value = self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    original_value,
                    mode,
                    relation_mode,
                    visited,
                );
                if value == original_value {
                    type_id
                } else {
                    let normalized = Type::Value { value };
                    ctx.types.insert_type_from_any(normalized, source_id)
                }
            }
            Type::TypeLiteral { .. } | Type::InferVar { .. } | Type::This | Type::Error => type_id,
        };

        // release the recursion guard for this type
        visited.pop();
        let dependencies = ctx.types.pop_normalization_dependency_scope();
        // cache the normalized result for reuse
        if should_cache {
            let dependency_versions = ctx.types.collect_dependency_versions(dependencies);
            ctx.types.set_normalized_type(
                mode,
                relation_key,
                type_id,
                normalized_id,
                dependency_versions,
            );
        }
        normalized_id
    }

    /// Normalize type alias references with static arguments.
    pub(crate) fn normalize_type_alias_reference_with_arguments(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> Option<LocalTypeId> {
        // normalize reference symbols to their declared type
        let symbol = self.normalize_reference_symbol_id(ctx.module_symbol_view(), symbol);
        let arguments = self.canonicalize_instance_arguments_for_key(arguments.to_vec());

        // keep abstract associated aliases symbolic here
        if self.alias_reference_is_opaque_for_normalization(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            ctx.tree,
            ctx.symbols,
            symbol,
        ) {
            return None;
        }

        // return cached normalization results when available
        let relation_key = relation_mode.cache_key();
        if relation_mode.is_cacheable()
            && let Some(entry) =
                ctx.types
                    .get_normalized_alias_reference(symbol, mode, relation_key, &arguments)
        {
            for (dependency_id, _) in &entry.dependency_versions.type_versions {
                ctx.types.record_normalization_dependency(*dependency_id);
            }
            for (dependency_id, _) in &entry.dependency_versions.symbol_versions {
                ctx.types
                    .record_normalization_symbol_dependency(*dependency_id);
            }
            return Some(entry.normalized_type);
        }

        ctx.types.push_normalization_dependency_scope();

        let normalized = {
            // resolve the instance type, including alias targets and remote imports
            let instance_type_id =
                self.instance_type_id_for_normalization(&mut ctx.reborrow(), symbol, source_id)?;

            // materialize unevaluated alias targets before normalization
            let materialized_instance = self.materialize_alias_instance_for_normalization(
                &mut ctx.reborrow(),
                symbol,
                instance_type_id,
            );

            // stop when the alias target already failed
            if ctx.types.get_type(materialized_instance).is_error() {
                return Some(materialized_instance);
            }

            // select the static arguments to substitute
            let resolved_arguments = self.resolved_static_arguments_for_normalization(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                &arguments,
            );

            // report recursion only for actual alias expansion
            if ctx
                .types
                .is_normalization_alias_in_progress(symbol, mode, relation_key, &arguments)
            {
                let node = source_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::RecursiveTypeInstantiation { node });
                let error_id = ctx.types.insert_type_from_any(Type::Error, source_id);
                return Some(error_id);
            }

            // stop non-converging alias expansion before it blows the stack
            if ctx.types.normalization_alias_in_progress_depth() >= MAX_ALIAS_NORMALIZATION_DEPTH {
                let node = source_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::RecursiveTypeInstantiation { node });
                let error_id = ctx.types.insert_type_from_any(Type::Error, source_id);
                return Some(error_id);
            }

            ctx.types.mark_normalization_alias_in_progress(
                symbol,
                mode,
                relation_key,
                arguments.clone(),
            );

            if resolved_arguments.is_empty() {
                Some(self.normalize_type_inner(
                    &mut ctx.reborrow(),
                    materialized_instance,
                    mode,
                    relation_mode,
                    visited,
                ))
            } else {
                // build type parameter substitutions
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    &mut ctx.reborrow(),
                    symbol,
                    source_id,
                    &resolved_arguments,
                );
                if substitutions.is_empty() {
                    // normalize the instance type (even when no substitutions are available)
                    Some(self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        materialized_instance,
                        mode,
                        relation_mode,
                        visited,
                    ))
                } else {
                    // instantiate and normalize with the full substitution pipeline
                    let materialized_instance = self
                        .apply_associated_projection_substitutions(
                            &mut ctx.reborrow(),
                            symbol,
                            materialized_instance,
                            &substitutions,
                        )
                        .unwrap_or(materialized_instance);
                    let mut materialize_cache = TypeRewriteCache::new();
                    let mut substitute_cache = HashMap::new();
                    let substituted = self.instantiate_type_with_substitutions(
                        &mut ctx.reborrow(),
                        source_id,
                        None,
                        materialized_instance,
                        &substitutions,
                        &mut materialize_cache,
                        &mut substitute_cache,
                    );
                    let substituted = self.resolve_unevaluated_alias_instantiation(
                        &mut ctx.reborrow(),
                        substituted,
                        &substitutions,
                    );
                    let normalized_id = self.normalize_type_inner(
                        &mut ctx.reborrow(),
                        substituted,
                        mode,
                        relation_mode,
                        visited,
                    );
                    Some(normalized_id)
                }
            }
        };

        let normalized = normalized.map(|normalized_id| {
            let mut visited = HashSet::new();
            if self.type_contains_reference_symbol(normalized_id, symbol, ctx.types, &mut visited) {
                let node = source_id
                    .into_global(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::RecursiveTypeInstantiation { node });
                return ctx.types.insert_type_from_any(Type::Error, source_id);
            }

            normalized_id
        });

        let dependencies = ctx.types.pop_normalization_dependency_scope();
        ctx.types
            .clear_normalization_alias_in_progress(symbol, mode, relation_key, &arguments);
        if let Some(normalized_id) = normalized
            && relation_mode.is_cacheable()
        {
            let dependency_versions = ctx.types.collect_dependency_versions(dependencies);
            ctx.types.set_normalized_alias_reference(
                symbol,
                mode,
                relation_key,
                arguments,
                normalized_id,
                dependency_versions,
            );
        }
        normalized
    }

    /// Resolve an instantiated unevaluated alias target under concrete substitutions.
    fn resolve_unevaluated_alias_instantiation(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> LocalTypeId {
        if substitutions.is_empty() {
            return type_id;
        }

        let Type::Unevaluated(expression_id) = ctx.types.get_type(type_id).clone() else {
            return type_id;
        };
        if !ctx.tree.has_node_id(expression_id.id) {
            return type_id;
        }

        // resolve structured local alias targets before applying substitutions
        if let Err(error) = self.resolve_declared_type(&mut ctx.reborrow(), type_id) {
            self.error(error);
        }
        if !matches!(ctx.types.get_type(type_id), Type::Unevaluated(_)) {
            let mut mapped_type_id = type_id;
            let mut substitution_cache = HashMap::new();
            mapped_type_id = self.substitute_static_parameters(
                mapped_type_id,
                substitutions,
                ctx.types,
                &mut substitution_cache,
            );

            let mut materialize_cache = TypeRewriteCache::new();
            mapped_type_id = self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                mapped_type_id,
                &mut materialize_cache,
            );

            return self.normalize_type_with_relation(
                &mut ctx.reborrow(),
                mapped_type_id,
                NormalizationMode::Assign,
                RelationMode::STATIC_EVAL,
            );
        }

        let mapped_type = match self.resolve_declared_type_expression_value(
            &mut ctx.reborrow(),
            expression_id,
            true,
            true,
            true,
            true,
            true,
        ) {
            Ok(mapped_type) => mapped_type,
            Err(error) => {
                self.error(error);
                return type_id;
            }
        };

        let mut mapped_type_id = ctx
            .types
            .insert_type_from_any(mapped_type, expression_id.into());
        let mut substitution_cache = HashMap::new();
        mapped_type_id = self.substitute_static_parameters(
            mapped_type_id,
            substitutions,
            ctx.types,
            &mut substitution_cache,
        );

        let mut materialize_cache = TypeRewriteCache::new();
        mapped_type_id = self.materialize_static_arguments_in_type(
            &mut ctx.reborrow(),
            mapped_type_id,
            &mut materialize_cache,
        );

        self.normalize_type_with_relation(
            &mut ctx.reborrow(),
            mapped_type_id,
            NormalizationMode::Assign,
            RelationMode::STATIC_EVAL,
        )
    }

    /// Resolve the instance type id used for alias normalization.
    fn instance_type_id_for_normalization(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        // fetch or import the alias target type when available
        if let Some(alias_target_id) =
            self.alias_target_type_id_for_symbol(&mut ctx.reborrow(), symbol, source_id)
        {
            // resolve unevaluated alias targets before assign normalization can collapse them to unknown
            if matches!(ctx.types.get_type(alias_target_id), Type::Unevaluated(_)) {
                if symbol.module_id == ctx.module.id && ctx.types.module_id == ctx.module.id {
                    if let Err(error) =
                        self.resolve_declared_type(&mut ctx.reborrow(), alias_target_id)
                    {
                        self.error(error);
                    }
                } else {
                    let dir = self.require_indexed_dir_declared(
                        &ctx.index,
                        ctx.compiler_context.revision(),
                        symbol.module_id,
                        ctx.profile,
                    );
                    match dir {
                        Ok(dir) => {
                            let module = ctx.compiler_context.module(symbol.module_id);
                            let module = module.as_ref();
                            let options = ctx
                                .compiler_context
                                .analyze_context_options_for_module(module.id);
                            let mut ctx = TypeContext::new(
                                ctx.compiler_context,
                                module,
                                ctx.profile,
                                &options,
                                &dir.tree,
                                &dir.symbols,
                                ctx.types,
                                ctx.index.clone(),
                            );
                            if let Err(error) =
                                self.resolve_declared_type(&mut ctx.reborrow(), alias_target_id)
                            {
                                self.error(error);
                            }
                        }
                        Err(error) => {
                            self.error(AnalyzeError::from(error));
                        }
                    }
                }
            }

            return Some(alias_target_id);
        }

        // keep abstract aliases opaque here
        None
    }

    /// Resolve the static arguments used for alias normalization.
    fn alias_reference_is_opaque_for_normalization(
        &self,
        context: &CompilerContext<'_>,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        symbol: GlobalSymbolId,
    ) -> bool {
        self.with_module_tree_symbol_view_or_local_for_artifact(
            context,
            module,
            profile,
            symbol.module_id,
            tree,
            symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let symbol_entry = view.symbols.get_symbol(symbol.local_id);

                // only associated type members can be abstract alias placeholders
                if symbol_entry.ty != SymbolType::TypeAlias {
                    return false;
                }

                let Some(primary_declaration) = symbol_entry.primary_declaration else {
                    return false;
                };
                if primary_declaration.local_id.ty != NodeType::Member {
                    return false;
                }

                let member_id = primary_declaration.local_id.into_typed::<Member>();
                let Member::AssociatedType { value, .. } = view.tree.get(member_id) else {
                    return false;
                };

                value.is_none()
            },
        )
        .unwrap_or(false)
    }

    /// Resolve the static arguments used for alias normalization.
    fn resolved_static_arguments_for_normalization(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        symbol: GlobalSymbolId,
        arguments: &[StaticArgument],
    ) -> Vec<StaticArgument> {
        // prefer resolved instance arguments when no arguments are present
        if arguments.is_empty()
            && let Some(resolved) = self.query_instance_arguments_for_node(
                source_id.into_global(ctx.module.id),
                Some(symbol),
                ctx.types,
            )
        {
            return resolved;
        }

        // prefer resolved instance arguments when unevaluated arguments are present
        if arguments
            .iter()
            .any(|argument| matches!(argument, StaticArgument::Unevaluated { .. }))
        {
            // resolve unevaluated arguments using the full reference resolver
            if let Ok(Some(resolved)) = self.resolve_type_reference_static_arguments(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                Some(arguments),
                false,
            ) {
                return resolved;
            }

            // fall back to local materialization when resolution is incomplete
            let resolved = self.materialize_static_arguments_for_reference(
                &mut ctx.reborrow(),
                symbol,
                source_id,
                arguments,
            );
            if resolved != arguments {
                return resolved;
            }

            return self
                .query_instance_arguments_for_node(
                    source_id.into_global(ctx.module.id),
                    Some(symbol),
                    ctx.types,
                )
                .unwrap_or_else(|| arguments.to_vec());
        }

        arguments.to_vec()
    }

    /// Materialize alias targets before normalization when needed.
    fn materialize_alias_instance_for_normalization(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        instance_type_id: LocalTypeId,
    ) -> LocalTypeId {
        if symbol.module_id == ctx.module.id {
            // skip materialization when the alias instance is already stable
            if !self.alias_instance_needs_materialization(
                &mut ctx.reborrow(),
                symbol,
                instance_type_id,
            ) {
                return instance_type_id;
            }

            // materialize static arguments using the alias module context
            let mut materialize_cache = HashMap::new();
            self.materialize_static_arguments_in_type(
                &mut ctx.reborrow(),
                instance_type_id,
                &mut materialize_cache,
            )
        } else {
            let dir = self.require_indexed_dir_declared(
                &ctx.index,
                ctx.compiler_context.revision(),
                symbol.module_id,
                ctx.profile,
            );
            let Ok(dir) = dir else {
                return instance_type_id;
            };

            let module = ctx.compiler_context.module(symbol.module_id);
            let module = module.as_ref();
            let options = ctx
                .compiler_context
                .analyze_context_options_for_module(module.id);
            let mut ctx = TypeContext::new(
                ctx.compiler_context,
                module,
                ctx.profile,
                &options,
                &dir.tree,
                &dir.symbols,
                ctx.types,
                ctx.index.clone(),
            );

            // skip materialization when the alias instance is already stable
            if !self.alias_instance_needs_materialization(
                &mut ctx.reborrow(),
                symbol,
                instance_type_id,
            ) {
                return instance_type_id;
            }

            // materialize static arguments using the alias module context
            let mut materialize_cache = HashMap::new();
            self.materialize_static_arguments_in_type(
                &mut ctx,
                instance_type_id,
                &mut materialize_cache,
            )
        }
    }

    /// Return true when an alias instance needs value materialization.
    fn alias_instance_needs_materialization(
        &self,
        ctx: &mut TypeContext<'_>,
        symbol: GlobalSymbolId,
        instance_type_id: LocalTypeId,
    ) -> bool {
        // check for unresolved static-evaluation convergence state
        if self.type_requires_static_evaluation_convergence(ctx.type_view(), instance_type_id) {
            return true;
        }

        // skip value materialization when the alias has no value parameters
        let Some(parameters) = self.collect_static_parameter_symbols(ctx.type_view(), symbol)
        else {
            return false;
        };
        let has_value_parameters = parameters.iter().any(|parameter_symbol| {
            self.generic_parameter_metadata_for_symbol_in_module(
                ctx.tree_symbol_view(),
                *parameter_symbol,
            )
            .0 == GenericParameterKind::Value
        });
        if !has_value_parameters {
            return false;
        }

        self.type_has_unevaluated_value_static_arguments(
            ctx.type_view(),
            instance_type_id,
            &mut HashSet::new(),
        )
    }

    /// Normalize a tuple element, lifting readonly modifiers into flags.
    fn normalize_tuple_element(
        &self,
        ctx: &mut TypeContext<'_>,
        element: TypeElement,
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> (TypeElement, bool) {
        // unwrap readonly/const modifiers into tuple element flags
        let mut element_ty_id = element.ty;
        let mut element_is_readonly = element.is_readonly;
        let mut did_change = false;
        if let Type::Readonly { target_type: right } = ctx.types.get_type(element_ty_id) {
            element_ty_id = *right;
            element_is_readonly = true;
            did_change = true;
        }

        // normalize the tuple element type
        let normalized_ty = self.normalize_type_inner(
            &mut ctx.reborrow(),
            element_ty_id,
            mode,
            relation_mode,
            visited,
        );
        if normalized_ty != element.ty || element_is_readonly != element.is_readonly {
            did_change = true;
        }

        let normalized_element = TypeElement {
            ty: normalized_ty,
            is_readonly: element_is_readonly,
            ..element
        };
        (normalized_element, did_change)
    }

    /// Normalize union types by flattening and collapsing special cases.
    fn normalize_union_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare union element collection
        let mut flattened = Vec::new();

        // normalize and collect union elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                &mut ctx.reborrow(),
                *element_id,
                mode,
                relation_mode,
                visited,
            );
            // flatten nested unions
            match ctx.types.get_type(normalized) {
                Type::Union { elements: union } => {
                    // keep elements unique
                    for element_id in union {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    // keep elements unique
                    if !flattened.contains(&normalized) {
                        flattened.push(normalized);
                    }
                }
            }
        }

        // collapse any or unknown and remove never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        for element_id in flattened {
            match ctx.types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        let filtered =
            self.collapse_exhaustive_simple_literal_union_members(filtered, type_id, ctx.types);

        // drop literal members that are already covered by one simple primitive
        let mut simplified = Vec::new();
        for element_id in filtered.iter().copied() {
            if self.union_element_is_subsumed_by_simple_primitive_member(
                element_id,
                &simplified,
                ctx.types,
            ) {
                continue;
            }

            simplified.retain(|existing| {
                !self.union_element_is_subsumed_by_simple_primitive_member(
                    *existing,
                    &[element_id],
                    ctx.types,
                )
            });
            simplified.push(element_id);
        }

        // honor dominating any or unknown
        if let Some(any_type) = any_type {
            return any_type;
        }
        if let Some(unknown_type) = unknown_type {
            return unknown_type;
        }

        // no matches means never
        if simplified.is_empty() {
            return never_type.unwrap_or_else(|| {
                ctx.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Never,
                    },
                    ctx.types.get_type_source(type_id),
                )
            });
        }

        // collapse safe literal to primitive cases
        let mut collapsed = Vec::with_capacity(filtered.len());
        'candidate: for candidate_id in filtered {
            let candidate_type = ctx.types.get_type(candidate_id).clone();
            let mut index = 0;
            while index < collapsed.len() {
                let existing_id = collapsed[index];
                let existing_type = ctx.types.get_type(existing_id).clone();

                // keep the broader existing element when it already covers the candidate
                if union_element_subsumes(&existing_type, &candidate_type) {
                    continue 'candidate;
                }

                // replace a narrower existing literal with the broader primitive candidate
                if union_element_subsumes(&candidate_type, &existing_type) {
                    collapsed.remove(index);
                    continue;
                }

                index += 1;
            }

            collapsed.push(candidate_id);
        }

        // collapse complete boolean literal unions to the primitive boolean type
        let mut has_true = false;
        let mut has_false = false;
        let mut all_boolean_literals = true;
        for element_id in &collapsed {
            match ctx.types.get_type(*element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true)),
                } => has_true = true,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(false)),
                } => has_false = true,
                _ => {
                    all_boolean_literals = false;
                    break;
                }
            }
        }
        if all_boolean_literals && has_true && has_false {
            return ctx.types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Boolean),
                },
                ctx.types.get_type_source(type_id),
            );
        }

        // short circuit when a single element remains
        if collapsed.len() == 1 {
            return collapsed[0];
        }

        // reuse existing union id when unchanged
        if collapsed == elements {
            return type_id;
        }

        ctx.types.intern_union_type(collapsed, type_id)
    }

    /// Normalize intersection types by flattening and collapsing special cases.
    fn normalize_intersection_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        elements: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
    ) -> LocalTypeId {
        // prepare intersection element collection
        let mut flattened = Vec::new();

        // normalize and collect intersection elements
        for element_id in elements {
            let normalized = self.normalize_type_inner(
                &mut ctx.reborrow(),
                *element_id,
                mode,
                relation_mode,
                visited,
            );
            // flatten nested intersections
            match ctx.types.get_type(normalized) {
                Type::Intersection {
                    elements: intersection,
                } => {
                    // keep elements unique
                    for element_id in intersection {
                        if !flattened.contains(element_id) {
                            flattened.push(*element_id);
                        }
                    }
                }
                _ => {
                    // keep elements unique
                    if !flattened.contains(&normalized) {
                        flattened.push(normalized);
                    }
                }
            }
        }

        // collapse any or unknown and handle never
        let mut any_type = None;
        let mut unknown_type = None;
        let mut never_type = None;
        let mut filtered = Vec::new();
        // split special literals from remaining elements
        for element_id in flattened {
            match ctx.types.get_type(element_id) {
                Type::TypeLiteral {
                    value: TypeLiteral::Any,
                } => any_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                } => unknown_type = Some(element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Never,
                } => never_type = Some(element_id),
                _ => filtered.push(element_id),
            }
        }

        // honor dominating never or any
        if let Some(never_type) = never_type {
            return never_type;
        }
        if let Some(any_type) = any_type {
            return any_type;
        }

        // strip nullish values when intersecting with empty object types
        let source_id = ctx.types.get_type_source(type_id);
        let mut empty_object_id = None;
        let mut remaining = Vec::new();
        for element_id in filtered {
            let is_empty_object = match ctx.types.get_type(element_id) {
                Type::Object {
                    fields,
                    call_signatures,
                    construct_signatures,
                    index_signatures,
                } => {
                    fields.is_empty()
                        && call_signatures.is_empty()
                        && construct_signatures.is_empty()
                        && index_signatures.is_empty()
                }
                _ => false,
            };

            if is_empty_object {
                if empty_object_id.is_none() {
                    empty_object_id = Some(element_id);
                }
            } else {
                remaining.push(element_id);
            }
        }

        let filtered = if let Some(empty_object_id) = empty_object_id {
            if remaining.is_empty() {
                return empty_object_id;
            }

            let mut stripped = Vec::new();
            for element_id in remaining {
                match ctx.types.get_type(element_id) {
                    Type::TypeLiteral {
                        value: TypeLiteral::Null | TypeLiteral::Undefined,
                    } => {
                        return ctx.types.insert_type_from_any(
                            Type::TypeLiteral {
                                value: TypeLiteral::Never,
                            },
                            source_id,
                        );
                    }
                    Type::Union { elements } => {
                        let mut non_nullish = Vec::new();
                        let mut has_nullish = false;
                        for union_id in elements {
                            match ctx.types.get_type(*union_id) {
                                Type::TypeLiteral {
                                    value: TypeLiteral::Null | TypeLiteral::Undefined,
                                } => {
                                    has_nullish = true;
                                }
                                _ => non_nullish.push(*union_id),
                            }
                        }

                        if !has_nullish {
                            stripped.push(element_id);
                            continue;
                        }

                        if non_nullish.is_empty() {
                            return ctx.types.insert_type_from_any(
                                Type::TypeLiteral {
                                    value: TypeLiteral::Never,
                                },
                                source_id,
                            );
                        }

                        if non_nullish.len() == 1 {
                            stripped.push(non_nullish[0]);
                        } else {
                            stripped.push(ctx.types.insert_type_from_any(
                                Type::Union {
                                    elements: non_nullish,
                                },
                                source_id,
                            ));
                        }
                    }
                    _ => stripped.push(element_id),
                }
            }

            stripped
        } else {
            remaining
        };

        // fall back to unknown when the intersection collapses
        if filtered.is_empty() {
            return unknown_type.unwrap_or_else(|| {
                ctx.types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    ctx.types.get_type_source(type_id),
                )
            });
        }

        // short circuit when a single element remains
        if filtered.len() == 1 {
            return filtered[0];
        }

        // reuse existing intersection id when unchanged
        if filtered == elements {
            return type_id;
        }

        ctx.types.intern_intersection_type(filtered, type_id)
    }

    /// Unwrap structural type aliases using cached instance types.
    pub(super) fn unwrap_normalization_alias_reference(
        &self,
        type_id: LocalTypeId,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        let mut current_id = type_id;
        let mut visited = Vec::new();
        while let Type::Reference {
            symbol,
            generic_arguments,
        } = types.get_type(current_id)
        {
            let symbol = *symbol;
            let generic_arguments = generic_arguments.clone();

            // exit when this is not a type alias
            if symbol.ty() != SymbolType::TypeAlias {
                break;
            };
            // avoid unwrapping aliases with explicit static arguments
            if generic_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
            {
                break;
            }
            // exit on alias cycles
            if visited.contains(&symbol) {
                break;
            }
            visited.push(symbol);

            // exit when the alias has no instance type yet
            types.record_normalization_symbol_dependency(symbol);
            let Some(instance_id) = types.get_instance_type_id(symbol) else {
                break;
            };
            current_id = instance_id;
        }

        current_id
    }

    /// Normalize a list of type ids, updating the change flag.
    fn normalize_type_list(
        &self,
        ctx: &mut TypeContext<'_>,
        type_ids: &[LocalTypeId],
        mode: NormalizationMode,
        relation_mode: RelationMode,
        visited: &mut Vec<LocalTypeId>,
        did_change: &mut bool,
    ) -> Vec<LocalTypeId> {
        let mut normalized = Vec::with_capacity(type_ids.len());
        // normalize each element and track changes
        for type_id in type_ids {
            let normalized_id = self.normalize_type_inner(
                &mut ctx.reborrow(),
                *type_id,
                mode,
                relation_mode,
                visited,
            );
            if normalized_id != *type_id {
                *did_change = true;
            }
            normalized.push(normalized_id);
        }
        normalized
    }

    /// Resolve the apparent type for type operations.
    pub(crate) fn apparent_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_TYPE_APPARENT);

        // skip apparent type expansion when the relation mode says so
        if !relation_mode.flags.use_apparent_type {
            return type_id;
        }

        // unwrap cached alias instances first
        let type_id = self.unwrap_normalization_alias_reference(type_id, ctx.types);

        // expand alias references with static arguments before apparent resolution
        let ty = ctx.types.get_type(type_id).clone();
        if let Type::Reference {
            symbol,
            generic_arguments: Some(generic_arguments),
        } = ty
            && matches!(symbol.ty(), SymbolType::TypeAlias)
        {
            let source_id = ctx.types.get_type_source(type_id);
            let mut visited = Vec::new();
            if let Some(expanded_id) = self.normalize_type_alias_reference_with_arguments(
                &mut ctx.reborrow(),
                source_id,
                symbol,
                &generic_arguments,
                NormalizationMode::Assign,
                relation_mode,
                &mut visited,
            ) {
                return expanded_id;
            }
        }

        // prefer apparent instance types for references and type as value wrappers
        if let Some(symbol) = self.unwrap_type_value_symbol(ctx.types, type_id) {
            let source_id = ctx.types.get_type_source(type_id);
            if let Some(apparent_id) =
                self.apparent_instance_type(&mut ctx.reborrow(), source_id, symbol)
            {
                return apparent_id;
            }
        }

        type_id
    }

    /// Resolve the apparent type for assignability checks.
    pub(crate) fn apparent_type_for_assignability(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        // skip apparent type expansion when the relation mode says so
        if !relation_mode.flags.use_apparent_type {
            return type_id;
        }

        // unwrap cached alias instances only
        self.unwrap_normalization_alias_reference(type_id, ctx.types)
    }

    /// Normalize and resolve the apparent type.
    pub(crate) fn normalize_apparent_type(
        &self,
        ctx: &mut TypeContext<'_>,
        type_id: LocalTypeId,
        mode: NormalizationMode,
        relation_mode: RelationMode,
    ) -> LocalTypeId {
        // normalize the input type first
        let type_id = {
            let mut normalize_visited = Vec::new();
            self.normalize_type_inner(
                &mut ctx.reborrow(),
                type_id,
                mode,
                relation_mode,
                &mut normalize_visited,
            )
        };

        // resolve apparent types after normalization
        let type_id =
            self.apparent_type_for_assignability(&mut ctx.reborrow(), type_id, relation_mode);

        // normalize again after apparent type expansion
        let mut normalize_visited = Vec::new();
        self.normalize_type_inner(
            &mut ctx.reborrow(),
            type_id,
            mode,
            relation_mode,
            &mut normalize_visited,
        )
    }
}
